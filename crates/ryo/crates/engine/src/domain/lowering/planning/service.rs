//! The services of the `planning` part: the order of a branch's atoms. A branch and a pipeline try their cheapest anchors
//! and keep the cheapest completion (`completing`); a pattern starts from its given cells. What a use of a definition
//! costs along the way is the definition's own plan's (`uses`), planned once for the branch however many accesses ask it.

use super::completing::service::{anchors, completed};
use super::types::{GivenTerms, PlannedBranch, PlanningWith, UnitRole, UnitStart};
use crate::domain::lowering::costing::constants::{ANCHORS_TRIED, ONE};
use crate::domain::lowering::costing::types::{CellState, UseCosts};
use crate::domain::lowering::definitions::types::Drafts;
use crate::domain::lowering::flattening::types::{FlatBranch, FlatTerm, FlatVariable};
use std::collections::HashSet;

/// How a term stands for the variables given so far.
#[must_use]
pub(crate) fn state_of(term: FlatTerm, given: &GivenTerms<'_>) -> CellState {
    match term {
        FlatTerm::Variable(variable) if given.variables.contains(&variable) => CellState::Register,
        // A variable an equality holds to a constant names a bucket until something gives it.
        FlatTerm::Variable(variable) if given.pins.contains_key(&variable) => CellState::Constant,
        FlatTerm::Variable(_) => CellState::Open,
        FlatTerm::Text(_) | FlatTerm::Number(_) => CellState::Constant,
        FlatTerm::File => CellState::File,
    }
}

/// The order of a branch's atoms for a role, from a start, with the uses costed so far. A branch and a pipeline try
/// their cheapest anchors and keep the cheapest completion; a pattern starts from its given cells. A completion with
/// fewer product stages goes before a cheaper one with more, and when each from the cheapest anchors holds one the other
/// anchors are tried too: a pipeline takes a product stage only when no order is keyed throughout. None: the role has
/// no program for the branch, because some access of it has no key.
#[must_use]
pub(crate) fn planned_with(
    with: PlanningWith<'_>,
    branch: &FlatBranch,
    start: UnitStart<'_>,
    role: UnitRole,
) -> Option<PlannedBranch> {
    if role == UnitRole::Pattern {
        return completed(with, branch, start, role, None);
    }
    let mut tried = anchors(with, branch, start, role);
    tried.sort_by_key(|anchor| (anchor.rank, anchor.atom));
    if tried.is_empty() {
        return completed(with, branch, start, role, None);
    }
    let best = |tried: &[super::types::NextAccess]| {
        let plans = tried
            .iter()
            .filter_map(|anchor| completed(with, branch, start, role, Some(*anchor)));
        plans.min_by_key(|plan| (plan.products.0, plan.cost))
    };
    let (first, rest) = tried.split_at(tried.len().min(ANCHORS_TRIED));
    match best(first) {
        Some(plan) if plan.products.0 > 0 => {
            let others = [Some(plan), best(rest)].into_iter().flatten();
            others.min_by_key(|plan| (plan.products.0, plan.cost))
        }
        keyed => keyed,
    }
}

/// The order of a branch's atoms for a role, with `given` variables bound at its start: `planned_with`, each use the
/// branch asks costed once for it.
#[must_use]
pub(crate) fn planned(
    drafts: &Drafts,
    branch: &FlatBranch,
    given: &HashSet<FlatVariable>,
    role: UnitRole,
) -> Option<PlannedBranch> {
    let costs = UseCosts::default();
    let with = PlanningWith {
        drafts,
        costs: &costs,
    };
    planned_with(with, branch, UnitStart { given, rows: ONE }, role)
}
