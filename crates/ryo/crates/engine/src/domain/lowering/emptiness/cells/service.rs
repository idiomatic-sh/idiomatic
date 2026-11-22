//! The services of the `cells` part: which of a branch's variables hold a text, read off its rows atoms: a text of the
//! file's own tree, or a text cell of a relation the language fills.

use super::types::FilledCell;
use crate::domain::lowering::flattening::types::{AtomRows, FlatAtom, FlatBranch, FlatTerm, FlatVariable};
use crate::domain::steps::types::Polarity;
use ryo_relations::domain::catalog::service::{fact_form, node_column, node_extent};
use ryo_relations::domain::relations::service::{columns, kind};
use ryo_relations::domain::relations::types::{CellKind, FactForm, NodeColumn, NodeExtent, Relation};
use std::collections::HashMap;

/// The variables of a branch that hold a text of the file's own tree: a cell of a node relation's `Text` column, in a
/// relation whose rows are the file's own tree (`NodeExtent::Own`). Such a text is a slice of the file's bytes, or a
/// text the language decoded for the element when the table was built (kept among the file's derived names): the
/// node table's declaration (`NodeColumn::Text`). An embedded segment's logical text is not counted: it may be made
/// after the file's first decision.
pub(crate) fn own_texts(branch: &FlatBranch) -> Vec<FlatVariable> {
    let mut texts = Vec::new();
    for atom in &branch.atoms {
        let (relation, cells): (Relation, Vec<Option<FlatTerm>>) = match atom {
            FlatAtom::Rows {
                rows: AtomRows::Relation(relation),
                cells,
                ..
            } => (*relation, cells.iter().copied().map(Some).collect()),
            FlatAtom::Decided {
                polarity: Polarity::Present,
                rows: AtomRows::Relation(relation),
                cells,
            } => (*relation, cells.clone()),
            FlatAtom::Rows { .. }
            | FlatAtom::Decided { .. }
            | FlatAtom::Latest { .. }
            | FlatAtom::Walk { .. }
            | FlatAtom::Nearest { .. }
            | FlatAtom::Closure { .. }
            | FlatAtom::Called { .. }
            | FlatAtom::Test { .. } => continue,
        };
        if !matches!(fact_form(relation), FactForm::NodeTable) || node_extent(relation) != Some(NodeExtent::Own) {
            continue;
        }
        for (column, cell) in columns(relation).into_iter().zip(cells) {
            if let (Some(NodeColumn::Text), Some(FlatTerm::Variable(variable))) = (node_column(column), cell) {
                texts.push(variable);
            }
        }
    }
    texts
}

/// The text cells of a branch's rows of filled relations: each variable bound to a symbol column of a relation the
/// language fills, with that relation and column. A test such a cell must pass is a test some row of the relation
/// passes in the file.
pub(crate) fn filled_texts(branch: &FlatBranch) -> HashMap<FlatVariable, FilledCell> {
    let mut texts = HashMap::new();
    for atom in &branch.atoms {
        let (relation, cells): (Relation, Vec<Option<FlatTerm>>) = match atom {
            FlatAtom::Rows {
                rows: AtomRows::Relation(relation),
                cells,
                ..
            } => (*relation, cells.iter().copied().map(Some).collect()),
            FlatAtom::Decided {
                polarity: Polarity::Present,
                rows: AtomRows::Relation(relation),
                cells,
            } => (*relation, cells.clone()),
            FlatAtom::Rows { .. }
            | FlatAtom::Decided { .. }
            | FlatAtom::Latest { .. }
            | FlatAtom::Walk { .. }
            | FlatAtom::Nearest { .. }
            | FlatAtom::Closure { .. }
            | FlatAtom::Called { .. }
            | FlatAtom::Test { .. } => continue,
        };
        if !matches!(fact_form(relation), FactForm::Filled(_)) {
            continue;
        }
        for (column, cell) in columns(relation).into_iter().zip(cells) {
            if let (CellKind::Symbol, Some(FlatTerm::Variable(variable))) = (kind(column), cell) {
                let _first = texts.entry(variable).or_insert(FilledCell { relation, column });
            }
        }
    }
    texts
}
