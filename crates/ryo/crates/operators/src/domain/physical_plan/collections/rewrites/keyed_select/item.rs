//! The `keyed_select` item of the `rewrites` collection: the filters of one relation keyed on one of its name columns become
//! lookups of one shared `hash_index` of it when two or more of them key on it; a single one keeps its scan, which costs less
//! than an index.

use super::super::interface::RewritesItem;
use crate::domain::comparing::service::fixed_prefix;
use crate::domain::physical_plan::types::{
    IndexKey, IndexPlace, KeyedIndex, Lowering, PhysicalNode, PlanNodeId, RowSource,
};
use ryo_language::domain::symbols::service::hash_of;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::{Operand, PlanComparison, QueryLiteral, Test};

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
    fixed_prefix(comparisons, columns)
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
    let mut keyed: Vec<KeyedIndex> = Vec::new();
    let mut keys: Vec<Vec<SelectKey>> = Vec::new();
    for (node, id) in nodes.iter().zip((0..).map(PlanNodeId)) {
        let Lowering::Filter { source, select } = &node.lowering else {
            continue;
        };
        let Some(key) = key_of(*source, &select.comparisons) else {
            continue;
        };
        let indexed = (RowSource::Facts(key.relation), IndexKey::Text(key.column));
        match keyed
            .iter()
            .position(|index| (index.source, &index.key) == (indexed.0, &indexed.1))
        {
            Some(at) => {
                if let (Some(index), Some(held)) = (keyed.get_mut(at), keys.get_mut(at)) {
                    index.readers.push(id);
                    held.push(key);
                }
            }
            None => {
                keyed.push(KeyedIndex {
                    source: indexed.0,
                    key: indexed.1,
                    readers: vec![id],
                });
                keys.push(vec![key]);
            }
        }
    }
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
