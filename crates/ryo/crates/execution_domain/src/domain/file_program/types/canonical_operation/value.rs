//! One operation read by its normalized inputs and the node edges it reads.
use crate::domain::coverage::types::RuleIndex;
use crate::domain::file_program::types::{
    BindingOffset, NodeInput, ObservationGraphInputId, ProgramNodeId,
};
use ryo_values::domain::plan_parts::service::{binding_place, step_in};
use ryo_values::domain::plan_parts::types::{Input, StepId};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::Operation;

/// One plan input resolved to the node edge it is.
struct ResolvedInput {
    /// The plan input, with a step read as the first step.
    input: Input,
    /// The node edge the input is.
    edge: NodeInput,
}

/// An operation with its step numbers normalized, and the node edges it reads.
pub(crate) struct CanonicalOperation {
    /// The operation read by its normalized inputs.
    pub operation: Operation,
    /// The node edges, in input order.
    pub inputs: Vec<NodeInput>,
}

fn node_input(
    input: Input,
    previous: &[Option<ProgramNodeId>],
    rule: RuleIndex,
    bound_offsets: &[BindingOffset],
) -> Option<ResolvedInput> {
    match input {
        Input::Relation(relation) => Some(ResolvedInput {
            input,
            edge: NodeInput::Relation(relation),
        }),
        Input::BoundObservation(input) => Some(ResolvedInput {
            input: Input::Step(StepId(0)),
            edge: NodeInput::Observation(ObservationGraphInputId(
                bound_offsets
                    .get(rule.0)?
                    .0
                    .checked_add(binding_place(input).0)?,
            )),
        }),
        Input::Step(step) => Some(ResolvedInput {
            input: Input::Step(StepId(0)),
            edge: NodeInput::Node(*step_in(previous, step)?.as_ref()?),
        }),
    }
}

impl CanonicalOperation {
    /// Preserve typed fields and ordered input edges; normalize only plan-local step
    /// numbers. Operation equality itself ignores diagnostic Comparison.origin.
    pub(crate) fn of(
        operation: &Operation,
        previous: &[Option<ProgramNodeId>],
        rule: RuleIndex,
        bound_offsets: &[BindingOffset],
    ) -> Option<Self> {
        let inputs = named(&inputs_of(operation))
            .iter()
            .map(|input| node_input(*input, previous, rule, bound_offsets))
            .collect::<Option<Vec<_>>>()?;
        let edges = inputs.iter().map(|resolved| resolved.edge).collect();
        let mut normalized = operation.clone();
        match &mut normalized {
            Operation::Select { from, .. }
            | Operation::Project { from, .. }
            | Operation::Extend { from, .. }
            | Operation::GroupCount { from, .. }
            | Operation::GroupDigest { from, .. }
            | Operation::Capture { from, .. }
            | Operation::Rank { from, .. }
            | Operation::Replace { from, .. }
            | Operation::Words { from, .. }
            | Operation::SpanBetween { from, .. }
            | Operation::SourceMatches { from, .. }
            | Operation::RenderSyntax { from, .. }
            | Operation::FitsRendered { from, .. }
            | Operation::SpanBounds { from, .. } => *from = inputs[0].input,
            Operation::Join { left, right, .. }
            | Operation::Union { left, right }
            | Operation::Intersection { left, right }
            | Operation::Difference { left, right }
            | Operation::Antijoin { left, right, .. }
            | Operation::OrderedMatch { left, right, .. } => {
                *left = inputs[0].input;
                *right = inputs[1].input;
            }
            Operation::Axis {
                from,
                edges,
                domain,
                ..
            } => {
                *from = inputs[0].input;
                *edges = inputs[1].input;
                *domain = inputs[2].input;
            }
            Operation::Closure { edges, .. } | Operation::Cycles { edges, .. } => {
                *edges = inputs[0].input
            }
            Operation::OrderedDfs { from, edges, .. } => {
                *from = inputs[0].input;
                *edges = inputs[1].input;
            }
            Operation::Rows { .. } | Operation::Hosted { .. } => {}
        }
        Some(Self {
            operation: normalized,
            inputs: edges,
        })
    }
}
