//! The services of the `planning` part: the order of a branch's atoms. A test runs as soon as its terms are given; a keyed
//! access that gives one row runs before a decision of a definition (a dear test waits for a keyed lookup); the cheapest
//! keyed access goes next. An access with no key is the unit's first enumeration, or a product stage of a pipeline: a branch
//! and a pattern that would need one anywhere else have no program, and the caller takes the table form.

use super::types::{
    AtomPlace, AtomPlacing, Enumerated, GivenTerms, NextAccess, OrderCompletion, PlannedBranch,
    TermGiven, UnitRole, WaitingAtom,
};
use crate::domain::lowering::costing::constants::{ANCHORS_TRIED, DESCENDANTS, ESTIMATE_CAP};
use crate::domain::lowering::costing::service::keyed;
use crate::domain::lowering::costing::types::{CellState, KeyChoice};
use crate::domain::lowering::definitions::types::{Drafts, RowEstimate};
use crate::domain::lowering::flattening::service::{pins_of, variables};
use crate::domain::lowering::flattening::types::{
    AtomRows, FlatAtom, FlatBranch, FlatTerm, FlatVariable, RowsPerFile,
};
use crate::domain::lowering::planning::walks::service::walk_rows;
use crate::domain::steps::types::CostClass;
use ryo_relations::domain::relations::types::Count;
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

/// Whether a term is given: a constant, the file, or a variable some access gave.
fn ready(term: FlatTerm, given: &GivenTerms<'_>) -> TermGiven {
    match term {
        FlatTerm::Variable(variable) if !given.variables.contains(&variable) => TermGiven::Open,
        FlatTerm::Variable(_) | FlatTerm::Text(_) | FlatTerm::Number(_) | FlatTerm::File => {
            TermGiven::Given
        }
    }
}

/// Whether every term is given.
fn all_ready<'t>(
    terms: impl IntoIterator<Item = &'t FlatTerm>,
    given: &GivenTerms<'_>,
) -> TermGiven {
    let mut terms = terms.into_iter();
    if terms.all(|term| ready(*term, given) == TermGiven::Given) {
        TermGiven::Given
    } else {
        TermGiven::Open
    }
}

/// The access of a rows atom now: a source of one row a file is keyed by the file once the unit has enumerated, and is the
/// unit's own first enumeration before.
fn access(
    drafts: &Drafts,
    atom: &FlatAtom,
    given: &GivenTerms<'_>,
    enumerated: Enumerated,
) -> Option<KeyChoice> {
    let FlatAtom::Rows {
        rows,
        cells,
        per_file,
    } = atom
    else {
        return None;
    };
    let states: Vec<CellState> = cells.iter().map(|cell| state_of(*cell, given)).collect();
    let mut choice = keyed(drafts, *rows, *per_file, &states);
    if *per_file == RowsPerFile::One && enumerated == Enumerated::NotYet {
        choice.class = CostClass::File;
    }
    Some(choice)
}

/// What is given at a completion's point.
fn given_now<'a>(state: &'a OrderCompletion<'_>) -> GivenTerms<'a> {
    GivenTerms {
        variables: &state.given,
        pins: &state.pins,
    }
}

/// Places an atom: its variables are given from here on, and its rows are visited.
fn place(state: &mut OrderCompletion<'_>, next: NextAccess) {
    let Some(atom) = state.branch.atoms.get(next.atom.0) else {
        return;
    };
    if let Some(placed) = state.placed.get_mut(next.atom.0) {
        *placed = AtomPlacing::Placed;
    }
    state.order.push(next.atom);
    match atom {
        FlatAtom::Rows { .. }
        | FlatAtom::Latest { .. }
        | FlatAtom::Walk { .. }
        | FlatAtom::Nearest { .. }
        | FlatAtom::Closure { .. } => {
            state.given.extend(variables(atom));
            state.enumerated = Enumerated::Already;
            state.rows = RowEstimate(state.rows.0.saturating_mul(next.rows.0)).min(ESTIMATE_CAP);
        }
        // A per-row meaning gives its outputs: the rows stay as they are.
        FlatAtom::Called { .. } => state.given.extend(variables(atom)),
        FlatAtom::Decided { .. } | FlatAtom::Test { .. } => {
            state.rows = RowEstimate((state.rows.0 / 2).max(1));
        }
    }
    state.cost = RowEstimate(state.cost.0.saturating_add(state.rows.0)).min(ESTIMATE_CAP);
}

/// The unplaced atoms, by place.
fn waiting<'a>(state: &'a OrderCompletion<'_>) -> impl Iterator<Item = WaitingAtom<'a>> {
    let atoms = state.branch.atoms.iter().enumerate();
    atoms
        .filter(|(at, _)| state.placed.get(*at) == Some(&AtomPlacing::Waiting))
        .map(|(at, atom)| WaitingAtom {
            place: AtomPlace(at),
            atom,
        })
}

/// Whether the unit may run a test or a decision now: a branch and a pipeline run none before their first enumeration,
/// which is their first step or stage.
fn settled(state: &OrderCompletion<'_>) -> TermGiven {
    if state.role != UnitRole::Pattern && state.enumerated == Enumerated::NotYet {
        TermGiven::Open
    } else {
        TermGiven::Given
    }
}

/// Whether a test of these terms can be written now. A program holds the file; a pipeline reads the file's cell from a
/// row at hand that holds it: a relation's row, or the definition's row its scan starts from (a keyed use gives no row).
fn file_held(state: &OrderCompletion<'_>, terms: [&FlatTerm; 2]) -> TermGiven {
    let asked = terms.iter().any(|term| **term == FlatTerm::File);
    let scanned = state.order.first().copied();
    let mut atoms = state.branch.atoms.iter().enumerate().zip(&state.placed);
    let held = atoms.any(|((place, atom), placing)| match (placing, atom) {
        (AtomPlacing::Placed, FlatAtom::Rows { rows, cells, .. }) => {
            cells.contains(&FlatTerm::File)
                && match rows {
                    AtomRows::Relation(_) => true,
                    AtomRows::Definition(_) => scanned == Some(AtomPlace(place)),
                    AtomRows::Made(_) => false,
                }
        }
        (AtomPlacing::Waiting, FlatAtom::Rows { .. })
        | (
            AtomPlacing::Placed | AtomPlacing::Waiting,
            FlatAtom::Latest { .. }
            | FlatAtom::Decided { .. }
            | FlatAtom::Walk { .. }
            | FlatAtom::Nearest { .. }
            | FlatAtom::Closure { .. }
            | FlatAtom::Called { .. }
            | FlatAtom::Test { .. },
        ) => false,
    });
    if asked && state.role == UnitRole::Pipeline && !held {
        TermGiven::Open
    } else {
        TermGiven::Given
    }
}

/// The first waiting test whose terms are given: a walk between two given elements is a test of them.
fn test(state: &OrderCompletion<'_>) -> Option<NextAccess> {
    let given = given_now(state);
    let found = waiting(state).find(|waiting| match waiting.atom {
        FlatAtom::Test { left, right, .. }
        | FlatAtom::Walk {
            start: left,
            reached: right,
            ..
        } => {
            all_ready([left, right], &given) == settled(state)
                && file_held(state, [left, right]) == TermGiven::Given
        }
        // A per-row meaning runs as soon as what it reads is given.
        FlatAtom::Called { arguments, .. } => all_ready(arguments, &given) == settled(state),
        FlatAtom::Rows { .. }
        | FlatAtom::Latest { .. }
        | FlatAtom::Nearest { .. }
        | FlatAtom::Closure { .. }
        | FlatAtom::Decided { .. } => false,
    });
    found
        .filter(|_| settled(state) == TermGiven::Given)
        .map(|waiting| NextAccess {
            atom: waiting.place,
            rows: RowEstimate(1),
        })
}

/// The first waiting decision whose asked cells are given.
fn decision(state: &OrderCompletion<'_>) -> Option<NextAccess> {
    let given = given_now(state);
    let found = waiting(state).find(|waiting| match waiting.atom {
        FlatAtom::Decided { cells, .. } => {
            all_ready(cells.iter().flatten(), &given) == TermGiven::Given
        }
        FlatAtom::Rows { .. }
        | FlatAtom::Latest { .. }
        | FlatAtom::Walk { .. }
        | FlatAtom::Nearest { .. }
        | FlatAtom::Closure { .. }
        | FlatAtom::Called { .. }
        | FlatAtom::Test { .. } => false,
    });
    found
        .filter(|_| settled(state) == TermGiven::Given)
        .map(|waiting| NextAccess {
            atom: waiting.place,
            rows: RowEstimate(1),
        })
}

/// The cheapest waiting access of the wanted class: keyed (a fan-out), or with no key (the file's rows).
fn cheapest(state: &OrderCompletion<'_>, class: CostClass) -> Option<NextAccess> {
    let given = given_now(state);
    let keyed = |ready: TermGiven| ready == TermGiven::Given && class == CostClass::Fanout;
    waiting(state)
        .filter_map(|waiting| {
            let (atom, each) = (waiting.place, waiting.atom);
            let rows = match each {
                FlatAtom::Rows { .. } => access(state.drafts, each, &given, state.enumerated)
                    .filter(|choice| choice.class == class)
                    .map(|choice| choice.rows),
                FlatAtom::Latest { key, bound, .. } => {
                    keyed(all_ready(key.iter().chain([bound]), &given)).then_some(RowEstimate(1))
                }
                FlatAtom::Walk {
                    direction,
                    start,
                    reached,
                    ..
                } => walk_rows(*direction, ready(*start, &given), ready(*reached, &given))
                    .filter(|_| class == CostClass::Fanout),
                FlatAtom::Nearest { start, .. } => {
                    keyed(ready(*start, &given)).then_some(RowEstimate(1))
                }
                // A closure walks from its source, or back from what is reached, within keys that are given.
                FlatAtom::Closure {
                    keys,
                    source,
                    reached,
                    ..
                } => {
                    let entered = ready(*source, &given) == TermGiven::Given
                        || ready(*reached, &given) == TermGiven::Given;
                    (entered && keyed(all_ready(keys, &given))).then_some(DESCENDANTS)
                }
                FlatAtom::Decided { .. } | FlatAtom::Called { .. } | FlatAtom::Test { .. } => None,
            };
            rows.map(|rows| NextAccess { atom, rows })
        })
        .min_by_key(|next| (next.rows, next.atom))
}

/// One completion of a branch's order from `first`, when given: none when the role forbids an access the branch needs.
fn completed(
    drafts: &Drafts,
    branch: &FlatBranch,
    given: &HashSet<FlatVariable>,
    role: UnitRole,
    first: Option<NextAccess>,
) -> Option<PlannedBranch> {
    let mut state = OrderCompletion {
        drafts,
        branch,
        role,
        given: given.clone(),
        pins: pins_of(branch),
        placed: vec![AtomPlacing::Waiting; branch.atoms.len()],
        order: Vec::with_capacity(branch.atoms.len()),
        rows: RowEstimate(1),
        cost: RowEstimate(0),
        enumerated: Enumerated::NotYet,
        products: Count(0),
    };
    if let Some(first) = first {
        place(&mut state, first);
    }
    while state.order.len() < branch.atoms.len() {
        if let Some(next) = test(&state) {
            place(&mut state, next);
            continue;
        }
        let keyed = cheapest(&state, CostClass::Fanout);
        if let Some(next) = keyed.filter(|next| next.rows.0 <= 1) {
            place(&mut state, next);
            continue;
        }
        if let Some(next) = decision(&state).or(keyed) {
            place(&mut state, next);
            continue;
        }
        // No access is keyed: the unit's first enumeration, or a product stage of a pipeline; else there is no program.
        let next = cheapest(&state, CostClass::File)?;
        match (role, state.enumerated) {
            (UnitRole::Branch | UnitRole::Pipeline, Enumerated::NotYet) => place(&mut state, next),
            (UnitRole::Pipeline, Enumerated::Already) => {
                state.products = Count(state.products.0.saturating_add(1));
                place(&mut state, next);
            }
            (UnitRole::Pattern, _) | (UnitRole::Branch, Enumerated::Already) => return None,
        }
    }
    Some(PlannedBranch {
        order: state.order,
        cost: state.cost,
        products: state.products,
    })
}

/// The order of a branch's atoms for a role, with `given` variables bound at its start. A branch and a pipeline try their
/// cheapest anchors and keep the cheapest completion; a pattern starts from its given cells. A completion with fewer
/// product stages goes before a cheaper one with more, and when each from the cheapest anchors holds one the other
/// anchors are tried too: a pipeline takes a product stage only when no order is keyed throughout. None: the role has
/// no program for the branch, because some access of it has no key.
#[must_use]
pub(crate) fn planned(
    drafts: &Drafts,
    branch: &FlatBranch,
    given: &HashSet<FlatVariable>,
    role: UnitRole,
) -> Option<PlannedBranch> {
    if role == UnitRole::Pattern {
        return completed(drafts, branch, given, role, None);
    }
    let pins = pins_of(branch);
    let start = GivenTerms {
        variables: given,
        pins: &pins,
    };
    let mut anchors: Vec<NextAccess> = branch
        .atoms
        .iter()
        .enumerate()
        .filter_map(|(at, atom)| {
            access(drafts, atom, &start, Enumerated::NotYet).map(|choice| NextAccess {
                atom: AtomPlace(at),
                rows: choice.rows,
            })
        })
        .collect();
    anchors.sort_by_key(|anchor| (anchor.rows, anchor.atom));
    if anchors.is_empty() {
        return completed(drafts, branch, given, role, None);
    }
    let best = |anchors: &[NextAccess]| {
        let plans = anchors
            .iter()
            .filter_map(|anchor| completed(drafts, branch, given, role, Some(*anchor)));
        plans.min_by_key(|plan| (plan.products.0, plan.cost))
    };
    let (first, rest) = anchors.split_at(anchors.len().min(ANCHORS_TRIED));
    match best(first) {
        Some(plan) if plan.products.0 > 0 => {
            let others = [Some(plan), best(rest)].into_iter().flatten();
            others.min_by_key(|plan| (plan.products.0, plan.cost))
        }
        keyed => keyed,
    }
}
