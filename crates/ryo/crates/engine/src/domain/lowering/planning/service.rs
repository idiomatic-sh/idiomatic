//! The services of the `planning` part: the order of a branch's atoms. A test runs as soon as its terms are given; a keyed
//! access that gives one row runs before a decision of a definition (a dear test waits for a keyed lookup); the cheapest
//! keyed access goes next. An access with no key is the unit's first enumeration, or a product stage of a pipeline: a branch
//! and a pattern that would need one anywhere else have no program, and the caller takes the table form.

use super::types::{Enumerated, PlannedBranch, UnitRole};
use crate::domain::lowering::costing::constants::{ANCHORS_TRIED, ESTIMATE_CAP};
use crate::domain::lowering::costing::service::keyed;
use crate::domain::lowering::costing::types::{CellState, KeyChoice};
use crate::domain::lowering::definitions::types::{Drafts, RowEstimate};
use crate::domain::lowering::flattening::types::{FlatAtom, FlatBranch, FlatTerm, FlatVariable, RowsPerFile};
use crate::domain::steps::types::CostClass;
use std::collections::HashSet;

/// How a term stands for the variables given so far.
#[must_use]
pub(crate) fn state_of(term: FlatTerm, given: &HashSet<FlatVariable>) -> CellState {
    match term {
        FlatTerm::Variable(variable) if given.contains(&variable) => CellState::Register,
        FlatTerm::Variable(_) => CellState::Open,
        FlatTerm::Text(_) | FlatTerm::Number(_) => CellState::Constant,
        FlatTerm::File => CellState::File,
    }
}

/// Whether a term is given: anything but a variable nothing gave yet.
fn ready(term: FlatTerm, given: &HashSet<FlatVariable>) -> bool {
    state_of(term, given) != CellState::Open
}

/// The access of a rows atom now, and whether it may be placed as a keyed access: a source of one row a file is keyed by the
/// file once the unit has enumerated, and is the unit's own first enumeration before.
fn access(
    drafts: &Drafts,
    atom: &FlatAtom,
    given: &HashSet<FlatVariable>,
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

/// The state of one completion of a branch's order.
struct Completion<'a> {
    /// The drafts the estimates read.
    drafts: &'a Drafts,
    /// The branch.
    branch: &'a FlatBranch,
    /// The variables given so far.
    given: HashSet<FlatVariable>,
    /// Which atoms are placed.
    placed: Vec<bool>,
    /// The order so far.
    order: Vec<usize>,
    /// The rows the unit holds at this point.
    rows: RowEstimate,
    /// The rows visited so far.
    cost: RowEstimate,
    /// Whether rows were enumerated yet.
    enumerated: Enumerated,
}

impl Completion<'_> {
    /// Places an atom: its variables are given from here on, and its rows are visited.
    fn place(&mut self, at: usize, gives: RowEstimate) {
        let Some(atom) = self.branch.atoms.get(at) else {
            return;
        };
        if let Some(placed) = self.placed.get_mut(at) {
            *placed = true;
        }
        self.order.push(at);
        match atom {
            FlatAtom::Rows { .. } | FlatAtom::Latest { .. } => {
                self.given.extend(crate::domain::lowering::flattening::service::variables(atom));
                self.enumerated = Enumerated::Already;
                self.rows = RowEstimate(self.rows.0.saturating_mul(gives.0)).min(ESTIMATE_CAP);
            }
            FlatAtom::Decided { .. } | FlatAtom::Test { .. } => {
                self.rows = RowEstimate((self.rows.0 / 2).max(1));
            }
        }
        self.cost = RowEstimate(self.cost.0.saturating_add(self.rows.0)).min(ESTIMATE_CAP);
    }

    /// The unplaced atoms, by place.
    fn waiting(&self) -> impl Iterator<Item = (usize, &FlatAtom)> {
        let atoms = self.branch.atoms.iter().enumerate();
        atoms.filter(|(at, _)| self.placed.get(*at) == Some(&false))
    }

    /// The first waiting test whose terms are given. A branch and a pipeline run no test before their first enumeration,
    /// which is their first step or stage.
    fn test(&self, role: UnitRole) -> Option<usize> {
        if role != UnitRole::Pattern && self.enumerated == Enumerated::NotYet {
            return None;
        }
        self.waiting()
            .find(|(_, atom)| match atom {
                FlatAtom::Test { left, right, .. } => {
                    ready(*left, &self.given) && ready(*right, &self.given)
                }
                FlatAtom::Rows { .. } | FlatAtom::Latest { .. } | FlatAtom::Decided { .. } => false,
            })
            .map(|(at, _)| at)
    }

    /// The first waiting decision whose asked cells are given.
    fn decision(&self, role: UnitRole) -> Option<usize> {
        if role != UnitRole::Pattern && self.enumerated == Enumerated::NotYet {
            return None;
        }
        self.waiting()
            .find(|(_, atom)| match atom {
                FlatAtom::Decided { cells, .. } => {
                    cells.iter().flatten().all(|cell| ready(*cell, &self.given))
                }
                FlatAtom::Rows { .. } | FlatAtom::Latest { .. } | FlatAtom::Test { .. } => false,
            })
            .map(|(at, _)| at)
    }

    /// The cheapest waiting access of the wanted class: keyed (a fan-out), or with no key (the file's rows).
    fn cheapest(&self, class: CostClass) -> Option<(usize, RowEstimate)> {
        self.waiting()
            .filter_map(|(at, atom)| match atom {
                FlatAtom::Rows { .. } => access(self.drafts, atom, &self.given, self.enumerated)
                    .filter(|choice| choice.class == class)
                    .map(|choice| (at, choice.rows)),
                FlatAtom::Latest { key, bound, .. } => {
                    let given = key.iter().chain([bound]).all(|cell| ready(*cell, &self.given));
                    (given && class == CostClass::Fanout).then_some((at, RowEstimate(1)))
                }
                FlatAtom::Decided { .. } | FlatAtom::Test { .. } => None,
            })
            .min_by_key(|(at, rows)| (*rows, *at))
    }
}

/// One completion of a branch's order from `first`, when given: none when the role forbids an access the branch needs.
fn completed(
    drafts: &Drafts,
    branch: &FlatBranch,
    given: &HashSet<FlatVariable>,
    role: UnitRole,
    first: Option<(usize, RowEstimate)>,
) -> Option<PlannedBranch> {
    let mut state = Completion {
        drafts,
        branch,
        given: given.clone(),
        placed: vec![false; branch.atoms.len()],
        order: Vec::with_capacity(branch.atoms.len()),
        rows: RowEstimate(1),
        cost: RowEstimate(0),
        enumerated: Enumerated::NotYet,
    };
    if let Some((at, rows)) = first {
        state.place(at, rows);
    }
    while state.order.len() < branch.atoms.len() {
        if let Some(at) = state.test(role) {
            state.place(at, RowEstimate(1));
            continue;
        }
        let keyed = state.cheapest(CostClass::Fanout);
        if let Some((at, rows)) = keyed.filter(|(_, rows)| rows.0 <= 1) {
            state.place(at, rows);
            continue;
        }
        if let Some(at) = state.decision(role) {
            state.place(at, RowEstimate(1));
            continue;
        }
        if let Some((at, rows)) = keyed {
            state.place(at, rows);
            continue;
        }
        // No access is keyed: the unit's first enumeration, or a product stage of a pipeline; else there is no program.
        let (at, rows) = state.cheapest(CostClass::File)?;
        let allowed = match (role, state.enumerated) {
            (UnitRole::Branch | UnitRole::Pipeline, Enumerated::NotYet)
            | (UnitRole::Pipeline, Enumerated::Already) => true,
            (UnitRole::Pattern, _) | (UnitRole::Branch, Enumerated::Already) => false,
        };
        if !allowed {
            return None;
        }
        state.place(at, rows);
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
    let mut anchors: Vec<(usize, RowEstimate)> = branch
        .atoms
        .iter()
        .enumerate()
        .filter_map(|(at, atom)| {
            access(drafts, atom, given, Enumerated::NotYet).map(|choice| (at, choice.rows))
        })
        .collect();
    anchors.sort_by_key(|(at, rows)| (*rows, *at));
    if anchors.is_empty() {
        return completed(drafts, branch, given, role, None);
    }
    anchors
        .into_iter()
        .take(ANCHORS_TRIED)
        .filter_map(|anchor| completed(drafts, branch, given, role, Some(anchor)))
        .min_by_key(|plan| plan.cost)
}
