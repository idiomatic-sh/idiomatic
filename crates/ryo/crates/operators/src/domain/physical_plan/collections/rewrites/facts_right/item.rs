//! Spike (ryo-fmx3): the `facts_right` item of the `rewrites` collection. A probe whose right rows are a select of a
//! relation's columns, with no comparison, reads the same rows, in the same order and with the same repeats, as the relation
//! itself with its key and right output columns read through the kept columns. So it probes the relation: the copy is made
//! only if another reader needs it, and the probes of one relation and key share one index of the file.

use super::super::interface::RewritesItem;
use crate::domain::equality_join::types::{ProbeKey, ProbeMode};
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::{KeyedIndex, NodeReaders, ProbeIndex, RowSource};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Side};

/// `facts_right`, one item of the `rewrites` collection.
pub(crate) struct FactsRightItem;

impl RewritesItem for FactsRightItem {
    fn rewrite(&self, nodes: &mut [PhysicalNode], _indexes: &mut Vec<KeyedIndex>) {
        if !crate::spike::lazy() || std::env::var_os("RYO_NO_FACTS_RIGHT").is_some() {
            return;
        }
        for at in 0..nodes.len() {
            let Some(Lowering::Probe {
                right: RowSource::Node(read),
                probe,
                index: ProbeIndex::Own,
                left,
            }) = nodes.get(at).map(|node| &node.lowering)
            else {
                continue;
            };
            if *left == RowSource::Node(*read)
                || matches!(probe.mode, ProbeMode::Ordered(_))
                || !probe.filter.comparisons.is_empty()
                || !probe.filter.kept.is_empty()
                || !probe.filter.bindings.is_empty()
            {
                continue;
            }
            let read_at = usize::try_from(read.0).unwrap_or(usize::MAX);
            let Some(Lowering::Filter {
                source: source @ RowSource::Facts(_),
                select,
            }) = nodes.get(read_at).map(|node| &node.lowering)
            else {
                continue;
            };
            if !select.comparisons.is_empty() || !select.bindings.is_empty() {
                continue;
            }
            let through = |column: ColumnIndex| -> Option<ColumnIndex> {
                if select.kept.is_empty() {
                    Some(column)
                } else {
                    select.kept.get(usize::from(column)).copied()
                }
            };
            let ProbeKey::Columns { left: left_on, right: right_on } = &probe.key else {
                continue;
            };
            let Some(right_on) = right_on.iter().map(|column| through(*column)).collect::<Option<Vec<_>>>() else {
                continue;
            };
            let mut output = probe.output.clone();
            if output
                .iter_mut()
                .filter(|column| column.side == Side::Right)
                .any(|column| match through(column.column) {
                    Some(kept) => {
                        column.column = kept;
                        false
                    }
                    None => true,
                })
            {
                continue;
            }
            let mut probe = probe.clone();
            probe.key = ProbeKey::Columns {
                left: left_on.clone(),
                right: right_on,
            };
            probe.output = output;
            let source = *source;
            if let Some(node) = nodes.get_mut(read_at) {
                node.readers = NodeReaders(node.readers.0.saturating_sub(1));
            }
            if let Some(Lowering::Probe {
                right,
                probe: parts,
                ..
            }) = nodes.get_mut(at).map(|node| &mut node.lowering)
            {
                *right = source;
                *parts = probe;
            }
        }
    }
}
