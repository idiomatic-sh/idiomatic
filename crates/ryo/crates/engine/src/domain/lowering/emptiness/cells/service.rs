//! The services of the `cells` part: which of a branch's variables hold a text, read off its rows atoms: a text of the
//! file's own tree, or a text cell of a relation the language fills.

use super::types::{FilledCell, ShapeElement};
use crate::domain::lowered::types::{ShapeNode, ShapeStar, TreeShape};
use crate::domain::lowering::definitions::service::text_word;
use ryo_language::domain::facts::types::CellWord;
use crate::domain::lowering::flattening::types::{AtomRows, FlatAtom, FlatBranch, FlatTerm, FlatVariable};
use crate::domain::steps::types::{AxisDirection, Polarity};
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

/// The word of a constant text a cell holds, or is pinned to.
fn text_of(pins: &HashMap<FlatVariable, FlatTerm>, cell: Option<FlatTerm>) -> Option<CellWord> {
    let held = match cell? {
        FlatTerm::Variable(variable) => pins.get(&variable).copied()?,
        own => own,
    };
    match held {
        FlatTerm::Text(number) => Some(text_word(number)),
        FlatTerm::Variable(_) | FlatTerm::Number(_) | FlatTerm::File => None,
    }
}

/// What the branch holds of each element of the node table: read off its rows of node relations, each row an element
/// (its `Id` cell), with its kind, its field and its parent where the row holds them.
fn elements(
    branch: &FlatBranch,
    pins: &HashMap<FlatVariable, FlatTerm>,
) -> HashMap<FlatVariable, ShapeElement> {
    let mut found: HashMap<FlatVariable, ShapeElement> = HashMap::new();
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
        let Some(extent) = node_extent(relation) else {
            continue;
        };
        let held = columns(relation);
        let Some(FlatTerm::Variable(id)) = held
            .iter()
            .zip(&cells)
            .find(|(column, _)| node_column(**column) == Some(NodeColumn::Id))
            .and_then(|(_, cell)| *cell)
        else {
            continue;
        };
        let element = found.entry(id).or_default();
        element.own |= extent == NodeExtent::Own;
        for (column, cell) in held.into_iter().zip(cells) {
            match node_column(column) {
                Some(NodeColumn::Kind) => element.kind = element.kind.or(text_of(pins, cell)),
                Some(NodeColumn::Field) => element.field = element.field.or(text_of(pins, cell)),
                Some(NodeColumn::Parent) => {
                    if let Some(FlatTerm::Variable(parent)) = cell
                        && parent != id
                    {
                        element.parent = element.parent.or(Some(parent));
                    }
                }
                Some(
                    NodeColumn::Id
                    | NodeColumn::Ordinal
                    | NodeColumn::Start
                    | NodeColumn::End
                    | NodeColumn::Span
                    | NodeColumn::File
                    | NodeColumn::Text,
                )
                | None => {}
            }
        }
    }
    found
}

/// The elements a branch walks to above an element: the reached end of an ancestor axis from it, the start of a
/// descendant axis to it.
fn walked_above(branch: &FlatBranch, element: FlatVariable) -> Vec<FlatVariable> {
    let mut above = Vec::new();
    for atom in &branch.atoms {
        if let FlatAtom::Walk {
            direction,
            start: FlatTerm::Variable(start),
            reached: FlatTerm::Variable(reached),
            ..
        } = atom
        {
            match direction {
                AxisDirection::Ancestor if *start == element => above.push(*reached),
                AxisDirection::Descendant if *reached == element => above.push(*start),
                AxisDirection::Ancestor
                | AxisDirection::Descendant
                | AxisDirection::Parent
                | AxisDirection::NextSibling
                | AxisDirection::PreviousSibling => {}
            }
        }
    }
    above
}

/// The neighbourhoods a branch holds of its elements with a kind: for each, its own field, its parent's kind and field,
/// and its children's, where the branch holds any. None when no element with a kind has a neighbour held, or a field:
/// the kind alone is the free terms' already.
pub(crate) fn tree_shape(
    branch: &FlatBranch,
    pins: &HashMap<FlatVariable, FlatTerm>,
) -> Option<TreeShape> {
    let found = elements(branch, pins);
    let node = |element: &ShapeElement| ShapeNode {
        kind: element.kind,
        field: element.field,
        own: element.own,
    };
    let said = |element: &ShapeElement| element.kind.is_some() || element.field.is_some();
    // The stars in a fixed order of what they hold, never of the variables' numbers: the same branch written another way
    // gives the same shape.
    let mut stars = Vec::new();
    for (variable, element) in found.iter().filter(|(_, element)| element.kind.is_some()) {
        let parent = element
            .parent
            .and_then(|parent| found.get(&parent))
            .filter(|parent| said(parent))
            .map(node);
        let mut children: Vec<ShapeNode> = found
            .iter()
            .filter(|(child, held)| *child != variable && held.parent == Some(*variable) && said(held))
            .map(|(_, held)| node(held))
            .collect();
        children.sort_by_key(|child| format!("{child:?}"));
        children.dedup();
        let mut ancestors: Vec<CellWord> = walked_above(branch, *variable)
            .iter()
            .filter_map(|above| found.get(above).and_then(|held| held.kind))
            .collect();
        ancestors.sort_by_key(|word| word.0);
        ancestors.dedup();
        if parent.is_none() && children.is_empty() && ancestors.is_empty() && element.field.is_none() {
            continue;
        }
        stars.push(ShapeStar {
            anchor: node(element),
            parent,
            children,
            ancestors,
        });
    }
    stars.sort_by_key(|star| format!("{star:?}"));
    stars.dedup();
    (!stars.is_empty()).then_some(TreeShape { stars })
}
