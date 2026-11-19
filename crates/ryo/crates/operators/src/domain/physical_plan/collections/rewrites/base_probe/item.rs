//! Spike (ryo-fmx3): the `base_probe` item of the `rewrites` collection: a probe whose right rows are a pure projection of a
//! relation (a select with no comparison, or a projection keeping every row) probes the relation itself, its right columns
//! renamed through the projection, so the file's base rows are indexed in place and never copied into the projection's
//! table first. The relation holds the projection's rows in the same order with the same repeats, so the probe answers the
//! same rows.

use super::super::interface::RewritesItem;
use crate::domain::equality_join::types::{ProbeKey, ProbeMode};
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::{KeyedIndex, NodeReaders, ProbeIndex, RowSource};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Operand, RepeatedRows, Side};

/// `base_probe`, one item of the `rewrites` collection.
pub(crate) struct BaseProbeItem;

impl RewritesItem for BaseProbeItem {
    fn rewrite(&self, nodes: &mut [PhysicalNode], _indexes: &mut Vec<KeyedIndex>) {
        for at in 0..nodes.len() {
            let Lowering::Probe {
                right: RowSource::Node(read),
                ..
            } = &nodes[at].lowering
            else {
                continue;
            };
            let read = usize::try_from(read.0).unwrap_or(usize::MAX);
            let Some((relation, columns)) = nodes.get(read).and_then(|node| pure(&node.lowering)) else {
                continue;
            };
            let map = |column: ColumnIndex| -> Option<ColumnIndex> {
                if columns.is_empty() {
                    Some(column)
                } else {
                    columns.get(usize::try_from(column.0).ok()?).copied()
                }
            };
            let Lowering::Probe {
                right,
                probe,
                index,
                ..
            } = &mut nodes[at].lowering
            else {
                continue;
            };
            if !matches!(index, ProbeIndex::Own)
                || !matches!(probe.mode, ProbeMode::Inner | ProbeMode::Semi | ProbeMode::Anti)
                || !probe.filter.kept.is_empty()
                || !probe.filter.bindings.is_empty()
            {
                continue;
            }
            let ProbeKey::Columns { right: keys, .. } = &probe.key else {
                continue;
            };
            let Some(keys) = keys.iter().map(|column| map(*column)).collect::<Option<Vec<_>>>() else {
                continue;
            };
            let mut comparisons = probe.filter.comparisons.clone();
            let mut mapped = true;
            for comparison in &mut comparisons {
                for operand in [&mut comparison.left, &mut comparison.right] {
                    match operand {
                        Operand::Column(column) => match map(*column) {
                            Some(to) => *column = to,
                            None => mapped = false,
                        },
                        Operand::Function(_) => mapped = false,
                        Operand::Parameter(_) | Operand::Literal(_) => {}
                    }
                }
            }
            let mut output = probe.output.clone();
            for column in &mut output {
                if column.side == Side::Right {
                    match map(column.column) {
                        Some(to) => column.column = to,
                        None => mapped = false,
                    }
                }
            }
            if !mapped {
                continue;
            }
            if let ProbeKey::Columns { right: old, .. } = &mut probe.key {
                *old = keys;
            }
            probe.filter.comparisons = comparisons;
            probe.output = output;
            *right = RowSource::Facts(relation);
            if let Some(projection) = nodes.get_mut(read) {
                projection.readers = NodeReaders(projection.readers.0.saturating_sub(1));
            }
        }
    }
}

/// The relation a pure projection reads and the columns it keeps (none: every column), when `lowering` is one.
fn pure(lowering: &Lowering) -> Option<(ryo_relations::domain::relations::types::Relation, Vec<ColumnIndex>)> {
    match lowering {
        Lowering::Filter {
            source: RowSource::Facts(relation),
            select,
        } if select.comparisons.is_empty() && select.bindings.is_empty() && entity_free(*relation) => {
            Some((*relation, select.kept.clone()))
        }
        Lowering::Project {
            source: RowSource::Facts(relation),
            project,
        } if project.distinct == RepeatedRows::Kept && !project.columns.is_empty() && entity_free(*relation) => {
            Some((*relation, project.columns.clone()))
        }
        _ => None,
    }
}

/// Whether the entity sub-engine leaves `relation` alone: its two relations stay as planned, so its stages do not change.
fn entity_free(relation: ryo_relations::domain::relations::types::Relation) -> bool {
    use ryo_relations::domain::relations::types::Relation;
    !matches!(relation, Relation::SemanticSyntax | Relation::SemanticChild)
}
