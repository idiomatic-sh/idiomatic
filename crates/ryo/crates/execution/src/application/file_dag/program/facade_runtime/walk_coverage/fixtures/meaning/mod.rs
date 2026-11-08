//! What each operation means for a source walk, and the route that meaning implies.
use crate::domain::executable_program::types::{executable_program::value::ExecutableProgram, InputSource};
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plans::service::kind_of;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_operators::types::OperationKind;

/// Where an instruction's first row comes from, as the walk may route it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in super::super) enum Route {
    Shared,
    Representation,
    Dependent,
    Observation,
    Row,
    Derived,
    Request,
    Continuation,
}

/// Whether a per-file relation head is independently eligible for the shared walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum Eligibility {
    Eligible,
    Ineligible,
}

/// Where an instruction stands in its region.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum Place {
    Head(Eligibility),
    Later,
}

/// Whether the region's first input is a named observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum FirstInput {
    Observation,
    Ordinary,
}

/// What an operation means for a source walk, chosen per kind with no wildcard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum Meaning {
    /// Reads its input as selected or projected rows; the route follows the input.
    Selection,
    /// Computes new columns from each row.
    PerRow,
    /// Needs whole inputs: sets, joins, groups, ordering, graphs.
    WholeInput,
    /// Rows the request owns, not a relation.
    RequestOwned,
}

pub(in super::super) fn meaning(kind: OperationKind) -> Meaning {
    match kind {
        OperationKind::Select | OperationKind::Project => Meaning::Selection,
        OperationKind::Extend
        | OperationKind::Capture
        | OperationKind::Rank
        | OperationKind::Replace
        | OperationKind::SpanBetween
        | OperationKind::SourceMatches
        | OperationKind::RenderSyntax
        | OperationKind::FitsRendered
        | OperationKind::SpanBounds => Meaning::PerRow,
        OperationKind::Join
        | OperationKind::Union
        | OperationKind::Intersection
        | OperationKind::Difference
        | OperationKind::Antijoin
        | OperationKind::GroupCount
        | OperationKind::GroupDigest
        | OperationKind::OrderedMatch
        | OperationKind::Words
        | OperationKind::Axis
        | OperationKind::Closure
        | OperationKind::OrderedDfs
        | OperationKind::Cycles => Meaning::WholeInput,
        OperationKind::Rows | OperationKind::Hosted => Meaning::RequestOwned,
    }
}

/// The route of the first instruction of a region.
/// `head` is the independent eligibility of a per-file relation head.
pub(in super::super) fn head_route(
    operation: &Operation,
    first: FirstInput,
    head: Eligibility,
) -> Route {
    match meaning(kind_of(operation)) {
        Meaning::PerRow => Route::Row,
        Meaning::WholeInput => Route::Derived,
        Meaning::RequestOwned => Route::Request,
        Meaning::Selection => {
            let (Operation::Select { from, .. } | Operation::Project { from, .. }) = operation
            else {
                panic!("a selection meaning is Select or Project")
            };
            match from {
                Input::Relation(_) if head == Eligibility::Eligible => Route::Shared,
                Input::Relation(_) => Route::Representation,
                Input::BoundObservation(_) => Route::Observation,
                Input::Step(_) if first == FirstInput::Observation => Route::Observation,
                Input::Step(_) => Route::Dependent,
            }
        }
    }
}

/// The route of one instruction of a compiled program.
pub(in super::super) fn route(
    program: &ExecutableProgram,
    region: &crate::domain::executable_program::types::region::value::Region,
    instruction: &crate::domain::executable_program::types::instruction::value::Instruction,
    place: Place,
) -> Route {
    if instruction.logical().0 >= program.logical_regions().len() {
        Route::Observation
    } else if let Place::Head(head) = place {
        head_route(
            &instruction.operation(),
            if matches!(region.inputs().first(), Some(InputSource::Observation(_))) {
                FirstInput::Observation
            } else {
                FirstInput::Ordinary
            },
            head,
        )
    } else {
        Route::Continuation
    }
}
