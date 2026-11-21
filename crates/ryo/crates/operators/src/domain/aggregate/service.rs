//! Format a group without taking ownership of reduction state or first-row acquisition.
use crate::domain::contract::types::PortableOperatorFault;
use ryo_relations::domain::relations::types::{Count, Integer, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
pub(crate) fn aggregate(
    row: &[Value],
    keys: &[ColumnIndex],
    count: Count,
    digest: Option<Integer>,
    target: &mut Vec<Value>,
) -> Result<(), PortableOperatorFault> {
    target.clear();
    group_cells(
        row,
        keys,
        Value::Number(Integer(i64::try_from(count.0).unwrap_or(i64::MAX))),
        digest.map(Value::Number),
        target,
    )
}

/// The one meaning of a counted or digested grouping, over cells of any form: appends to `target` one group's row, which is
/// the cells of `row` (any row of the group) at `keys`, in the keys' order, then the digest when one is given, then the
/// count. Today's `group` kernel calls it over typed values and the file-phase engine over its cell words.
///
/// # Errors
///
/// `NoSuchColumn` for the first key that names no cell of `row`; `target` then holds the keys before it.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::aggregate::service::group_cells;
/// use ryo_values::domain::plan_parts::types::ColumnIndex;
///
/// let mut row = Vec::new();
/// group_cells(&[7_u64, 9], &[ColumnIndex(1)], 3, None, &mut row)?;
/// assert_eq!(row, [9, 3]);
/// # Ok::<(), ryo_operators::domain::contract::types::PortableOperatorFault>(())
/// ```
pub fn group_cells<C: Copy>(
    row: &[C],
    keys: &[ColumnIndex],
    count: C,
    digest: Option<C>,
    target: &mut Vec<C>,
) -> Result<(), PortableOperatorFault> {
    crate::domain::project::service::project(row, keys, target)?;
    if let Some(digest) = digest {
        target.push(digest);
    }
    target.push(count);
    Ok(())
}

/// The one lowering of a grouping, counted or digested, for the `group` kernel.
pub(crate) fn lower(
    keys: &[ColumnIndex],
    measure: super::types::GroupMeasure,
) -> super::types::GroupParts {
    super::types::GroupParts {
        keys: keys.to_vec(),
        measure,
    }
}
