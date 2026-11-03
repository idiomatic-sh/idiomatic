//! Streaming group acquisition preserves row-validation and upstream-error precedence.
use crate::domain::operating::{
    errors::OperatingError,
    types::{GroupHash, GroupNumber, GroupValue},
};
use crate::domain::operator::contract::types::AggregationProgram;
use crate::domain::table_cells::types::RowIndex;
use crate::domain::table_dedup::{service::find_or_add, types::RowFound};
use crate::domain::table_groups::{
    service::hash_of,
    types::{Group, SetDigest},
};
use crate::domain::table_model::{
    errors::TableModelError,
    service::{cell, column_in, push_row},
    types::{Addition, RowCount, Table},
};
use crate::domain::table_texts::types::Texts;
use ryo_relations::domain::relations::types::{CellKind, Value};
use std::hash::Hasher as _;

pub(crate) struct Aggregation {
    program: AggregationProgram,
    keys: Table,
    first_rows: Table,
    seen: hashbrown::HashTable<RowIndex>,
    counts: Vec<usize>,
    pairs: hashbrown::HashSet<GroupValue>,
    hashes: Vec<GroupHash>,
    probe: Vec<Value>,
}
impl Aggregation {
    /// Validate metadata before requesting the stream, in the original per-class order.
    pub(crate) fn new(
        program: &AggregationProgram,
        input_kinds: &[CellKind],
    ) -> Result<Self, OperatingError> {
        let (keys, over, digest) = match program {
            AggregationProgram::Count { keys, distinct } => (keys, *distinct, false),
            AggregationProgram::Digest { keys, over } => (keys, Some(*over), true),
        };
        let check_over = || -> Result<(), OperatingError> {
            if let Some(column) = over
                && usize::try_from(column.0).map_or(true, |at| at >= input_kinds.len())
            {
                return Err(TableModelError::NoSuchColumn { index: column.0 }.into());
            }
            Ok(())
        };
        if digest {
            check_over()?;
        }
        let kinds = keys
            .iter()
            .map(|column| {
                column_in(input_kinds, *column)
                    .copied()
                    .ok_or(TableModelError::NoSuchColumn { index: column.0 })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if !digest {
            check_over()?;
        }
        Ok(Self {
            program: program.clone(),
            keys: Table {
                kinds,
                ..Table::default()
            },
            first_rows: Table {
                kinds: input_kinds.to_vec(),
                ..Table::default()
            },
            seen: hashbrown::HashTable::new(),
            counts: Vec::new(),
            pairs: hashbrown::HashSet::new(),
            hashes: Vec::new(),
            probe: Vec::new(),
        })
    }
    /// Read and hash this row now; a later upstream error cannot hide a bad earlier row.
    pub(crate) fn push(&mut self, row: &[Value], texts: &Texts<'_>) -> Result<(), OperatingError> {
        let columns = match &self.program {
            AggregationProgram::Count { keys, .. } | AggregationProgram::Digest { keys, .. } => {
                keys
            }
        };
        self.probe.clear();
        for column in columns {
            self.probe.push(cell(row, *column)?);
        }
        let RowFound { at: group, added } =
            find_or_add(&mut self.keys, &mut self.seen, &self.probe);
        if added == Addition::Added {
            push_row(&mut self.first_rows, row);
            self.counts.push(0);
        }
        let group_number = GroupNumber(u32::try_from(group.0).unwrap_or(u32::MAX));
        match &self.program {
            AggregationProgram::Count { distinct, .. } => {
                let counted = match distinct {
                    None => true,
                    Some(column) => self.pairs.insert(GroupValue {
                        group: group_number,
                        value: cell(row, *column)?,
                    }),
                };
                if counted {
                    *self
                        .counts
                        .get_mut(group.0)
                        .ok_or(OperatingError::ShapeMismatch)? += 1;
                }
            }
            AggregationProgram::Digest { over, .. } => self.hashes.push(GroupHash {
                group: group_number,
                hash: hash_of(cell(row, *over)?, texts),
            }),
        }
        Ok(())
    }
    /// Retain only original first rows and descriptors; the primitive performs projection/emission.
    pub(crate) fn finish(mut self) -> (Table, Vec<Group>) {
        let mut groups = Vec::with_capacity(self.keys.count.0);
        match self.program {
            AggregationProgram::Count { .. } => {
                for (group, counted) in self.counts.into_iter().enumerate() {
                    groups.push(Group {
                        first: RowIndex(group),
                        counted: RowCount(counted),
                        digest: SetDigest(0),
                    });
                }
            }
            AggregationProgram::Digest { .. } => {
                self.hashes.sort_unstable();
                self.hashes.dedup();
                let mut at = 0;
                for group in 0..self.keys.count.0 {
                    let mut hasher = rustc_hash::FxHasher::default();
                    let mut counted = 0;
                    while let Some(each) = self.hashes.get(at)
                        && usize::try_from(each.group.0).unwrap_or(usize::MAX) == group
                    {
                        hasher.write_u64(each.hash.0);
                        counted += 1;
                        at += 1;
                    }
                    groups.push(Group {
                        first: RowIndex(group),
                        counted: RowCount(counted),
                        digest: SetDigest(hasher.finish().cast_signed()),
                    });
                }
            }
        }
        (self.first_rows, groups)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::table_texts::types::SourceView;
    use ryo_language::domain::{symbols::types::Symbols};
use ryo_relations::domain::relations::types::Integer;
    use ryo_values::domain::plan_parts::types::ColumnIndex;
    fn n(value: i64) -> Value {
        Value::Number(Integer(value))
    }
    #[test]
    fn representative_rows_keep_original_columns_and_distinct_counts() {
        let program = AggregationProgram::Count {
            keys: vec![ColumnIndex(1)],
            distinct: Some(ColumnIndex(0)),
        };
        let mut group = Aggregation::new(&program, &[CellKind::Number; 2]).unwrap();
        let symbols = Symbols::default();
        let texts = Texts {
            symbols: &symbols,
            source: SourceView(&[]),
            paths: &[],
            arenas: &[],
        };
        for row in [[n(9), n(2)], [n(9), n(2)], [n(8), n(1)], [n(7), n(2)]] {
            group.push(&row, &texts).unwrap();
        }
        let (first, groups) = group.finish();
        assert_eq!(first.cells, vec![n(9), n(2), n(8), n(1)]);
        assert_eq!(
            groups
                .iter()
                .map(|group| (group.first.0, group.counted.0))
                .collect::<Vec<_>>(),
            vec![(0, 2), (1, 1)]
        );
    }
    #[test]
    fn bad_early_row_fails_before_caller_requests_later_upstream_work() {
        let program = AggregationProgram::Digest {
            keys: vec![ColumnIndex(0)],
            over: ColumnIndex(1),
        };
        let mut group = Aggregation::new(&program, &[CellKind::Number; 2]).unwrap();
        let symbols = Symbols::default();
        let texts = Texts {
            symbols: &symbols,
            source: SourceView(&[]),
            paths: &[],
            arenas: &[],
        };
        assert!(group.push(&[n(1)], &texts).is_err());
    }
    #[test]
    fn count_and_digest_preserve_their_different_static_error_order() {
        let count = AggregationProgram::Count {
            keys: vec![ColumnIndex(99)],
            distinct: Some(ColumnIndex(98)),
        };
        let digest = AggregationProgram::Digest {
            keys: vec![ColumnIndex(99)],
            over: ColumnIndex(98),
        };
        assert!(matches!(
            Aggregation::new(&count, &[CellKind::Number]),
            Err(OperatingError::Table(TableModelError::NoSuchColumn {
                index: 99
            }))
        ));
        assert!(matches!(
            Aggregation::new(&digest, &[CellKind::Number]),
            Err(OperatingError::Table(TableModelError::NoSuchColumn {
                index: 98
            }))
        ));
    }
}
