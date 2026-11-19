//! Ordered positions shared by consumers of the same immutable population and field.
use super::super::entity_columns::value::{EntityFileColumns, EntityPositions};
use super::super::{EntityColumn, EntityLiteralCode, EntityPosition};
use crate::domain::row_predicates::types::Program;
use rustc_hash::FxHashMap;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::rows::types::RowCount;

/// Up to two MiB of resident entity-position payload across all of this file's indexes.
const INDEX_POSITIONS: RowCount =
    RowCount((2 * 1024 * 1024) / std::mem::size_of::<EntityPosition>());

#[derive(Debug, Default)]
enum SubscribedEntityRows {
    #[default]
    Unread,
    Scanned,
    Retained(FxHashMap<EntityLiteralCode, EntityPositions>),
    Full,
}

#[derive(Debug, Default)]
pub(in super::super::super) struct EntitySubscriptions {
    indexes: Vec<SubscribedEntityRows>,
    held: RowCount,
}

impl EntitySubscriptions {
    pub(in super::super::super) fn selected(
        &mut self,
        program: &Program,
        step: StepId,
        source: &EntityPositions,
        fields: &[EntityColumn],
        graph: &EntityFileColumns,
    ) -> Option<EntityPositions> {
        let subscription = program.subscriptions[usize::from(step)]?;
        if self.indexes.is_empty() {
            self.indexes
                .resize_with(program.partitions.len(), SubscribedEntityRows::default);
        }
        let slot = &mut self.indexes[subscription.partition.0];
        match slot {
            SubscribedEntityRows::Unread => {
                *slot = SubscribedEntityRows::Scanned;
                return None;
            }
            SubscribedEntityRows::Scanned => {
                if source.len() > INDEX_POSITIONS.0.saturating_sub(self.held.0) {
                    *slot = SubscribedEntityRows::Full;
                    return None;
                }
                let partition = &program.partitions[subscription.partition.0];
                let mut selected: FxHashMap<_, Vec<_>> = partition
                    .literals
                    .iter()
                    .filter_map(|literal| match u32::try_from(literal.0) {
                        Ok(raw) => Some(EntityLiteralCode(raw)),
                        Err(_) => None,
                    })
                    .map(|code| (code, Vec::new()))
                    .collect();
                let field = graph.field(fields[usize::from(partition.column)]);
                for &position in source.iter() {
                    if let Some(code) = graph.code(field.get(position))
                        && let Some(rows) = selected.get_mut(&code)
                    {
                        rows.push(position);
                    }
                }
                self.held.0 += selected.values().map(Vec::len).sum::<usize>();
                *slot = SubscribedEntityRows::Retained(
                    selected
                        .into_iter()
                        .map(|(code, rows)| (code, rows.into()))
                        .collect(),
                );
            }
            SubscribedEntityRows::Retained(_) => {}
            SubscribedEntityRows::Full => return None,
        }
        let SubscribedEntityRows::Retained(buckets) = slot else {
            return None;
        };
        let code = match u32::try_from(subscription.literal.0) {
            Ok(raw) => EntityLiteralCode(raw),
            Err(_) => return None,
        };
        buckets.get(&code).cloned()
    }
}
