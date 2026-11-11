//! The support of the `fusion` cases.
use super::super::super::super::fixtures::operations::corpus::Corpus;
use ryo_execution_domain::domain::executable_program::{
    fusion::types::{Fusibility, Ineligible},
    types::LogicalInstruction,
};
use ryo_execution_domain::domain::shared_walk::types::walk_route::value::{WalkBarrier, WalkRoute};
use ryo_values::domain::plans::service::kind_of;
use ryo_values::domain::query_operators::types::OperationKind;
use std::borrow::Cow;
use std::collections::{BTreeSet, HashSet};

/// The independent oracle: why each kind is never a continuation, chosen with no wildcard.
pub(super) fn excluded(kind: OperationKind) -> Option<Ineligible> {
    match kind {
        OperationKind::Select | OperationKind::Project => None,
        OperationKind::Rows | OperationKind::Hosted => Some(Ineligible::Source),
        OperationKind::Join
        | OperationKind::Union
        | OperationKind::Intersection
        | OperationKind::Difference
        | OperationKind::Antijoin
        | OperationKind::OrderedMatch
        | OperationKind::Axis
        | OperationKind::OrderedDfs => Some(Ineligible::MultiInput),
        OperationKind::GroupCount
        | OperationKind::GroupDigest
        | OperationKind::Words
        | OperationKind::Closure
        | OperationKind::Cycles => Some(Ineligible::Whole),
        OperationKind::Extend
        | OperationKind::Capture
        | OperationKind::Rank
        | OperationKind::Replace
        | OperationKind::SpanBetween
        | OperationKind::SourceMatches
        | OperationKind::RenderSyntax
        | OperationKind::FitsRendered
        | OperationKind::SpanBounds => Some(Ineligible::RowFault),
    }
}

pub(super) fn label(route: WalkRoute) -> &'static str {
    match route {
        WalkRoute::Shared(_) => "shared",
        WalkRoute::Barrier(WalkBarrier::Representation) => "representation",
        WalkRoute::Barrier(WalkBarrier::DependentInput) => "dependent",
        WalkRoute::Barrier(WalkBarrier::Observation) => "observation",
        WalkRoute::Barrier(WalkBarrier::RowSemantics) => "row",
        WalkRoute::Barrier(WalkBarrier::DerivedOrOrdered) => "derived",
        WalkRoute::Barrier(WalkBarrier::RequestOwnedSource) => "request",
        WalkRoute::Barrier(WalkBarrier::Continuation) => "continuation",
    }
}

#[derive(Debug, PartialEq)]
pub(super) enum Refusal {
    Unclassified,
    Mismatch,
    Uncovered,
}

/// What a verified classification covered: the instructions it checked, the operation kinds it
/// saw, and the routes it saw them under, by label in order.
#[derive(Debug, PartialEq)]
pub(super) struct Coverage {
    pub(super) instructions: usize,
    pub(super) kinds: usize,
    pub(super) routes: Cow<'static, [&'static str]>,
}

/// Every instruction of every kind and route of the corpus has a classification that agrees
/// with its kind and with its place in its region.
pub(super) fn verify(
    corpus: &Corpus,
    lookup: &dyn Fn(LogicalInstruction) -> Option<Fusibility>,
) -> Result<Coverage, Refusal> {
    let program = &corpus.program;
    let mut kinds_seen = HashSet::new();
    let mut routes_seen = HashSet::new();
    let mut instructions = 0;
    for region in program.regions() {
        for (at, instruction) in region.instructions().iter().enumerate() {
            let kind = kind_of(&instruction.operation());
            let class = lookup(instruction.logical()).ok_or(Refusal::Unclassified)?;
            if let Some(why) = excluded(kind)
                && class != Fusibility::Excluded(why)
            {
                return Err(Refusal::Mismatch);
            }
            // The compiler consumes the classification: only a Fusable instruction continues.
            if (at > 0) != (class == Fusibility::Fusable) {
                return Err(Refusal::Mismatch);
            }
            kinds_seen.extend([kind]);
            instructions += 1;
            if let Some(Some(route)) = corpus.walk.physical.get(instruction.logical().0) {
                routes_seen.extend([(kind, label(*route))]);
            }
        }
    }
    let all = OperationKind::ALL.iter().copied().collect::<HashSet<_>>();
    if kinds_seen != all {
        return Err(Refusal::Uncovered);
    }
    for needed in [
        "shared",
        "representation",
        "dependent",
        "row",
        "derived",
        "request",
        "continuation",
    ] {
        if !routes_seen.iter().any(|(_, route)| *route == needed) {
            return Err(Refusal::Uncovered);
        }
    }
    // Every kind appears under at least one route of the inventory.
    for kind in OperationKind::ALL {
        if !routes_seen.iter().any(|(seen, _)| seen == kind) {
            return Err(Refusal::Uncovered);
        }
    }
    let routes = routes_seen
        .iter()
        .map(|(_, route)| *route)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    Ok(Coverage {
        instructions,
        kinds: kinds_seen.len(),
        routes: routes.into(),
    })
}
