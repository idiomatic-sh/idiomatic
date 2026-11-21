//! The service of the `digesting` part: one step written as the bytes its digest is taken of, with each input as that
//! input's digest and each parameter as the value it is bound to. Free functions with no IO and no ports.

use crate::domain::plan_parts::types::{Input, Operand, ParameterId, PlanComparison, StepId};
use crate::domain::plans::service::each_input;
use crate::domain::plans::types::{Operation, PlanStep};
use crate::domain::query_bindings::types::{ParameterBinding, RuleBindings};
use crate::domain::query_plans::types::{NodeBytes, NodeDigest, Plan, StepBytes};
use crate::domain::written_rows::types::RowsSource;
use ryo_relations::domain::relations::service::name;
use std::hash::Hash;

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

/// The digest of one step, from the digests of the steps it reads: the step's bytes are written whole, every number at a
/// fixed width, and `digest` is asked once. `digests` holds the digest of each step digested so far, by its place.
pub(crate) fn step_digest(
    plan: &Plan,
    step: &PlanStep,
    bindings: RuleBindings<'_>,
    digests: &[Option<NodeDigest>],
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
            // A step the plan does not hold, or one of a ring of steps that read each other, has no digest: the place is
            // written as it is, which no other plan's step of an equal operation can share by accident with a real input.
            Input::Step(read) => match digests.get(usize::from(read)).copied().flatten() {
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
