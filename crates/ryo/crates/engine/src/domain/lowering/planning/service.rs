//! The services of the `planning` part: the order of a branch's atoms. A branch and a pipeline try their cheapest anchors
//! and keep the cheapest completion (`completing`); a pattern starts from its given cells. What a use of a definition
//! costs along the way is the definition's own plan's (`uses`), planned once for the pack however many accesses ask it.

use super::completing::service::{anchors, completed};
use super::types::{GivenTerms, PlannedBranch, PlanningWith, UnitRole, UnitStart};
use super::uses::service::with_pack;
use crate::domain::lowering::costing::constants::{ANCHORS_TRIED, ONE};
use crate::domain::lowering::costing::types::CellState;
use crate::domain::lowering::definitions::types::Drafts;
use crate::domain::lowering::flattening::types::{
    FlatAtom, FlatBranch, FlatTerm, FlatVariable, RowMeaning,
};
use crate::domain::steps::calls::service::call_shape;
use crate::domain::steps::calls::types::CallRows;
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

/// Whether a cell given at a pattern's start is one a meaning that gives several rows for one gives (a source search's
/// occurrence, a capture's slot): that meaning cannot be asked for it, so each ask would run the meaning whole and test
/// what it gave against the cell. Such a pattern has no keyed program: its rows are a table, made once a file.
fn fanned_given(branch: &FlatBranch, given: &HashSet<FlatVariable>) -> bool {
    branch.atoms.iter().any(|atom| match atom {
        FlatAtom::Called {
            meaning: RowMeaning::Call(meaning),
            outputs,
            ..
        } => call_shape(*meaning).rows == CallRows::Several
            && outputs.iter().any(
                |output| matches!(output, FlatTerm::Variable(variable) if given.contains(variable)),
            ),
        FlatAtom::Called { .. }
        | FlatAtom::Rows { .. }
        | FlatAtom::Latest { .. }
        | FlatAtom::Decided { .. }
        | FlatAtom::Walk { .. }
        | FlatAtom::Nearest { .. }
        | FlatAtom::Closure { .. }
        | FlatAtom::Test { .. } => false,
    })
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
        if fanned_given(branch, start.given) {
            return None;
        }
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

/// The order of a branch's atoms for a role, with `given` variables bound at its start: `planned_with`, each use costed
/// once for the pack.
#[must_use]
pub(crate) fn planned(
    drafts: &Drafts,
    branch: &FlatBranch,
    given: &HashSet<FlatVariable>,
    role: UnitRole,
) -> Option<PlannedBranch> {
    with_pack(drafts, |with| {
        planned_with(with, branch, UnitStart { given, rows: ONE }, role)
    })
}
