//! Engine-owned grouping storage and text resolution; primitive reduction runs through the injected inbound capability.
use super::types::{CountRoom, GroupTarget, Hashes, ResolvedValues};
use crate::domain::operating::{
    errors::OperatingError, service::written_result, types::PrimitiveSemantics,
};
use ryo_derived::domain::key::types::On;
use ryo_operators::domain::reduction::types::DigestDatum;
use ryo_operators::domain::reduction::types::DigestValues;
use ryo_operators::domain::reduction::types::GroupColumns;
use ryo_operators::domain::reduction::types::HashScratch;
use ryo_operators::domain::table_texts::types::Texts;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_values::domain::{plan_parts::types::ColumnIndex, rows::types::Table};

/// Counts the rows of each group of a table through the operators.
///
/// # Errors
///
/// [`OperatingError::WrongOperation`] when the operators answer other than written, and the operators' fault mapped
/// into its [`OperatingError`].
pub fn count_groups(
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
/// Digests one column of each group of a table through the operators.
///
/// # Errors
///
/// [`OperatingError::WrongOperation`] when the operators answer other than written, and the operators' fault mapped
/// into its [`OperatingError`].
pub fn digest_groups(
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
