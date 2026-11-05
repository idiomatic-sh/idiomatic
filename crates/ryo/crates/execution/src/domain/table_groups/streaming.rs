//! Invocation-owned reduction storage. Every reduction decision runs through the injected semantic contract.
use crate::domain::operating::{
    errors::OperatingError,
    types::{GroupNumber, GroupValue},
};
use crate::domain::operator::contract::types::{AggregationProgram, written_result};
use crate::domain::table_dedup::{service::find_or_add, types::RowFound};
use crate::domain::table_groups::{
    service::{GroupTarget, plan, resolved},
    types::Group,
};
use crate::domain::table_model::{service::push_row, types::Addition};
use crate::domain::table_texts::types::Texts;
use crate::outbound::operators::interface::OperatorsPort;
use ryo_operators::inbound::use_cases::operators::interface::{
    AggregationCheckOrder, AggregationHash, DigestDatum, OperatorCall, SemanticInterrupted,
    StreamingReduction,
};
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::rows::types::{RowIndex, Table};

pub(crate) struct Aggregation {
    program: AggregationProgram,
    keys: Table,
    first_rows: Table,
    seen: hashbrown::HashTable<RowIndex>,
    counts: Vec<usize>,
    pairs: hashbrown::HashSet<GroupValue>,
    hashes: Vec<AggregationHash>,
    probe: Vec<Value>,
}
struct Storage<'a, 'text> {
    state: &'a mut Aggregation,
    texts: &'a Texts<'text>,
}
impl StreamingReduction for Storage<'_, '_> {
    fn group(&mut self, key: &[Value], row: &[Value]) -> Result<RowIndex, SemanticInterrupted> {
        let RowFound { at, added } = find_or_add(&mut self.state.keys, &mut self.state.seen, key);
        if added == Addition::Added {
            push_row(&mut self.state.first_rows, row);
            self.state.counts.push(0);
        }
        Ok(at)
    }
    fn insert_value(&mut self, group: RowIndex, value: Value) -> bool {
        self.state.pairs.insert(GroupValue {
            group: GroupNumber(u32::try_from(group.0).unwrap_or(u32::MAX)),
            value,
        })
    }
    fn count(&mut self, group: RowIndex) -> Option<&mut usize> {
        self.state.counts.get_mut(group.0)
    }
    fn datum(&self, value: Value) -> DigestDatum<'_> {
        resolved(value, self.texts)
    }
    fn append_hash(&mut self, hash: AggregationHash) {
        self.state.hashes.push(hash);
    }
}
impl Aggregation {
    pub(crate) fn new(
        program: &AggregationProgram,
        input_kinds: &[CellKind],
        operators: &dyn OperatorsPort,
    ) -> Result<Self, OperatingError> {
        let mut kinds = Vec::new();
        let answer = operators.operators(OperatorCall::CheckAggregation {
            program: plan(program),
            kinds: input_kinds,
            order: AggregationCheckOrder::Streaming,
            target: &mut kinds,
        });
        written_result(answer, None)?;
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
    pub(crate) fn push(
        &mut self,
        row: &[Value],
        texts: &Texts<'_>,
        operators: &dyn OperatorsPort,
    ) -> Result<(), OperatingError> {
        // Move borrowed invocation scratch out temporarily; no metadata or rows are copied.
        let program = std::mem::replace(
            &mut self.program,
            AggregationProgram::Count {
                keys: Vec::new(),
                distinct: None,
            },
        );
        let mut key = std::mem::take(&mut self.probe);
        let mut driver = Storage { state: self, texts };
        let answer = operators.operators(OperatorCall::StreamAggregation {
            program: plan(&program),
            row,
            key: &mut key,
            driver: &mut driver,
        });
        let result = written_result(answer, None);
        driver.state.program = program;
        driver.state.probe = key;
        result
    }
    pub(crate) fn finish(
        mut self,
        operators: &dyn OperatorsPort,
    ) -> Result<(Table, Vec<Group>), OperatingError> {
        let mut groups = Vec::with_capacity(self.keys.count.0);
        let answer = operators.operators(OperatorCall::FinishAggregation {
            counts: &self.counts,
            hashes: &mut self.hashes,
            digest: matches!(self.program, AggregationProgram::Digest { .. }),
            target: &mut GroupTarget(&mut groups),
        });
        written_result(answer, None)?;
        Ok((self.first_rows, groups))
    }
}

#[cfg(test)]
mod tests {
    use super::super::service_test::OPERATORS;
    use super::*;
    use ryo_language::domain::symbols::types::Symbols;
    use ryo_relations::domain::relations::types::Integer;
    use ryo_values::domain::files::types::SourceView;
    use ryo_values::domain::plan_parts::types::ColumnIndex;
    use ryo_values::domain::rows::errors::TableModelError;
    fn n(value: i64) -> Value {
        Value::Number(Integer(value))
    }
    #[test]
    fn representative_rows_keep_original_columns_and_distinct_counts() {
        let program = AggregationProgram::Count {
            keys: vec![ColumnIndex(1)],
            distinct: Some(ColumnIndex(0)),
        };
        let mut group = Aggregation::new(&program, &[CellKind::Number; 2], &OPERATORS).unwrap();
        let symbols = Symbols::default();
        let texts = Texts {
            symbols: &symbols,
            source: SourceView(&[]),
            paths: &[],
            arenas: &[],
        };
        for row in [[n(9), n(2)], [n(9), n(2)], [n(8), n(1)], [n(7), n(2)]] {
            group.push(&row, &texts, &OPERATORS).unwrap();
        }
        let (first, groups) = group.finish(&OPERATORS).unwrap();
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
        let mut group = Aggregation::new(&program, &[CellKind::Number; 2], &OPERATORS).unwrap();
        let symbols = Symbols::default();
        let texts = Texts {
            symbols: &symbols,
            source: SourceView(&[]),
            paths: &[],
            arenas: &[],
        };
        assert!(group.push(&[n(1)], &texts, &OPERATORS).is_err());
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
            Aggregation::new(&count, &[CellKind::Number], &OPERATORS),
            Err(OperatingError::Table(TableModelError::NoSuchColumn {
                index: 99
            }))
        ));
        assert!(matches!(
            Aggregation::new(&digest, &[CellKind::Number], &OPERATORS),
            Err(OperatingError::Table(TableModelError::NoSuchColumn {
                index: 98
            }))
        ));
    }
}

use ryo_values::domain::rows::types::RowCount;
