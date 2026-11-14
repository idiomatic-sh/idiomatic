//! The `executable_program` service: the request's rules merged into one logical program, and the planned program assembled from
//! it and the operators' physical plan.

use crate::domain::coverage::types::RuleIndex;
use crate::domain::executable_program::types::executable_program::value::ExecutableProgram;
use crate::domain::executable_program::types::logical_build::value::LogicalBuild;
use crate::domain::executable_program::types::program_inputs::value::ProgramInputs;
use crate::domain::executable_program::types::{
    Consumers, Draft, DraftInputs, RootKind, RuleInput, RuleNodes, Serving,
};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::service::schema_of;
use ryo_operators::domain::physical_plan::types::logical_program::value::LogicalProgram;
use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use ryo_operators::domain::physical_plan::types::{
    LogicalNode, NodeBindings, PlanInput, PlanInputList, PlanNodeId, RuleOrdinal,
};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::{Input, LiteralText, StepScope};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::{Operation, Plan};
use std::hash::{Hash, Hasher};

pub(crate) mod logical;
use crate::domain::executable_program::constants::BOUND_STAGE;
use logical::service::{derived_roots, read_inputs};

/// The request's rules as one logical program: equal steps over equal inputs, in the same scope and with the same bindings,
/// are one node; each rule's emitted node and the nodes its frontier and named roots keep are recorded; each node's schema
/// is checked from the catalog's columns before any row is read; the root kinds come from each tree select's infallible prefix; and
/// each rule's step after the files or bound input marks it not served until the fourth stage. `inputs` holds the nodes'
/// input lists for as long as the program borrows them.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::service::{assemble, logical, rule};
/// use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
/// let program = assemble(logical(&[], &mut ProgramInputs::default()).unwrap(), Default::default());
/// assert!(rule(&program, ryo_execution_domain::domain::coverage::types::RuleIndex(0)).is_err());
/// ```
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
        let mut steps: Vec<Result<PlanNodeId, OperatingError>> =
            Vec::with_capacity(plan.steps.len());
        let mut stage = None;
        let observed: Vec<Vec<CellKind>> = input
            .observations
            .0
            .iter()
            .map(|bound| bound.columns.iter().map(|column| column.kind).collect())
            .collect();
        for step in &plan.steps {
            if step.scope == StepScope::CrossFile
                || matches!(
                    step.operation,
                    Operation::Rows { .. } | Operation::Hosted { .. }
                )
            {
                stage = Some(BOUND_STAGE);
            }
            let read = inputs_of(&step.operation);
            let Some(DraftInputs {
                list,
                shapes,
                fault,
                stage: bound,
            }) = read_inputs(named(&read), &steps, &drafts, &observed)
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
        // A rule with an authoring error in any of its steps answers that error, in step order, before any row: its emit may be
        // sound while a step after it is not.
        let authored = steps.iter().find_map(|step| match step {
            Ok(node) => drafts[node.0 as usize].schema.as_ref().err().cloned(),
            Err(error) => Some(error.clone()),
        });
        let emit = match (authored, steps.get(plan.emit.step.0 as usize)) {
            (Some(error), _) => Err(error),
            (None, Some(Ok(node))) => Ok(*node),
            (None, Some(Err(error))) => Err(error.clone()),
            (None, None) => Err(OperatingError::NotRun),
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
    let roots = derived_roots(&drafts, &inputs.lists);
    let schemas = drafts.iter().map(|draft| draft.schema.clone()).collect();
    Ok(LogicalBuild {
        program,
        rules: nodes,
        serving,
        consumers,
        roots,
        schemas,
    })
}

/// The planned program: the logical build's rules, consumers and root kinds with the operators' plan of it; each rule that
/// reads a node of a kind with no lowering at this stage is not served until the latest stage among them.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::service::{assemble, logical, physical};
/// use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
/// let program = assemble(logical(&[], &mut ProgramInputs::default()).unwrap(), Default::default());
/// let _plan = physical(&program);
/// ```
#[must_use]
pub fn assemble(build: LogicalBuild<'_>, plan: PhysicalPlan) -> ExecutableProgram {
    let LogicalBuild {
        rules,
        mut serving,
        consumers,
        roots,
        schemas,
        ..
    } = build;
    for unserved in plan.unserved() {
        for (nodes, served) in rules.iter().zip(serving.iter_mut()) {
            if nodes
                .steps
                .iter()
                .any(|step| step.as_ref() == Ok(&unserved.node))
            {
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
    ExecutableProgram {
        plan,
        rules,
        serving,
        consumers,
        roots,
        schemas,
    }
}

/// Whether `rule` runs at this stage.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::service::{assemble, logical, serving};
/// use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
/// let program = assemble(logical(&[], &mut ProgramInputs::default()).unwrap(), Default::default());
/// assert!(serving(&program, ryo_execution_domain::domain::coverage::types::RuleIndex(0)).is_err());
/// ```
///
/// # Errors
///
/// `NotRun` for a rule the program does not have.
pub fn serving(program: &ExecutableProgram, rule: RuleIndex) -> Result<Serving, OperatingError> {
    program
        .serving
        .get(rule.0)
        .copied()
        .ok_or(OperatingError::NotRun)
}

/// The nodes that start from the tree nodes of the kind the plans write as `kind`, in node order.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::service::{assemble, logical, roots};
/// use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
/// let program = assemble(logical(&[], &mut ProgramInputs::default()).unwrap(), Default::default());
/// assert!(roots(&program, &ryo_values::domain::plan_parts::types::LiteralText("function_definition".into())).is_empty());
/// ```
#[must_use]
pub fn roots<'a>(program: &'a ExecutableProgram, kind: &LiteralText) -> &'a [PlanNodeId] {
    program.roots.get(kind).map_or(&[], Vec::as_slice)
}

/// Each kind text the plans write for a root kind, with the nodes that start from the tree nodes of that kind; in no order.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::service::{assemble, logical, root_kinds};
/// use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
/// let program = assemble(logical(&[], &mut ProgramInputs::default()).unwrap(), Default::default());
/// assert_eq!(root_kinds(&program).count(), 0);
/// ```
pub fn root_kinds(program: &ExecutableProgram) -> impl Iterator<Item = RootKind<'_>> {
    program
        .roots
        .iter()
        .map(|(kind, nodes)| RootKind { kind, nodes })
}

/// The nodes of `rule`.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::service::{assemble, logical, rule};
/// use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
/// let program = assemble(logical(&[], &mut ProgramInputs::default()).unwrap(), Default::default());
/// assert!(rule(&program, ryo_execution_domain::domain::coverage::types::RuleIndex(0)).is_err());
/// ```
///
/// # Errors
///
/// `NotRun` for a rule the program does not have.
pub fn rule(program: &ExecutableProgram, rule: RuleIndex) -> Result<&RuleNodes, OperatingError> {
    program.rules.get(rule.0).ok_or(OperatingError::NotRun)
}

/// How many consumers `node` has.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::service::{assemble, logical, consumers};
/// use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
/// let program = assemble(logical(&[], &mut ProgramInputs::default()).unwrap(), Default::default());
/// assert!(consumers(&program, ryo_operators::domain::physical_plan::types::PlanNodeId(0)).is_err());
/// ```
///
/// # Errors
///
/// `NotRun` for a node the program does not have.
pub fn consumers(
    program: &ExecutableProgram,
    node: PlanNodeId,
) -> Result<Consumers, OperatingError> {
    program
        .consumers
        .get(node.0 as usize)
        .copied()
        .ok_or(OperatingError::NotRun)
}

/// The output columns of `node`.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::service::{assemble, logical, schema};
/// use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
/// use ryo_operators::domain::physical_plan::types::PlanNodeId;
/// use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
/// let mut inputs = ProgramInputs::default();
/// let program = assemble(logical(&[], &mut inputs).unwrap(), PhysicalPlan::default());
/// assert!(schema(&program, PlanNodeId(0)).is_err());
/// ```
///
/// # Errors
///
/// The schema fault of the node or of a node it reads; `NotRun` for a node the program does not have.
pub fn schema(
    program: &ExecutableProgram,
    node: PlanNodeId,
) -> Result<&[CellKind], OperatingError> {
    match program.schemas.get(node.0 as usize) {
        Some(Ok(kinds)) => Ok(kinds),
        Some(Err(error)) => Err(error.clone()),
        None => Err(OperatingError::NotRun),
    }
}

/// The operators' plan of the program.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::service::{assemble, logical, physical};
/// use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
/// let program = assemble(logical(&[], &mut ProgramInputs::default()).unwrap(), Default::default());
/// let _plan = physical(&program);
/// ```
#[must_use]
pub fn physical(program: &ExecutableProgram) -> &PhysicalPlan {
    &program.plan
}
