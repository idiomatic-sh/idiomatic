//! The `executable_program` service: the request's rules merged into one logical program, and the planned program assembled from
//! it and the operators' physical plan.

use crate::domain::coverage::types::RuleIndex;
use crate::domain::executable_program::types::executable_program::value::ExecutableProgram;
use crate::domain::executable_program::types::logical_build::value::LogicalBuild;
use crate::domain::executable_program::types::program_inputs::value::ProgramInputs;
use crate::domain::executable_program::types::{
    Consumers, Draft, DraftInputs, DraftedColumns, RootKind, RuleInput, RuleNodes, RuleScopes,
    StepAt,
};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::service::schema_of;
use crate::domain::operation_schema::types::InputShape;
use ryo_operators::domain::physical_plan::types::logical_program::value::LogicalProgram;
use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use ryo_operators::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use ryo_operators::domain::physical_plan::types::{
    LogicalNode, NodeBindings, PlanInputList, PlanNodeId,
};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::{Input, LiteralText};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::{Operation, Plan};
use std::hash::{Hash, Hasher};

pub(crate) mod logical;
pub mod refusals;
pub mod scopes;
use logical::service::{
    after_inputs, answered_steps, derived_roots, held, observation_reaches, read_inputs, stated,
    timed,
};
use refusals::service::{deferred_layouts, inputs_by_node};
use scopes::service::scope_at;

/// The request's rules as one logical program: equal steps over equal inputs, in the same scope and with the same bindings,
/// are one node; each rule's emitted node and the nodes its frontier and named roots keep are recorded; each node's schema
/// is checked from the catalog's columns before any row is read; the root kinds come from each tree select's infallible prefix; and
/// each rule's step after the files marks it not served until the fourth stage. `inputs` holds the nodes'
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
    // A hosted step reads its host relation's columns, which no plan input names.
    let hosted = std::mem::take(&mut inputs.hosted);
    let mut drafts: Vec<Draft<'p>> = Vec::new();
    let mut buckets = hashbrown::HashMap::<u64, Vec<usize>>::new();
    let mut nodes = Vec::with_capacity(rules.len());
    let mut scopes = Vec::with_capacity(rules.len());
    for (rule, input) in rules.iter().enumerate() {
        let plan: &'p Plan = input.plan;
        let mut steps: Vec<Result<PlanNodeId, OperatingError>> =
            Vec::with_capacity(plan.steps.len());
        // Each step's own schema fault, as its operation states it, before a row-by-row operand defers it.
        let mut own_faults: Vec<Option<OperatingError>> = Vec::with_capacity(plan.steps.len());
        let ordinal = RuleOrdinal(u32::try_from(rule).map_err(|_| OperatingError::ShapeMismatch)?);
        let observed: Vec<Vec<CellKind>> = input
            .observations
            .0
            .iter()
            .map(|bound| bound.columns.iter().map(|column| column.kind).collect())
            .collect();
        for step in &plan.steps {
            let read = inputs_of(&step.operation);
            let at = StepAt {
                scope: step.scope,
                rule: ordinal,
                plan,
            };
            let Some(DraftInputs {
                list,
                shapes,
                fault,
                timing: read_timing,
            }) = read_inputs(named(&read), &steps, &drafts, &observed, at)
            else {
                own_faults.push(None);
                steps.push(Err(OperatingError::NotRun));
                continue;
            };
            // A hosted step reads the host relation the request declares at its place; one the request does not host is a
            // relation nobody read.
            let host = if let Operation::Hosted { relation } = &step.operation {
                Some(
                    hosted
                        .get(usize::try_from(relation.0).unwrap_or(usize::MAX))
                        .map(|kinds| InputShape { kinds, tree: None })
                        .ok_or(OperatingError::NotRead),
                )
            } else {
                None
            };
            let read_own = fault.is_none() && host.is_none();
            let own = match (fault, host) {
                (Some(error), _) | (None, Some(Err(error))) => Err(error),
                (None, Some(Ok(shape))) => schema_of(&step.operation, &[shape], input.bindings.0),
                (None, None) => schema_of(&step.operation, &shapes, input.bindings.0),
            };
            own_faults.push(own.as_ref().err().cloned());
            let DraftedColumns {
                schema,
                layout,
                timing,
            } = if read_own {
                timed(
                    &step.operation,
                    &shapes,
                    own,
                    after_inputs(read_timing, &list),
                )
            } else {
                stated(own, after_inputs(read_timing, &list))
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
                    layout,
                    timing,
                });
                inputs.lists.push(list);
                buckets.entry(hash).or_default().push(at);
                at
            });
            steps.push(Ok(PlanNodeId(
                u32::try_from(at).map_err(|_| OperatingError::ShapeMismatch)?,
            )));
        }
        let refused = answered_steps(plan, &mut steps, &own_faults, &drafts);
        let emit = match steps.get(usize::try_from(plan.emit.step.0).unwrap_or(usize::MAX)) {
            Some(Ok(node)) => Ok(*node),
            Some(Err(error)) => Err(error.clone()),
            None => Err(OperatingError::NotRun),
        };
        scopes.push(RuleScopes {
            emit: scope_at(plan, plan.emit.step),
            observations: plan
                .observations
                .iter()
                .map(|observation| match observation.source {
                    Input::Step(id) => scope_at(plan, id),
                    Input::Relation(_) | Input::BoundObservation(_) => observation.scope,
                })
                .collect(),
        });
        nodes.push(RuleNodes {
            steps,
            emit,
            refused,
        });
    }
    inputs.hosted = hosted;
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
    let mut engine = Vec::with_capacity(rules.len());
    let consumers = held(&mut program, rules, &nodes, &inputs.lists, &mut engine)?;
    let roots = derived_roots(&drafts, &inputs.lists);
    let layouts = drafts.iter().map(|draft| draft.layout.clone()).collect();
    let reads = inputs_by_node(&inputs.lists);
    let deferred = deferred_layouts(&drafts);
    let reach = observation_reaches(&drafts, &inputs.lists);
    Ok(LogicalBuild {
        program,
        rules: nodes,
        consumers,
        engine,
        roots,
        layouts,
        reads,
        deferred,
        scopes,
        reach,
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
        consumers,
        engine,
        roots,
        layouts,
        reads,
        deferred,
        scopes,
        reach,
        ..
    } = build;
    ExecutableProgram {
        plan,
        rules,
        consumers,
        engine,
        roots,
        layouts,
        reads,
        deferred,
        scopes,
        reach,
    }
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
        .get(usize::try_from(node.0).unwrap_or(usize::MAX))
        .copied()
        .ok_or(OperatingError::NotRun)
}

/// The engine uses of `rule`, the uses of a node that are not node readers: the node it emits, then each node it holds for a
/// frontier or a named output, each once per use, as the operators count them.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::service::{assemble, engine_uses, logical};
/// use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
/// let program = assemble(logical(&[], &mut ProgramInputs::default()).unwrap(), Default::default());
/// assert!(engine_uses(&program, ryo_execution_domain::domain::coverage::types::RuleIndex(0)).is_err());
/// ```
///
/// # Errors
///
/// `NotRun` for a rule the program does not have.
pub fn engine_uses(
    program: &ExecutableProgram,
    rule: RuleIndex,
) -> Result<&[PlanNodeId], OperatingError> {
    program
        .engine
        .get(rule.0)
        .map(Vec::as_slice)
        .ok_or(OperatingError::NotRun)
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
