//! The `query_plans` service: free functions named by verb over a whole compiled query, with no IO and no ports.

use super::types::{NodeBytes, NodeDigest, NodeDigests, Plan, RuleReads, StepBytes};
use crate::domain::plan_parts::types::{Input, Operand, ParameterId, PlanComparison, StepId};
use crate::domain::plans::service::{each_input, inputs_of, named};
use crate::domain::plans::types::{Operation, PlanStep};
use crate::domain::query_bindings::types::{ParameterBinding, RuleBindings};
use crate::domain::written_rows::types::RowsSource;
use ryo_relations::domain::relations::service::name;
use std::hash::Hash;

/// The relations `plan`'s steps read, each once, in the order a step first reads it: the one walk of a rule's read set.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::query_plans::service::reads_of;
/// use ryo_values::domain::query_plans::types::Plan;
/// fn reads(plan: &Plan) -> usize {
///     reads_of(plan).0.len()
/// }
/// ```
#[must_use]
pub fn reads_of(plan: &Plan) -> RuleReads {
    let mut reads = Vec::new();
    for step in &plan.steps {
        for input in named(&inputs_of(&step.operation)) {
            if let Input::Relation(relation) = *input
                && !reads.contains(&relation)
            {
                reads.push(relation);
            }
        }
    }
    RuleReads(reads.into_boxed_slice())
}

/// What stands in a digested step where an input was: a step's place in its plan is its rule's own, so the input's digest is
/// written after the step instead.
const INPUT_PLACE: Input = Input::Step(StepId(0));

/// What stands in a digested step where a parameter was named: a parameter's place among its plan's parameters is its rule's
/// own, so the value it is bound to is written after the step instead.
const PARAMETER_PLACE: ParameterId = ParameterId(0);

/// Gives `visit` each parameter an operand names, in the order written.
fn each_operand_parameter(operand: &mut Operand, visit: &mut dyn FnMut(&mut ParameterId)) {
    match operand {
        Operand::Parameter(parameter) => visit(parameter),
        Operand::Function(applied) => {
            for argument in &mut applied.arguments {
                each_operand_parameter(argument, visit);
            }
        }
        Operand::Column(_) | Operand::Literal(_) => {}
    }
}

/// Gives `visit` each parameter the comparisons name.
fn each_comparison_parameter(
    comparisons: &mut [PlanComparison],
    visit: &mut dyn FnMut(&mut ParameterId),
) {
    for comparison in comparisons {
        each_operand_parameter(&mut comparison.left, visit);
        each_operand_parameter(&mut comparison.right, visit);
    }
}

/// Gives `visit` each parameter an operation names, in the order written: every place an operation holds one.
fn each_parameter(operation: &mut Operation, visit: &mut dyn FnMut(&mut ParameterId)) {
    match operation {
        Operation::Select { comparisons, .. } => each_comparison_parameter(comparisons, visit),
        Operation::Join { filter, .. } => each_comparison_parameter(filter, visit),
        Operation::Axis { domain_filter, .. } => each_comparison_parameter(domain_filter, visit),
        Operation::Extend { values, .. } => {
            for value in values {
                each_operand_parameter(value, visit);
            }
        }
        Operation::Rank { sets, .. } => visit(sets),
        Operation::Rows { source, .. } => match source {
            RowsSource::Parameter(parameter) => visit(parameter),
            RowsSource::Written(_) => {}
        },
        Operation::Project { .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::Antijoin { .. }
        | Operation::GroupCount { .. }
        | Operation::GroupDigest { .. }
        | Operation::Capture { .. }
        | Operation::Replace { .. }
        | Operation::OrderedMatch { .. }
        | Operation::AsOf { .. }
        | Operation::GroupBound { .. }
        | Operation::GroupOrdinal { .. }
        | Operation::Words { .. }
        | Operation::Closure { .. }
        | Operation::Follow { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Worklist { .. }
        | Operation::Cycles { .. }
        | Operation::Hosted { .. }
        | Operation::SpanBetween { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::SpanBounds { .. } => {}
    }
}

/// Writes what a parameter is bound to: its kind, then its values, each after its length.
fn write_binding(writer: &mut StepBytes, binding: &ParameterBinding) {
    match binding {
        ParameterBinding::Number(number) => (0_u8, number).hash(writer),
        ParameterBinding::Flag(flag) => (1_u8, flag).hash(writer),
        ParameterBinding::Text(text) => (2_u8, text).hash(writer),
        ParameterBinding::Numbers(numbers) => (3_u8, numbers).hash(writer),
        ParameterBinding::Texts(texts) => (4_u8, texts).hash(writer),
        ParameterBinding::TextSets(sets) => (5_u8, sets).hash(writer),
        ParameterBinding::Tables(tables) => {
            (6_u8, tables.len()).hash(writer);
            for table in tables {
                table.0.len().hash(writer);
                for cell in &table.0 {
                    (&cell.key, &cell.value).hash(writer);
                }
            }
        }
    }
}

/// The digest of one step, from the digests of the steps before it: the step's bytes are written whole, every number at a fixed
/// width, and `digest` is asked once.
fn step_digest(
    plan: &Plan,
    step: &PlanStep,
    bindings: RuleBindings<'_>,
    earlier: &[NodeDigest],
    digest: &dyn Fn(NodeBytes<'_>) -> NodeDigest,
) -> NodeDigest {
    let mut writer = StepBytes::default();
    let mut written = step.operation.clone();
    let (mut inputs, mut parameters) = (Vec::new(), Vec::new());
    each_input(&mut written, &mut |input| {
        inputs.push(std::mem::replace(input, INPUT_PLACE))
    });
    each_parameter(&mut written, &mut |parameter| {
        parameters.push(std::mem::replace(parameter, PARAMETER_PLACE));
    });
    (&written, step.scope).hash(&mut writer);
    for input in inputs {
        match input {
            Input::Relation(relation) => (0_u8, name(relation).0).hash(&mut writer),
            // A step that is not before this one has no digest: the place is written as it is, which no other plan's step
            // of an equal operation can share by accident with a real input.
            Input::Step(read) => match earlier.get(usize::from(read)) {
                Some(digest) => (1_u8, digest.0).hash(&mut writer),
                None => (2_u8, read.0).hash(&mut writer),
            },
            Input::BoundObservation(binding) => {
                (3_u8, plan.observation_inputs.get(usize::from(binding))).hash(&mut writer);
            }
        }
    }
    for parameter in parameters {
        match bindings
            .0
            .get(usize::try_from(parameter.0).unwrap_or(usize::MAX))
        {
            Some(binding) => {
                4_u8.hash(&mut writer);
                write_binding(&mut writer, binding);
            }
            // A parameter the rule binds nothing to: its declaration is all that is known of it.
            None => (
                5_u8,
                plan.parameters
                    .get(usize::try_from(parameter.0).unwrap_or(usize::MAX)),
            )
                .hash(&mut writer),
        }
    }
    digest(NodeBytes(&writer.0))
}

/// The digest of every step of `plan`, in step order, each step visited once: a step reads only the steps before it, so each
/// is written once and `digest` is asked once a step, whatever the plan's depth. A step's digest covers its operation and
/// constants, where it runs, each input's digest (a relation by its catalog name) and, for each parameter it names, the value
/// `bindings` binds it to: two rules whose plans differ only in a bound value never share a digest, and two equal sub-plans of
/// any two rules share one whatever their places in their plans.
///
/// `digest` is the caller's: this crate hashes nothing. It is given a step's bytes, numbers at fixed widths with the least
/// byte first, so with one `digest` a plan's digests are the same on every machine; the host that owns the hash pins one.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::query_bindings::types::RuleBindings;
/// use ryo_values::domain::query_plans::service::node_digests;
/// use ryo_values::domain::query_plans::types::{NodeBytes, NodeDigest, Plan};
///
/// fn digests(plan: &Plan, digest: &dyn Fn(NodeBytes<'_>) -> NodeDigest) -> usize {
///     node_digests(plan, RuleBindings(&[]), digest).0.len()
/// }
/// ```
#[must_use]
pub fn node_digests(
    plan: &Plan,
    bindings: RuleBindings<'_>,
    digest: &dyn Fn(NodeBytes<'_>) -> NodeDigest,
) -> NodeDigests {
    let mut digests: Vec<NodeDigest> = Vec::with_capacity(plan.steps.len());
    for step in &plan.steps {
        let node = step_digest(plan, step, bindings, &digests, digest);
        digests.push(node);
    }
    NodeDigests(digests)
}
