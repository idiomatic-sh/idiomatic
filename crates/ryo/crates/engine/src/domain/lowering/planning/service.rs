//! The services of the `planning` part: the order of a branch's atoms. A test runs as soon as its terms are given; a keyed
//! access that gives one row runs before a decision of a definition (a dear test waits for a keyed lookup); the cheapest
//! keyed access goes next. An access with no key is the unit's first enumeration, or a product stage of a pipeline: a branch
//! and a pattern that would need one anywhere else have no program, and the caller takes the table form.

use super::types::{
    AtomPlace, Completion, Enumerated, GivenTerms, NextAccess, Placing, PlannedBranch, TermGiven,
    UnitRole,
};
use crate::domain::lowering::costing::constants::{
    ANCESTORS, ANCHORS_TRIED, CHILDREN, DESCENDANTS, ESTIMATE_CAP,
};
use crate::domain::lowering::costing::service::keyed;
use crate::domain::lowering::costing::types::{CellState, KeyChoice};
use crate::domain::lowering::definitions::types::{Drafts, RowEstimate};
use crate::domain::lowering::flattening::service::{pins_of, variables};
use crate::domain::lowering::flattening::types::{
    FlatAtom, FlatBranch, FlatTerm, FlatVariable, RowsPerFile,
};
use crate::domain::steps::types::{AxisDirection, CostClass};
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
fn all_ready<'t>(terms: impl IntoIterator<Item = &'t FlatTerm>, given: &GivenTerms<'_>) -> TermGiven {
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
fn given_now<'a>(state: &'a Completion<'_>) -> GivenTerms<'a> {
    GivenTerms {
        variables: &state.given,
        pins: &state.pins,
    }
}

/// Places an atom: its variables are given from here on, and its rows are visited.
fn place(state: &mut Completion<'_>, next: NextAccess) {
    let Some(atom) = state.branch.atoms.get(next.atom.0) else {
        return;
    };
    if let Some(placed) = state.placed.get_mut(next.atom.0) {
        *placed = Placing::Placed;
    }
    state.order.push(next.atom);
    match atom {
        FlatAtom::Rows { .. }
        | FlatAtom::Latest { .. }
        | FlatAtom::Walk { .. }
        | FlatAtom::Nearest { .. } => {
            state.given.extend(variables(atom));
            state.enumerated = Enumerated::Already;
            state.rows = RowEstimate(state.rows.0.saturating_mul(next.rows.0)).min(ESTIMATE_CAP);
        }
        FlatAtom::Decided { .. } | FlatAtom::Test { .. } => {
            state.rows = RowEstimate((state.rows.0 / 2).max(1));
        }
    }
    state.cost = RowEstimate(state.cost.0.saturating_add(state.rows.0)).min(ESTIMATE_CAP);
}

/// The unplaced atoms, by place.
fn waiting<'a>(state: &'a Completion<'_>) -> impl Iterator<Item = (AtomPlace, &'a FlatAtom)> {
    let atoms = state.branch.atoms.iter().enumerate();
    atoms
        .filter(|(at, _)| state.placed.get(*at) == Some(&Placing::Waiting))
        .map(|(at, atom)| (AtomPlace(at), atom))
}

/// Whether the unit may run a test or a decision now: a branch and a pipeline run none before their first enumeration,
/// which is their first step or stage.
fn settled(state: &Completion<'_>) -> TermGiven {
    if state.role != UnitRole::Pattern && state.enumerated == Enumerated::NotYet {
        TermGiven::Open
    } else {
        TermGiven::Given
    }
}

/// The first waiting test whose terms are given: a walk between two given elements is a test of them.
fn test(state: &Completion<'_>) -> Option<NextAccess> {
    let given = given_now(state);
    let found = waiting(state).find(|(_, atom)| match atom {
        FlatAtom::Test { left, right, .. }
        | FlatAtom::Walk {
            start: left,
            reached: right,
            ..
        } => all_ready([left, right], &given) == settled(state),
        FlatAtom::Rows { .. }
        | FlatAtom::Latest { .. }
        | FlatAtom::Nearest { .. }
        | FlatAtom::Decided { .. } => false,
    });
    found
        .filter(|_| settled(state) == TermGiven::Given)
        .map(|(atom, _)| NextAccess {
            atom,
            rows: RowEstimate(1),
        })
}

/// The first waiting decision whose asked cells are given.
fn decision(state: &Completion<'_>) -> Option<NextAccess> {
    let given = given_now(state);
    let found = waiting(state).find(|(_, atom)| match atom {
        FlatAtom::Decided { cells, .. } => {
            all_ready(cells.iter().flatten(), &given) == TermGiven::Given
        }
        FlatAtom::Rows { .. }
        | FlatAtom::Latest { .. }
        | FlatAtom::Walk { .. }
        | FlatAtom::Nearest { .. }
        | FlatAtom::Test { .. } => false,
    });
    found
        .filter(|_| settled(state) == TermGiven::Given)
        .map(|(atom, _)| NextAccess {
            atom,
            rows: RowEstimate(1),
        })
}

/// The rows a walk gives from the one end of it that is given: what that end reaches along the axis, or against it. None
/// when both ends are given (the walk is a test) or neither is.
fn walk_rows(direction: AxisDirection, start: TermGiven, reached: TermGiven) -> Option<RowEstimate> {
    match (start, reached, direction) {
        (TermGiven::Given, TermGiven::Open, AxisDirection::Parent) => Some(RowEstimate(1)),
        (TermGiven::Given, TermGiven::Open, AxisDirection::Ancestor)
        | (TermGiven::Open, TermGiven::Given, AxisDirection::Descendant) => Some(ANCESTORS),
        (TermGiven::Given, TermGiven::Open, AxisDirection::Descendant)
        | (TermGiven::Open, TermGiven::Given, AxisDirection::Ancestor) => Some(DESCENDANTS),
        (TermGiven::Open, TermGiven::Given, AxisDirection::Parent)
        | (
            TermGiven::Given | TermGiven::Open,
            TermGiven::Given | TermGiven::Open,
            AxisDirection::NextSibling | AxisDirection::PreviousSibling,
        ) if start != reached => Some(CHILDREN),
        (TermGiven::Given | TermGiven::Open, TermGiven::Given | TermGiven::Open, _) => None,
    }
}

/// The cheapest waiting access of the wanted class: keyed (a fan-out), or with no key (the file's rows).
fn cheapest(state: &Completion<'_>, class: CostClass) -> Option<NextAccess> {
    let given = given_now(state);
    let keyed = |ready: TermGiven| ready == TermGiven::Given && class == CostClass::Fanout;
    waiting(state)
        .filter_map(|(atom, each)| {
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
                FlatAtom::Decided { .. } | FlatAtom::Test { .. } => None,
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
    let mut state = Completion {
        drafts,
        branch,
        role,
        given: given.clone(),
        pins: pins_of(branch),
        placed: vec![Placing::Waiting; branch.atoms.len()],
        order: Vec::with_capacity(branch.atoms.len()),
        rows: RowEstimate(1),
        cost: RowEstimate(0),
        enumerated: Enumerated::NotYet,
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
            (UnitRole::Branch | UnitRole::Pipeline, Enumerated::NotYet)
            | (UnitRole::Pipeline, Enumerated::Already) => place(&mut state, next),
            (UnitRole::Pattern, _) | (UnitRole::Branch, Enumerated::Already) => return None,
        }
    }
    Some(PlannedBranch {
        order: state.order,
        cost: state.cost,
    })
}

/// The order of a branch's atoms for a role, with `given` variables bound at its start. A branch and a pipeline try their
/// cheapest anchors and keep the cheapest completion; a pattern starts from its given cells. None: the role has no program
/// for the branch, because some access of it has no key.
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
    anchors
        .into_iter()
        .take(ANCHORS_TRIED)
        .filter_map(|anchor| completed(drafts, branch, given, role, Some(anchor)))
        .min_by_key(|plan| plan.cost)
}
