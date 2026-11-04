//! Deferred set rows through one portable meaning and an engine-scoped raw order.
use crate::domain::lowering::{service::shape_at, types::Lowering};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::{
    derived::ScopedDerived,
    failure::FailureLatch,
    types::{PrimitiveSemantics, written_result},
};
use crate::domain::positions::{
    service::{count, key_into, made_of, row, slot_of, value},
    types::{Keeps, KeyColumns, Keys, LoweredRoom, Positions, Sides, Slots, Stride},
};
use crate::domain::table_cells::types::TreeRows;
use crate::domain::table_model::service::positions;
use ryo_derived::domain::contract::types::RawOrder;
use ryo_operators::domain::contract::types::{
    OperatorCall, SetInput, SetInterrupted, SetKind, SetPositionsDriver,
};
use ryo_relations::domain::relations::{service::to_bits, types::Position};
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::rows::types::{RowCount, RowIndex};

struct UnionRows<'a> {
    slots: &'a mut Vec<u32>,
    keep: &'a Keeps,
    stride: Stride,
    next: RowIndex,
    current: Option<RowIndex>,
    written: RowIndex,
    failure: FailureLatch,
}
impl SetPositionsDriver for UnionRows<'_> {
    fn next(&mut self) -> Result<bool, SetInterrupted> {
        self.failure
            .run(|| {
                self.current = (self.next.0 < self.keep.0.len()).then_some(self.next);
                self.next.0 += usize::from(self.current.is_some());
                Ok(self.current.is_some())
            })
            .map_err(|()| SetInterrupted)
    }
    fn right_empty(&mut self) -> Result<bool, SetInterrupted> {
        self.failure
            .run(|| Err(OperatingError::WrongOperation))
            .map_err(|()| SetInterrupted)
    }
    fn contains_right(&mut self) -> Result<bool, SetInterrupted> {
        self.failure
            .run(|| Err(OperatingError::WrongOperation))
            .map_err(|()| SetInterrupted)
    }
    fn first_time(&mut self) -> Result<bool, SetInterrupted> {
        self.failure
            .run(|| {
                self.current
                    .and_then(|at| self.keep.0.get(at.0))
                    .copied()
                    .ok_or(OperatingError::ShapeMismatch)
            })
            .map_err(|()| SetInterrupted)
    }
    fn emit(&mut self) -> Result<(), SetInterrupted> {
        self.failure
            .run(|| {
                let at = self.current.ok_or(OperatingError::WrongOperation)?;
                self.slots.copy_within(
                    at.0 * self.stride.0..(at.0 + 1) * self.stride.0,
                    self.written.0 * self.stride.0,
                );
                self.written.0 += 1;
                Ok(())
            })
            .map_err(|()| SetInterrupted)
    }
}

/// Build the physical concatenation, then let the portable Union meaning choose its first rows.
pub(crate) fn union(
    semantics: &dyn PrimitiveSemantics,
    derived: &mut dyn ScopedDerived,
    tree: &TreeRows<'_>,
    lowering: &Lowering,
    id: StepId,
    sides: Sides,
    room: &mut LoweredRoom,
    out: &mut Slots,
) -> Result<(), OperatingError> {
    let shape = shape_at(lowering, id)?;
    let stride = Stride(shape.columns.len().max(1));
    let LoweredRoom {
        made, keys, keep, ..
    } = room;
    for side in [sides.left, sides.right] {
        let columns = shape_at(lowering, side)?;
        let rows = made_of(made, side).ok_or(OperatingError::NotRun)?;
        for at in positions(count(rows)) {
            let each = row(rows, at);
            out.0.extend(
                columns
                    .columns
                    .iter()
                    .map(|column| slot_of(each, *column).0),
            );
        }
    }
    keys.0.clear();
    for each in out.0.chunks(stride.0) {
        for (position, column) in each.iter().zip(&shape.columns) {
            keys.0
                .push(to_bits(value(tree, column.field, Position(*position))).0);
        }
    }
    let mut marks = std::mem::take(keep);
    let result = derived.with_raw_order(&keys.0, stride.0, &mut |order: &RawOrder| {
        order.firsts(&mut marks.0);
        let mut driver = UnionRows {
            slots: &mut out.0,
            keep: &marks,
            stride,
            next: RowIndex(0),
            current: None,
            written: RowIndex(0),
            failure: FailureLatch::default(),
        };
        let answer = semantics.operators(OperatorCall::SetRow {
            input: SetInput::Positions {
                kind: SetKind::Union,
                driver: &mut driver,
            },
        });
        let result = written_result(answer, driver.failure.take());
        if result.is_ok() {
            driver.slots.truncate(driver.written.0 * stride.0);
        }
        result
    });
    *keep = marks;
    result
}

struct AntijoinRows<'a, 't> {
    tree: &'a TreeRows<'t>,
    left: &'a Positions,
    columns: &'a [crate::domain::lowering::types::LoweredColumn],
    left_on: &'a [ryo_values::domain::plan_parts::types::ColumnIndex],
    order: &'a RawOrder,
    probe: &'a mut Keys,
    out: &'a mut Slots,
    right_count: RowCount,
    next: RowIndex,
    current: Option<RowIndex>,
    failure: FailureLatch,
}
impl SetPositionsDriver for AntijoinRows<'_, '_> {
    fn next(&mut self) -> Result<bool, SetInterrupted> {
        self.failure
            .run(|| {
                self.current = (self.next.0 < count(self.left).0).then_some(self.next);
                self.next.0 += usize::from(self.current.is_some());
                Ok(self.current.is_some())
            })
            .map_err(|()| SetInterrupted)
    }
    fn right_empty(&mut self) -> Result<bool, SetInterrupted> {
        self.failure
            .run(|| Ok(self.right_count.0 == 0))
            .map_err(|()| SetInterrupted)
    }
    fn contains_right(&mut self) -> Result<bool, SetInterrupted> {
        self.failure
            .run(|| {
                let current = self.current.ok_or(OperatingError::WrongOperation)?;
                self.probe.0.clear();
                key_into(
                    self.tree,
                    self.columns,
                    row(self.left, current),
                    self.left_on,
                    self.probe,
                );
                Ok(!self.order.hits(&self.probe.0).is_empty())
            })
            .map_err(|()| SetInterrupted)
    }
    fn first_time(&mut self) -> Result<bool, SetInterrupted> {
        self.failure
            .run(|| Err(OperatingError::WrongOperation))
            .map_err(|()| SetInterrupted)
    }
    fn emit(&mut self) -> Result<(), SetInterrupted> {
        self.failure
            .run(|| {
                let current = self.current.ok_or(OperatingError::WrongOperation)?;
                self.out.0.extend_from_slice(row(self.left, current).0);
                Ok(())
            })
            .map_err(|()| SetInterrupted)
    }
}

/// Borrow the right raw order for the whole deferred antijoin semantic call.
pub(crate) fn antijoin(
    semantics: &dyn PrimitiveSemantics,
    derived: &mut dyn ScopedDerived,
    tree: &TreeRows<'_>,
    lowering: &Lowering,
    sides: Sides,
    keys_of: KeyColumns<'_>,
    room: &mut LoweredRoom,
    out: &mut Slots,
) -> Result<(), OperatingError> {
    let left_shape = shape_at(lowering, sides.left)?;
    let right_shape = shape_at(lowering, sides.right)?;
    let LoweredRoom {
        made, keys, probe, ..
    } = room;
    let left = made_of(made, sides.left).ok_or(OperatingError::NotRun)?;
    let right = made_of(made, sides.right).ok_or(OperatingError::NotRun)?;
    let stride = Stride(keys_of.right.len());
    keys.0.clear();
    for at in positions(count(right)) {
        key_into(
            tree,
            &right_shape.columns,
            row(right, at),
            keys_of.right,
            keys,
        );
    }
    derived.with_raw_order(&keys.0, stride.0, &mut |order: &RawOrder| {
        let mut driver = AntijoinRows {
            tree,
            left,
            columns: &left_shape.columns,
            left_on: keys_of.left,
            order,
            probe,
            out,
            right_count: count(right),
            next: RowIndex(0),
            current: None,
            failure: FailureLatch::default(),
        };
        let answer = semantics.operators(OperatorCall::SetRow {
            input: SetInput::Positions {
                kind: SetKind::Antijoin,
                driver: &mut driver,
            },
        });
        written_result(answer, driver.failure.take())
    })
}
