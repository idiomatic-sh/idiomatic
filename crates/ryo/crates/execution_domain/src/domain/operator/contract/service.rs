// ignore: idiomatic.arch/file-length - exhaustive semantic declarations: one match per operation kind, each naming every kind it leaves to its default, which a split would scatter across files
//! Exhaustive semantic declarations; strategy, cache admission and lifetimes are not kinds.
use super::types::*;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::presence::{
    types::InputMask, types::PresenceMask, types::RowFailure,
};
use ryo_values::domain::query_operators::types::OperationKind;

/// The input `operation` builds into an index of keys before it reads a row, if any: the set an intersection, a difference or
/// an anti-join tests each left row against. The operation reads it as a resource, so it is fed none of its rows.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::operator::contract::service::indexed_input;
/// use ryo_values::domain::plan_parts::types::{Input, InputNumber, StepId};
/// use ryo_values::domain::plans::types::Operation;
/// let (left, right) = (Input::Step(StepId(0)), Input::Step(StepId(1)));
/// assert_eq!(indexed_input(&Operation::Difference { left, right }), Some(InputNumber(1)));
/// assert_eq!(indexed_input(&Operation::Union { left, right }), None);
/// ```
#[must_use]
pub fn indexed_input(
    operation: &ryo_values::domain::plans::types::Operation,
) -> Option<ryo_values::domain::plan_parts::types::InputNumber> {
    use super::resources::types::ResourceProgram;
    match super::resources::service::prepare(operation) {
        ResourceProgram::Index { input, .. } => Some(input),
        ResourceProgram::None
        | ResourceProgram::Appended(_)
        | ResourceProgram::Rank(_)
        | ResourceProgram::Join(_)
        | ResourceProgram::Count { .. }
        | ResourceProgram::Digest { .. }
        | ResourceProgram::Closure { .. }
        | ResourceProgram::Cycles { .. }
        | ResourceProgram::Ordered { .. }
        | ResourceProgram::Dfs(_)
        | ResourceProgram::Axis(_) => None,
    }
}

/// One semantic input and dependency contract, independent of output delivery.
pub fn semantics(kind: OperationKind) -> OperationSemantics {
    use OperationKind::*;
    let (demand, _resource, effects, absence) = match kind {
        Select => (
            DemandOrder::Unary,
            ResourceNeeds::Predicates,
            Effects::Pure,
            0,
        ),
        Project => (
            DemandOrder::Unary,
            ResourceNeeds::DistinctRows,
            Effects::Pure,
            0,
        ),
        Extend => (
            DemandOrder::Unary,
            ResourceNeeds::None,
            Effects::DerivedText,
            0,
        ),
        Join => (
            DemandOrder::LeftThenRight,
            ResourceNeeds::JoinKeys,
            Effects::Pure,
            0,
        ),
        Union => (
            DemandOrder::LeftThenRight,
            ResourceNeeds::DistinctRows,
            Effects::Pure,
            0,
        ),
        Intersection => (
            DemandOrder::LeftThenRight,
            ResourceNeeds::KeyMembership,
            Effects::Pure,
            0,
        ),
        Difference | Antijoin => (
            DemandOrder::LeftThenRight,
            ResourceNeeds::KeyMembership,
            Effects::Pure,
            2,
        ),
        GroupCount | GroupBound | GroupOrdinal => (
            DemandOrder::Unary,
            ResourceNeeds::GroupCounts,
            Effects::Pure,
            1,
        ),
        GroupDigest => (
            DemandOrder::Unary,
            ResourceNeeds::GroupDigest,
            Effects::Pure,
            1,
        ),
        Capture | Words | Replace => (
            DemandOrder::Unary,
            ResourceNeeds::LiteralPatterns,
            Effects::DerivedText,
            0,
        ),
        Rank | SpanBetween | SpanBounds => {
            (DemandOrder::Unary, ResourceNeeds::None, Effects::Pure, 0)
        }
        OrderedMatch | AsOf => (
            DemandOrder::LeftThenRight,
            ResourceNeeds::OrderedKeys,
            Effects::Pure,
            3,
        ),
        Axis => (
            DemandOrder::LeftThenRight,
            ResourceNeeds::Adjacency,
            Effects::Pure,
            0,
        ),
        Closure | Follow | Cycles => (
            DemandOrder::Unary,
            ResourceNeeds::Adjacency,
            Effects::Pure,
            0,
        ),
        OrderedDfs => (
            DemandOrder::LeftThenRight,
            ResourceNeeds::OrderedAdjacency,
            Effects::DerivedText,
            3,
        ),
        // An edge, a lattice row or a root more can change the queue, and so which rows the trace holds.
        Worklist => (
            DemandOrder::LeftThenRight,
            ResourceNeeds::OrderedAdjacency,
            Effects::Pure,
            7,
        ),
        Rows | Hosted => (
            DemandOrder::None,
            ResourceNeeds::None,
            Effects::RequestSource,
            0,
        ),
        SourceMatches => (
            DemandOrder::Unary,
            ResourceNeeds::LiteralPatterns,
            Effects::PhysicalSourceText,
            0,
        ),
        RenderSyntax => (
            DemandOrder::Unary,
            ResourceNeeds::None,
            Effects::DerivedText,
            0,
        ),
        FitsRendered => (
            DemandOrder::Unary,
            ResourceNeeds::None,
            Effects::PhysicalSourceText,
            0,
        ),
    };
    let shape = match kind {
        Rows | Hosted => InputMode::RequestSource,
        GroupCount | GroupBound | GroupOrdinal | GroupDigest | OrderedMatch | AsOf | Closure
        | Follow | Cycles | OrderedDfs | Worklist | RenderSyntax => InputMode::Views,
        Replace | Select | Project | Extend | Join | Union | Intersection | Difference
        | Antijoin | Capture | Rank | Words | Axis | SpanBetween | SourceMatches | FitsRendered
        | SpanBounds => InputMode::Row {
            primary: ryo_values::domain::plan_parts::types::InputNumber(0),
        },
    };
    let row_inputs = RowInputMask(match shape {
        InputMode::Row { .. } => {
            if kind == Union {
                3
            } else {
                1
            }
        }
        InputMode::Views | InputMode::RequestSource => 0,
    });
    let acquisition = if matches!(
        kind,
        Join | Rank | Capture | Words | Replace | SpanBetween | SpanBounds | SourceMatches
    ) {
        AcquisitionBoundary::FirstRow
    } else {
        AcquisitionBoundary::BeforeRows
    };
    let preflight = if matches!(
        kind,
        Select | Project | Join | Union | Intersection | Difference
    ) {
        PreflightOrder::SchemaFirst
    } else {
        PreflightOrder::ResourcesFirst
    };
    OperationSemantics {
        contract: SemanticContract {
            shape,
            demand,
            acquisition,
            row_inputs,
            preflight,
        },
        effects,
        absence: AbsenceMask(absence),
        presence: presence(kind),
    }
}

/// The inputs that must have a row for the operation to answer one: each is implied by the kernel, which emits nothing
/// from an empty required input. An operation that could answer a row from empty inputs declares `Always`.
fn presence(kind: OperationKind) -> PresenceMask {
    use OperationKind::*;
    match kind {
        Select | Project | Extend | Capture | Words | Replace | Rank | SpanBetween | SpanBounds
        | SourceMatches | RenderSyntax | FitsRendered | GroupCount | GroupBound | GroupOrdinal
        | GroupDigest => PresenceMask::AllOf(InputMask(1)),
        Join | Intersection | OrderedMatch | AsOf => PresenceMask::AllOf(InputMask(3)),
        Union => PresenceMask::AnyOf(InputMask(3)),
        // The right input only removes rows.
        Antijoin | Difference => PresenceMask::AllOf(InputMask(1)),
        // The walk starts from the rows of the first input.
        Axis | OrderedDfs | Worklist => PresenceMask::AllOf(InputMask(1)),
        // No edge, no pair.
        Closure | Follow | Cycles => PresenceMask::AllOf(InputMask(1)),
        Rows | Hosted => PresenceMask::Always,
    }
}




