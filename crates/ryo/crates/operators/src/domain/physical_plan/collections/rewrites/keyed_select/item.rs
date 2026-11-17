//! The `keyed_select` item of the `rewrites` collection: the filters of one relation keyed on one of its name columns become
//! lookups of one shared `hash_index` of it when two or more of them key on it; a single one keeps its scan, which costs less
//! than an index.

use super::super::interface::RewritesItem;
use crate::domain::physical_plan::service::indexes::service::read_by;
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::{
    IndexIdentity, IndexKey, IndexPlace, KeyedIndex, KeyedIndexes, PlanNodeId, RowSource,
};
use ryo_language::domain::symbols::service::hash_of;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::service::fixed_prefix;
use ryo_values::domain::plan_parts::types::{
    Operand, PlanComparison, QueryLiteral, RowWidth, Test,
};

/// `keyed_select`, one item of the `rewrites` collection.
pub(crate) struct KeyedSelectItem;

impl RewritesItem for KeyedSelectItem {
    fn rewrite(&self, nodes: &mut [PhysicalNode], indexes: &mut Vec<KeyedIndex>) {
        keyed_selects(nodes, indexes);
    }
}

/// The key a select fixes in its infallible prefix: a name column of a relation equal to a written text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SelectKey {
    /// The relation.
    relation: ryo_relations::domain::relations::types::Relation,
    /// The name column.
    column: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The hash of the written text.
    key: ryo_relations::domain::relations::types::TextHash,
}

/// The key a filter of a relation fixes in its infallible prefix: a name column equal to a written, non-empty text. A key
/// after the prefix is no key: a comparison before it may fail, and the lookup would skip the rows it fails on.
fn key_of(source: RowSource, comparisons: &[PlanComparison]) -> Option<SelectKey> {
    let RowSource::Facts(relation) = source else {
        return None;
    };
    let columns = kinds(relation);
    fixed_prefix(comparisons, RowWidth(columns.len()))
        .0
        .iter()
        .find_map(|comparison| {
            let ((Operand::Column(column), Operand::Literal(QueryLiteral::Text(text)))
            | (Operand::Literal(QueryLiteral::Text(text)), Operand::Column(column))) =
                (&comparison.left, &comparison.right)
            else {
                return None;
            };
            let named = columns
                .get(usize::try_from(column.0).unwrap_or(usize::MAX))
                .is_some_and(|kind| *kind == CellKind::Symbol);
            (comparison.test == Test::Equal && named && !text.0.is_empty()).then(|| SelectKey {
                relation,
                column: *column,
                key: hash_of(ByteSlice(text.0.as_bytes())),
            })
        })
}

/// The `keyed_select` rewrite: the filters of one relation keyed on one of its columns become lookups of one shared index of
/// it when two or more of them key on it; a single one keeps its scan, which costs less than an index.
fn keyed_selects(nodes: &mut [PhysicalNode], indexes: &mut Vec<KeyedIndex>) {
    let mut keyed = KeyedIndexes::default();
    let mut keys: Vec<Vec<SelectKey>> = Vec::new();
    for (node, id) in nodes.iter().zip((0..).map(PlanNodeId)) {
        let Lowering::Filter { source, select } = &node.lowering else {
            continue;
        };
        let Some(key) = key_of(*source, &select.comparisons) else {
            continue;
        };
        let at = read_by(
            &mut keyed,
            IndexIdentity {
                source: RowSource::Facts(key.relation),
                key: IndexKey::Text(key.column),
            },
            id,
        );
        if keys.len() <= at.0 {
            keys.resize_with(at.0 + 1, Vec::new);
        }
        if let Some(held) = keys.get_mut(at.0) {
            held.push(key);
        }
    }
    let keyed = keyed.indexes;
    for (index, held) in keyed.into_iter().zip(keys) {
        if index.readers.len() < 2 {
            continue;
        }
        let shared = IndexPlace(indexes.len());
        for (lookup, key) in index.readers.iter().zip(&held) {
            if let Some(node) = nodes.get_mut(usize::try_from(lookup.0).unwrap_or(usize::MAX))
                && let Lowering::Filter { select, .. } =
                    std::mem::replace(&mut node.lowering, Lowering::Unlowered)
            {
                node.lowering = Lowering::Lookup {
                    index: shared,
                    key: key.key,
                    select,
                };
            }
        }
        indexes.push(index);
    }
}
