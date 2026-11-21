//! The services of the `branches` part: what a file must hold for a branch to give a row, read off the branch itself. Each
//! of its rows atoms needs a row of its source; a cell held to a constant names the value the row holds in that column; and
//! a use of a definition with a cell held to a constant needs what the definition's own branches need with that column
//! held to it: a literal is carried through a definition into the need, and each reader of a shared definition has its own.

use crate::domain::lowered::types::{Emptiness, RowValue};
use crate::domain::lowering::definitions::service::{draft, text_word};
use crate::domain::lowering::definitions::types::Drafts;
use crate::domain::lowering::flattening::service::pins_of;
use crate::domain::lowering::flattening::types::{
    AtomRows, FlatAtom, FlatBranch, FlatTerm, FlatVariable,
};
use crate::domain::steps::types::{DefinitionId, Polarity};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::catalog::service::{fact_form, node_column};
use ryo_relations::domain::relations::service::{columns, kind};
use ryo_relations::domain::relations::types::{CellKind, FactForm, NodeColumn, Relation};
use std::collections::HashMap;

/// How deep a constant is carried through definitions, and how many branches of a definition may be left that can give
/// a row with it: past either, the use needs the definition's own formula. The branches LEFT are counted, not the
/// definition's: a definition of many branches that each hold the column to its own constant (a union of a relation's
/// kinds) is where a reader's constant decides most, leaving the one branch that holds it.
const CARRIED: (usize, usize) = (4, 8);

/// The constant a cell is held to: its own, or the one an equality pins its variable to.
fn held(pins: &HashMap<FlatVariable, FlatTerm>, cell: FlatTerm) -> Option<FlatTerm> {
    match cell {
        FlatTerm::Text(_) | FlatTerm::Number(_) => Some(cell),
        FlatTerm::Variable(variable) => pins.get(&variable).copied(),
        FlatTerm::File => None,
    }
}

/// The word a constant is in a column of a kind: a text in a symbol column, a number in a number column. None for any
/// other pair: nothing is said of a value the row may hold in another form.
fn word(kind: CellKind, constant: FlatTerm) -> Option<CellWord> {
    match (kind, constant) {
        (CellKind::Symbol, FlatTerm::Text(number)) => Some(text_word(number)),
        // A number in a symbol column is a closed set's word, held as its own symbol (`tables::literals`).
        (CellKind::Number | CellKind::Symbol, FlatTerm::Number(number)) => {
            Some(CellWord(number.0.cast_unsigned()))
        }
        (
            CellKind::Node
            | CellKind::Symbol
            | CellKind::Span
            | CellKind::Number
            | CellKind::Flag
            | CellKind::File,
            FlatTerm::Text(_) | FlatTerm::Number(_) | FlatTerm::Variable(_) | FlatTerm::File,
        ) => None,
    }
}

/// What a row of a relation with these cells needs: for the node table, a node of the kind or a child in the field a cell
/// is held to; for a filled relation, a row, with each value a cell is held to.
fn relation_need(
    relation: Relation,
    cells: &[Option<FlatTerm>],
    pins: &HashMap<FlatVariable, FlatTerm>,
    parts: &mut Vec<Emptiness>,
) {
    let filled = matches!(fact_form(relation), FactForm::Filled(_));
    let mut valued = false;
    for (column, cell) in columns(relation).into_iter().zip(cells) {
        let decided = match (filled, node_column(column)) {
            (true, _) | (false, Some(NodeColumn::Kind | NodeColumn::Field)) => true,
            (false, _) => false,
        };
        let value = cell
            .and_then(|cell| held(pins, cell))
            .and_then(|constant| word(kind(column), constant));
        if let (true, Some(value)) = (decided, value) {
            parts.push(Emptiness::Row(relation, Some(RowValue { column, value })));
            valued = true;
        }
    }
    if filled && !valued {
        parts.push(Emptiness::Row(relation, None));
    }
}

/// What a row of a definition with these cells needs: the definition's own formula; or, with a cell held to a constant,
/// what some branch of it needs with that column held to it.
fn definition_need(
    drafts: &Drafts,
    definition: DefinitionId,
    cells: &[Option<FlatTerm>],
    pins: &HashMap<FlatVariable, FlatTerm>,
    depth: usize,
) -> Emptiness {
    let constants: Vec<(usize, FlatTerm)> = cells
        .iter()
        .enumerate()
        .filter_map(|(place, cell)| {
            cell.and_then(|cell| held(pins, cell))
                .map(|held| (place, held))
        })
        .collect();
    let branches = draft(drafts, definition).map(|draft| &draft.branches);
    let Some(branches) = branches.filter(|_| !constants.is_empty() && depth < CARRIED.0) else {
        return Emptiness::Definition(definition);
    };
    let mut some = Vec::new();
    for branch in branches {
        let mut carried = HashMap::new();
        let mut possible = true;
        for (place, constant) in &constants {
            match branch.row.get(*place) {
                Some(FlatTerm::Variable(variable)) => {
                    let _old = carried.insert(*variable, *constant);
                }
                Some(own @ (FlatTerm::Text(_) | FlatTerm::Number(_))) => {
                    possible &= own == constant
                }
                Some(FlatTerm::File) | None => {}
            }
        }
        if possible {
            let need = needed(drafts, branch, &carried, depth.saturating_add(1));
            // A branch that holds the column to another constant gives no row: it is not a way to have one.
            if !never(&need) {
                some.push(need);
            }
        }
        if some.len() > CARRIED.1 {
            return Emptiness::Definition(definition);
        }
    }
    Emptiness::Any(some.into())
}

/// What a file must hold for a branch to give a row, with `carried` variables of it held to constants by its reader.
fn needed(
    drafts: &Drafts,
    branch: &FlatBranch,
    carried: &HashMap<FlatVariable, FlatTerm>,
    depth: usize,
) -> Emptiness {
    let mut pins = pins_of(branch);
    for (variable, constant) in carried {
        // A variable its own branch holds to another constant gives no row.
        if pins
            .insert(*variable, *constant)
            .is_some_and(|own| own != *constant)
        {
            return Emptiness::Any(Box::default());
        }
    }
    let (mut parts, mut uses) = (Vec::new(), Vec::new());
    for atom in &branch.atoms {
        let (rows, cells): (AtomRows, Vec<Option<FlatTerm>>) = match atom {
            FlatAtom::Rows { rows, cells, .. } => {
                (*rows, cells.iter().copied().map(Some).collect())
            }
            FlatAtom::Decided {
                polarity: Polarity::Present,
                rows,
                cells,
            } => (*rows, cells.clone()),
            FlatAtom::Decided {
                polarity: Polarity::Absent,
                ..
            }
            | FlatAtom::Latest { .. }
            | FlatAtom::Walk { .. }
            | FlatAtom::Nearest { .. }
            | FlatAtom::Closure { .. }
            | FlatAtom::Called { .. }
            | FlatAtom::Test { .. } => continue,
        };
        match rows {
            AtomRows::Relation(relation) => relation_need(relation, &cells, &pins, &mut parts),
            AtomRows::Definition(definition) => {
                uses.push(definition_need(drafts, definition, &cells, &pins, depth));
            }
            // A made table is asked for a row by making it: the branch's reader decides when.
            AtomRows::Made(_) => {}
        }
    }
    // What a relation says is asked before what another unit decides.
    parts.extend(uses);
    Emptiness::All(parts.into())
}

/// What a file must hold for a branch to give a row, read off the branch.
#[must_use]
pub(crate) fn branch_need(drafts: &Drafts, branch: &FlatBranch) -> Emptiness {
    needed(drafts, branch, &HashMap::new(), 0)
}

/// What a file must hold for some of a step's branches to give a row.
#[must_use]
pub(crate) fn branches_need(drafts: &Drafts, branches: &[FlatBranch]) -> Emptiness {
    let each = branches.iter().map(|branch| branch_need(drafts, branch));
    Emptiness::Any(each.collect())
}

/// Whether a formula holds in no file: some part of an `All` never does, or no part of an `Any` can.
fn never(formula: &Emptiness) -> bool {
    match formula {
        Emptiness::All(parts) => parts.iter().any(never),
        Emptiness::Any(parts) => parts.iter().all(never),
        Emptiness::Unknown
        | Emptiness::Kind(_)
        | Emptiness::Row(..)
        | Emptiness::Definition(_)
        | Emptiness::Table(_) => false,
    }
}

/// Whether a formula holds in every file: it says nothing.
fn always(formula: &Emptiness) -> bool {
    match formula {
        Emptiness::All(parts) => parts.iter().all(always),
        Emptiness::Any(parts) => parts.iter().any(always),
        Emptiness::Unknown
        | Emptiness::Kind(_)
        | Emptiness::Row(..)
        | Emptiness::Definition(_)
        | Emptiness::Table(_) => false,
    }
}

/// A formula with what the branches need beside it; the formula alone when the branches' need says nothing.
#[must_use]
pub(crate) fn with_need(formula: Emptiness, need: Emptiness) -> Emptiness {
    if always(&need) {
        formula
    } else {
        Emptiness::All([formula, need].into())
    }
}
