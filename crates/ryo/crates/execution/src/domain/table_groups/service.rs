//! Engine-owned grouping storage and text resolution; primitive reduction runs through the injected inbound capability.
use super::types::{CountRoom, Group, Hashes, SetDigest};
use crate::domain::{
    operating::errors::OperatingError,
    operator::contract::types::{PrimitiveSemantics, written_result},
    table_texts::{
        service::datum,
        types::{Datum, Texts},
    },
};
use ryo_derived::domain::contract::types::On;
use ryo_operators::domain::reduction::types::DigestDatum;
use ryo_operators::domain::reduction::types::DigestValues;
use ryo_operators::domain::reduction::types::GroupColumns;
use ryo_operators::domain::reduction::types::{GroupDigest, GroupOutput, HashScratch};
use ryo_values::domain::row_texts::types::CellText;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::{
    plan_parts::types::ColumnIndex,
    rows::types::{RowCount, RowIndex, Table},
};

pub(crate) struct GroupTarget<'a>(pub &'a mut Vec<Group>);
impl GroupOutput for GroupTarget<'_> {
    fn clear(&mut self) {
        self.0.clear();
    }
    fn push(&mut self, first: RowIndex, counted: RowCount, digest: GroupDigest) {
        self.0.push(Group {
            first,
            counted,
            digest: SetDigest(digest.0),
        });
    }
    fn sort_first(&mut self) {
        self.0.sort_unstable_by_key(|group| group.first);
    }
}

pub(crate) fn count_groups(
    table: &Table,
    on: On<'_>,
    distinct: Option<ColumnIndex>,
    room: &mut CountRoom<'_>,
    operators: &dyn PrimitiveSemantics,
) -> Result<(), OperatingError> {
    let answer = operators.operators(OperatorCall::ReduceCount {
        table,
        keys: match on {
            On::Whole => GroupColumns::Whole,
            On::Columns(keys) => GroupColumns::Columns(keys),
        },
        distinct,
        order: room.order,
        values: room.values,
        target: &mut GroupTarget(room.groups),
    });
    written_result(answer, None)
}
pub(crate) fn resolved<'a>(value: Value, texts: &'a Texts<'_>) -> DigestDatum<'a> {
    match datum(value, texts) {
        Datum::Unlocated => DigestDatum::Unlocated,
        Datum::Text(text) => DigestDatum::Text(CellText(text.0)),
        Datum::Number(number) => DigestDatum::Number(number),
        Datum::Flag(flag) => DigestDatum::Flag(flag),
        Datum::Numbers(numbers) => DigestDatum::Numbers(numbers),
        Datum::Texts(items) => DigestDatum::Texts(items),
    }
}
struct ResolvedValues<'a, 'text>(&'a Texts<'text>);
impl DigestValues for ResolvedValues<'_, '_> {
    fn datum(&self, value: Value) -> DigestDatum<'_> {
        resolved(value, self.0)
    }
}
pub(crate) fn digest_groups(
    table: &Table,
    keys: &[ColumnIndex],
    over: ColumnIndex,
    texts: &Texts<'_>,
    room: &mut CountRoom<'_>,
    hashes: &mut Hashes,
    operators: &dyn PrimitiveSemantics,
) -> Result<(), OperatingError> {
    let hasher = ResolvedValues(texts);
    let answer = operators.operators(OperatorCall::ReduceDigest {
        table,
        keys,
        over,
        order: room.order,
        hashes: HashScratch(&mut hashes.0),
        hasher: &hasher,
        target: &mut GroupTarget(room.groups),
    });
    written_result(answer, None)
}
