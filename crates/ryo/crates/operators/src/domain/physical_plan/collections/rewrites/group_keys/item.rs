//! Spike (ryo-fmx3): the `group_keys` item of the `rewrites` collection. A group answers one row per group, its keys from the
//! group's first row, in the order of its first row, whatever its measure; so a distinct projection of only its key
//! columns is the distinct projection of those key columns of its source, and the group, read by nothing else, never runs.

use super::super::interface::RewritesItem;
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::{EngineUses, KeyedIndex, NodeReaders, RowSource};
use ryo_values::domain::plan_parts::types::RepeatedRows;

/// `group_keys`, one item of the `rewrites` collection.
pub(crate) struct GroupKeysItem;

impl RewritesItem for GroupKeysItem {
    fn rewrite(&self, nodes: &mut [PhysicalNode], _indexes: &mut Vec<KeyedIndex>) {
        for at in 0..nodes.len() {
            let Some(PhysicalNode {
                lowering: Lowering::Project { source: RowSource::Node(group), project },
                ..
            }) = nodes.get(at)
            else {
                continue;
            };
            if project.distinct != RepeatedRows::Dropped {
                continue;
            }
            let group_at = usize::try_from(group.0).unwrap_or(usize::MAX);
            let Some(PhysicalNode {
                lowering: Lowering::Group { source, group: parts },
                readers,
                engine,
                ..
            }) = nodes.get(group_at)
            else {
                continue;
            };
            if *readers != NodeReaders(1) || *engine != EngineUses(0) {
                continue;
            }
            let Some(columns) = project
                .columns
                .iter()
                .map(|column| parts.keys.get(usize::from(*column)).copied())
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            let source = *source;
            if let Some(node) = nodes.get_mut(group_at) {
                node.readers = NodeReaders(0);
            }
            if let Some(PhysicalNode {
                lowering: Lowering::Project { source: read, project },
                ..
            }) = nodes.get_mut(at)
            {
                *read = source;
                project.columns = columns;
            }
        }
    }
}
