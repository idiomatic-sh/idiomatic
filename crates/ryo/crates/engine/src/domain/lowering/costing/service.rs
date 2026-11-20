//! The services of the `costing` part: the rows of a source, the access of an atom for the cells that stand bound, and the
//! rows of a branch, all by the fixed priors. A use is costed from the definition's own estimate, never from a caller's.

use super::constants::{
    CHILDREN, ELEMENTS, ESTIMATE_CAP, FILLED_ROWS, GROUP_ROWS, KEY_SHARE, KIND_SHARE,
    LITERAL_SHARE, MANY_ROWS,
};
use super::types::{CellState, KeyChoice};
use crate::domain::lowering::definitions::service::draft;
use crate::domain::lowering::definitions::types::{Drafts, RowEstimate};
use crate::domain::lowering::flattening::types::{
    AtomRows, FlatAtom, FlatBranch, FlatTerm, RowsPerFile,
};
use crate::domain::steps::types::CostClass;
use ryo_relations::domain::catalog::service::{fact_form, node_column};
use ryo_relations::domain::relations::service::columns;
use ryo_relations::domain::relations::types::{FactForm, FilledKey, NodeColumn, Relation};

/// One row in `share` of an estimate, and never none.
const fn shared(rows: RowEstimate, share: u64) -> RowEstimate {
    let part = rows.0 / share;
    RowEstimate(if part == 0 { 1 } else { part })
}

/// The rows a source is taken to hold in a file.
#[must_use]
pub(crate) fn source_rows(drafts: &Drafts, rows: AtomRows) -> RowEstimate {
    match rows {
        AtomRows::Relation(relation) => match fact_form(relation) {
            FactForm::NodeTable => ELEMENTS,
            FactForm::Filled(_) => FILLED_ROWS,
        },
        AtomRows::Definition(definition) => {
            draft(drafts, definition).map_or(RowEstimate(1), |draft| draft.rows)
        }
        AtomRows::Made(_) => GROUP_ROWS,
    }
}

/// The places of the cells in a state.
fn standing(states: &[CellState], state: CellState) -> Vec<usize> {
    let places = states.iter().enumerate();
    places
        .filter(|(_, each)| **each == state)
        .map(|(place, _)| place)
        .collect()
}

/// The access of the node table through a relation: it is keyed by one column. A register in the element's own column, its
/// parent's or its span's is a key; with none, a constant kind or text names a bucket of the file.
fn node_access(relation: Relation, states: &[CellState]) -> KeyChoice {
    let listed = columns(relation);
    let cell = |wanted: NodeColumn, state: CellState| {
        listed.iter().zip(states).position(|(column, each)| {
            *each == state && node_column(*column) == Some(wanted)
        })
    };
    let keys = [
        (NodeColumn::Id, CellState::Register, CostClass::Fanout, RowEstimate(1)),
        (NodeColumn::Parent, CellState::Register, CostClass::Fanout, CHILDREN),
        (NodeColumn::Span, CellState::Register, CostClass::Fanout, RowEstimate(1)),
        (NodeColumn::Kind, CellState::Constant, CostClass::File, shared(ELEMENTS, KIND_SHARE)),
        (NodeColumn::Text, CellState::Constant, CostClass::File, shared(ELEMENTS, LITERAL_SHARE)),
    ];
    for (column, state, class, rows) in keys {
        if let Some(place) = cell(column, state) {
            return KeyChoice {
                cells: vec![place],
                class,
                rows,
            };
        }
    }
    KeyChoice {
        cells: Vec::new(),
        class: CostClass::File,
        rows: ELEMENTS,
    }
}

/// The access of a source that takes every bound cell as one key: keyed by a register it is a fan-out, by constants alone a
/// bucket of the file, by nothing the file's rows. `own` is the rows a register key reaches.
fn composite_access(states: &[CellState], whole: RowEstimate, own: RowEstimate) -> KeyChoice {
    let registers = standing(states, CellState::Register);
    let constants = standing(states, CellState::Constant);
    let (class, rows) = match (registers.len(), constants.len()) {
        (0, 0) => (CostClass::File, whole),
        (0, _) => (CostClass::File, shared(whole, LITERAL_SHARE)),
        (1, 0) => (CostClass::Fanout, own),
        (_, _) => (CostClass::Fanout, shared(own, 2)),
    };
    let mut cells: Vec<usize> = registers.into_iter().chain(constants).collect();
    cells.sort_unstable();
    KeyChoice { cells, class, rows }
}

/// The access of an atom's rows for the cells that stand bound. A source that holds one row a file is keyed by the file.
#[must_use]
pub(crate) fn keyed(
    drafts: &Drafts,
    rows: AtomRows,
    per_file: RowsPerFile,
    states: &[CellState],
) -> KeyChoice {
    if per_file == RowsPerFile::One {
        return KeyChoice {
            cells: Vec::new(),
            class: CostClass::Fanout,
            rows: RowEstimate(1),
        };
    }
    let whole = source_rows(drafts, rows);
    match rows {
        AtomRows::Relation(relation) => match fact_form(relation) {
            FactForm::NodeTable => node_access(relation, states),
            FactForm::Filled(key) => {
                let listed = columns(relation);
                let by_own = |own| {
                    listed.iter().zip(states).any(|(column, state)| {
                        *column == own && *state == CellState::Register
                    })
                };
                let own = match key {
                    FilledKey::Unique(column) if by_own(column) => RowEstimate(1),
                    FilledKey::Many(column) if by_own(column) => MANY_ROWS,
                    FilledKey::Unique(_) | FilledKey::Many(_) | FilledKey::Whole => {
                        shared(whole, KEY_SHARE)
                    }
                };
                composite_access(states, whole, own)
            }
        },
        // A use keyed by constants alone names a bucket of the definition's rows: file-sized, as any bucket is.
        AtomRows::Definition(_) => composite_access(states, whole, shared(whole, KEY_SHARE)),
        AtomRows::Made(_) => composite_access(states, whole, RowEstimate(1)),
    }
}

/// The rows a branch is taken to give: those of its smallest source, a constant cell naming a bucket of it. A branch of no
/// source gives one row.
#[must_use]
pub(crate) fn branch_rows(drafts: &Drafts, branch: &FlatBranch) -> RowEstimate {
    let mut least = ESTIMATE_CAP;
    let mut any = false;
    for atom in &branch.atoms {
        if let FlatAtom::Rows {
            rows,
            cells,
            per_file,
        } = atom
        {
            let states: Vec<CellState> = cells
                .iter()
                .map(|cell| match cell {
                    FlatTerm::Variable(_) => CellState::Open,
                    FlatTerm::Text(_) | FlatTerm::Number(_) => CellState::Constant,
                    FlatTerm::File => CellState::File,
                })
                .collect();
            least = least.min(keyed(drafts, *rows, *per_file, &states).rows);
            any = true;
        }
    }
    if any { least } else { RowEstimate(1) }
}

/// The rows of a step's branches together, held to the cap.
#[must_use]
pub(crate) fn branches_rows(drafts: &Drafts, branches: &[FlatBranch]) -> RowEstimate {
    let sum = branches
        .iter()
        .fold(0_u64, |sum, branch| sum.saturating_add(branch_rows(drafts, branch).0));
    RowEstimate(sum).min(ESTIMATE_CAP)
}
