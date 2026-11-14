//! The `fused_program` service: the request's rules merged into one logical program, and the planned program assembled from
//! it and the operators' physical plan.

use crate::domain::coverage::types::RuleIndex;
use crate::domain::fused_program::types::fused_program::value::FusedProgram;
use crate::domain::fused_program::types::logical_build::value::LogicalBuild;
use crate::domain::fused_program::types::program_inputs::value::ProgramInputs;
use crate::domain::fused_program::types::{
    Consumers, Draft, RuleInput, RuleNodes, Serving, DraftInputs,
};
use crate::domain::kind_sets::service::names_a_kind;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::KindNaming;
use crate::domain::operation_schema::service::schema_of;
use crate::domain::operation_schema::types::InputShape;
use crate::domain::table_cells::service::native_form;
use ryo_operators::domain::comparing::service::fixed_prefix;
use ryo_operators::domain::physical_plan::types::logical_program::value::LogicalProgram;
use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use ryo_operators::domain::physical_plan::types::{
    LogicalNode, NodeBindings, PlanInput, PlanInputList, PlanNodeId, RuleOrdinal, ServedAt,
};
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::{
    Input, LiteralText, Operand, QueryLiteral, StepScope, Test,
};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::{Operation, Plan};
use std::hash::{Hash, Hasher};

/// The stage that serves a step after the files, a bound input and the rows and hosted kinds.
const BOUND_STAGE: ServedAt = ServedAt(4);

/// The request's rules as one logical program: equal steps over equal inputs, in the same scope and with the same bindings,
/// are one node; each rule's emitted node and the nodes its frontier and named roots keep are recorded; each node's schema
/// is checked from the catalog's columns before any row is read; the root kinds come from each tree select's infallible prefix; and
/// each rule's step after the files or bound input marks it not served until the fourth stage. `inputs` holds the nodes'
/// input lists for as long as the program borrows them.
///
/// # Errors
///
/// `ShapeMismatch` when the request has more rules or nodes than a plan can number.
pub fn logical<'p>(
    rules: &'p [RuleInput<'p>],
    inputs: &'p mut ProgramInputs,
) -> Result<LogicalBuild<'p>, OperatingError> {
    inputs.lists.clear();
    let mut drafts: Vec<Draft<'p>> = Vec::new();
    let mut buckets = hashbrown::HashMap::<u64, Vec<usize>>::new();
    let mut nodes = Vec::with_capacity(rules.len());
    let mut serving = Vec::with_capacity(rules.len());
    for (rule, input) in rules.iter().enumerate() {
        let plan: &'p Plan = input.plan;
        let mut steps: Vec<Result<PlanNodeId, OperatingError>> = Vec::with_capacity(plan.steps.len());
        let mut stage = None;
        let observed: Vec<Vec<CellKind>> = input
            .observations
            .0
            .iter()
            .map(|bound| bound.columns.iter().map(|column| column.kind).collect())
            .collect();
        for step in &plan.steps {
            if step.scope == StepScope::CrossFile
                || matches!(step.operation, Operation::Rows { .. } | Operation::Hosted { .. })
            {
                stage = Some(BOUND_STAGE);
            }
            let read = inputs_of(&step.operation);
            let Some(DraftInputs { list, shapes, fault, stage: bound }) =
                read_inputs(named(&read), &steps, &drafts, &observed)
            else {
                steps.push(Err(OperatingError::NotRun));
                continue;
            };
            stage = stage.or(bound);
            let schema = match fault {
                Some(error) => Err(error),
                None => schema_of(&step.operation, &shapes, input.bindings.0),
            };
            let hash = {
                let mut hasher = rustc_hash::FxHasher::default();
                step.operation.hash(&mut hasher);
                list.hash(&mut hasher);
                step.scope.hash(&mut hasher);
                hasher.finish()
            };
            let found = buckets.get(&hash).and_then(|bucket| {
                bucket.iter().copied().find(|at| {
                    let draft = &drafts[*at];
                    draft.operation == &step.operation
                        && draft.scope == step.scope
                        && inputs.lists[*at] == list
                        && rules[draft.rule.0].bindings.0 == input.bindings.0
                })
            });
            let at = found.unwrap_or_else(|| {
                let at = drafts.len();
                drafts.push(Draft {
                    operation: &step.operation,
                    rule: RuleIndex(rule),
                    scope: step.scope,
                    schema,
                });
                inputs.lists.push(list);
                buckets.entry(hash).or_default().push(at);
                at
            });
            steps.push(Ok(PlanNodeId(
                u32::try_from(at).map_err(|_| OperatingError::ShapeMismatch)?,
            )));
        }
        let emit = match steps.get(plan.emit.step.0 as usize) {
            Some(Ok(node)) => match &drafts[node.0 as usize].schema {
                Ok(_) => Ok(*node),
                Err(error) => Err(error.clone()),
            },
            Some(Err(error)) => Err(error.clone()),
            None => Err(OperatingError::NotRun),
        };
        serving.push(stage.map_or(Serving::Served, Serving::NotServedUntil));
        nodes.push(RuleNodes { steps, emit });
    }
    let inputs: &'p ProgramInputs = inputs;
    let mut program = LogicalProgram::new();
    for (draft, list) in drafts.iter().zip(&inputs.lists) {
        let _node: PlanNodeId = program.node(LogicalNode {
            operation: draft.operation,
            inputs: PlanInputList(list),
            bindings: NodeBindings(rules[draft.rule.0].bindings.0),
            scope: draft.scope,
        });
    }
    let mut consumers = vec![Consumers(0); drafts.len()];
    let count = |consumers: &mut Vec<Consumers>, node: PlanNodeId| {
        if let Some(each) = consumers.get_mut(node.0 as usize) {
            each.0 = each.0.saturating_add(1);
        }
    };
    for list in &inputs.lists {
        for input in list {
            if let PlanInput::Node(read) = input {
                count(&mut consumers, *read);
            }
        }
    }
    for (rule, (input, rule_nodes)) in rules.iter().zip(&nodes).enumerate() {
        let ordinal = RuleOrdinal(u32::try_from(rule).map_err(|_| OperatingError::ShapeMismatch)?);
        if let Some(Ok(node)) = rule_nodes.steps.get(input.plan.emit.step.0 as usize) {
            program.emit(ordinal, *node);
            count(&mut consumers, *node);
        }
        let kept = input.plan.frontier.iter().chain(
            input
                .plan
                .observations
                .iter()
                .map(|observation| &observation.source),
        );
        for each in kept {
            if let Input::Step(id) = each
                && let Some(Ok(node)) = rule_nodes.steps.get(id.0 as usize)
            {
                program.hold(*node);
                count(&mut consumers, *node);
            }
        }
    }
    let roots = root_kinds(&drafts, &inputs.lists);
    Ok(LogicalBuild {
        program,
        rules: nodes,
        serving,
        consumers,
        roots,
    })
}

/// The inputs `named` reads, given the nodes of the rule's earlier `steps`; none when one reads a step that has no node.
fn read_inputs<'s>(
    named: &[Input],
    steps: &[Result<PlanNodeId, OperatingError>],
    drafts: &'s [Draft<'_>],
    observed: &'s [Vec<CellKind>],
) -> Option<DraftInputs<'s>> {
    let mut read = DraftInputs {
        list: Vec::with_capacity(named.len()),
        shapes: Vec::with_capacity(named.len()),
        fault: None,
        stage: None,
    };
    for each in named {
        let (input, shape) = match each {
            Input::Relation(relation) => (
                PlanInput::Relation(*relation),
                InputShape {
                    kinds: kinds(*relation),
                    tree: native_form(*relation),
                },
            ),
            Input::Step(id) => {
                let Some(Ok(node)) = steps.get(id.0 as usize).cloned() else {
                    return None;
                };
                let draft = drafts.get(node.0 as usize)?;
                if let (Err(error), None) = (&draft.schema, &read.fault) {
                    read.fault = Some(error.clone());
                }
                (
                    PlanInput::Node(node),
                    InputShape {
                        kinds: draft.schema.as_deref().unwrap_or_default(),
                        tree: None,
                    },
                )
            }
            Input::BoundObservation(id) => {
                read.stage = Some(BOUND_STAGE);
                (
                    PlanInput::Observation(*id),
                    InputShape {
                        kinds: observed.get(id.0 as usize).map_or(&[], Vec::as_slice),
                        tree: None,
                    },
                )
            }
        };
        read.list.push(input);
        read.shapes.push(shape);
    }
    Some(read)
}

/// The nodes that start from the tree nodes of a kind: each select of a tree relation whose infallible prefix tests the kind
/// column equal to a written text, by that text. A kind test after a comparison that may fail is never a root kind, so its
/// fault is never skipped.
fn root_kinds(
    drafts: &[Draft<'_>],
    lists: &[Vec<PlanInput>],
) -> hashbrown::HashMap<LiteralText, Vec<PlanNodeId>> {
    let mut roots = hashbrown::HashMap::<LiteralText, Vec<PlanNodeId>>::new();
    for (at, (draft, list)) in drafts.iter().zip(lists).enumerate() {
        let (Operation::Select { comparisons, .. }, [PlanInput::Relation(relation)]) =
            (draft.operation, list.as_slice())
        else {
            continue;
        };
        if native_form(*relation).is_none() || draft.schema.is_err() {
            continue;
        }
        let kinds: &[CellKind] = kinds(*relation);
        for comparison in fixed_prefix(comparisons, kinds).0 {
            if comparison.test == Test::Equal
                && names_a_kind(comparison) == KindNaming::Names
                && let Operand::Literal(QueryLiteral::Text(text)) = &comparison.right
                && let Ok(node) = u32::try_from(at)
            {
                roots.entry(text.clone()).or_default().push(PlanNodeId(node));
            }
        }
    }
    roots
}

/// The planned program: the logical build's rules, consumers and root kinds with the operators' plan of it; each rule that
/// reads a node of a kind with no lowering at this stage is not served until the latest stage among them.
#[must_use]
pub fn assemble(build: LogicalBuild<'_>, plan: PhysicalPlan) -> FusedProgram {
    let LogicalBuild {
        rules,
        mut serving,
        consumers,
        roots,
        ..
    } = build;
    for unserved in plan.unserved() {
        for (nodes, served) in rules.iter().zip(serving.iter_mut()) {
            if nodes.steps.iter().any(|step| step.as_ref() == Ok(&unserved.node)) {
                *served = match *served {
                    Serving::NotServedUntil(stage) if stage >= unserved.until => {
                        Serving::NotServedUntil(stage)
                    }
                    Serving::Served | Serving::NotServedUntil(_) => {
                        Serving::NotServedUntil(unserved.until)
                    }
                };
            }
        }
    }
    FusedProgram {
        plan,
        rules,
        serving,
        consumers,
        roots,
    }
}

/// Whether `rule` runs at this stage.
///
/// # Errors
///
/// `NotRun` for a rule the program does not have.
pub fn serving(program: &FusedProgram, rule: RuleIndex) -> Result<Serving, OperatingError> {
    program
        .serving
        .get(rule.0)
        .copied()
        .ok_or(OperatingError::NotRun)
}

/// The nodes that start from the tree nodes of the kind the plans write as `kind`, in node order.
#[must_use]
pub fn roots<'a>(program: &'a FusedProgram, kind: &LiteralText) -> &'a [PlanNodeId] {
    program.roots.get(kind).map_or(&[], Vec::as_slice)
}

/// The nodes of `rule`.
///
/// # Errors
///
/// `NotRun` for a rule the program does not have.
pub fn rule(program: &FusedProgram, rule: RuleIndex) -> Result<&RuleNodes, OperatingError> {
    program.rules.get(rule.0).ok_or(OperatingError::NotRun)
}

/// How many consumers `node` has.
///
/// # Errors
///
/// `NotRun` for a node the program does not have.
pub fn consumers(program: &FusedProgram, node: PlanNodeId) -> Result<Consumers, OperatingError> {
    program
        .consumers
        .get(node.0 as usize)
        .copied()
        .ok_or(OperatingError::NotRun)
}

/// The operators' plan of the program.
#[must_use]
pub fn physical(program: &FusedProgram) -> &PhysicalPlan {
    &program.plan
}
