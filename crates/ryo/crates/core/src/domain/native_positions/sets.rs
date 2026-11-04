//! Set operations on borrowed position rows and an engine-scoped raw order.

use super::types::{PositionInput, PositionWork};
use crate::domain::{
    lowering::types::LoweredShape,
    operating::errors::OperatingError,
    operator::contract::{
        derived::ScopedDerived,
        failure::FailureLatch,
        types::{PrimitiveSemantics, WorkCounters, written_result},
    },
    positions::{
        service::{count, key_into, row, slot_of, value},
        types::{Keeps, Keys, Positions, Slots, Stride},
    },
    table_cells::types::TreeRows,
    table_model::service::positions,
};
use ryo_derived::domain::contract::types::RawOrder;
use ryo_operators::domain::contract::types::{
    OperatorCall, SetInput, SetInterrupted, SetKind, SetPositionsDriver,
};
use ryo_relations::domain::relations::{service::to_bits, types::Position};
use ryo_values::domain::{
    plans::types::Operation,
    rows::types::{RowCount, RowIndex},
};

struct UnionDriver<'a> {
    slots: &'a mut Slots,
    keep: &'a Keeps,
    stride: Stride,
    next: RowIndex,
    current: Option<RowIndex>,
    written: RowIndex,
    failure: FailureLatch,
}
impl SetPositionsDriver for UnionDriver<'_> {
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
                self.slots.0.copy_within(
                    at.0 * self.stride.0..(at.0 + 1) * self.stride.0,
                    self.written.0 * self.stride.0,
                );
                self.written.0 += 1;
                Ok(())
            })
            .map_err(|()| SetInterrupted)
    }
}

pub(super) fn union(
    semantics: &dyn PrimitiveSemantics,
    derived: &mut dyn ScopedDerived,
    tree: &TreeRows<'_>,
    inputs: &[PositionInput<'_>],
    output: &LoweredShape,
    work: &mut PositionWork,
    counts: &WorkCounters,
) -> Result<Positions, OperatingError> {
    let left = inputs
        .first()
        .copied()
        .ok_or(OperatingError::MissingInput { position: 1 })?;
    let right = inputs
        .get(1)
        .copied()
        .ok_or(OperatingError::MissingInput { position: 2 })?;
    let stride = output.columns.len().max(1);
    let mut slots = Slots::default();
    for (source_at, source) in [left, right].into_iter().enumerate() {
        if source.shape.columns.len() != output.columns.len() {
            return Err(OperatingError::ShapeMismatch);
        }
        for at in positions(count(source.rows)) {
            counts.found(source_at, 1);
            let each = row(source.rows, at);
            if output.columns.is_empty() {
                slots.0.push(
                    each.0
                        .first()
                        .copied()
                        .ok_or(OperatingError::ShapeMismatch)?,
                );
            } else {
                slots.0.extend(
                    source
                        .shape
                        .columns
                        .iter()
                        .map(|column| slot_of(each, *column).0),
                );
            }
        }
    }
    let PositionWork { keys, kept, .. } = work;
    keys.0.clear();
    if output.columns.is_empty() {
        // Every visible key is empty; one physical marker per row preserves cardinality
        // while the portable SetRow meaning chooses the first distinct empty row.
        keys.0.resize(slots.0.len(), 0);
    } else {
        for each in slots.0.chunks(stride) {
            for (position, column) in each.iter().zip(&output.columns) {
                keys.0
                    .push(to_bits(value(tree, column.field, Position(*position))).0);
            }
        }
    }
    derived.with_raw_order(&keys.0, stride, &mut |order: &RawOrder| {
        order.firsts(&mut kept.0);
        let mut driver = UnionDriver {
            slots: &mut slots,
            keep: kept,
            stride: Stride(stride),
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
        written_result(answer, driver.failure.take())?;
        driver.slots.0.truncate(driver.written.0 * stride);
        Ok(())
    })?;
    Ok(Positions {
        width: output.width,
        slots,
    })
}

struct AntiDriver<'a, 't> {
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
    counts: &'a WorkCounters,
}
impl SetPositionsDriver for AntiDriver<'_, '_> {
    fn next(&mut self) -> Result<bool, SetInterrupted> {
        self.failure
            .run(|| {
                self.current = (self.next.0 < count(self.left).0).then_some(self.next);
                if self.current.is_some() {
                    self.counts.found(0, 1);
                    self.next.0 += 1;
                }
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
                let at = self.current.ok_or(OperatingError::WrongOperation)?;
                self.probe.0.clear();
                key_into(
                    self.tree,
                    self.columns,
                    row(self.left, at),
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
                let at = self.current.ok_or(OperatingError::WrongOperation)?;
                self.out.0.extend_from_slice(row(self.left, at).0);
                Ok(())
            })
            .map_err(|()| SetInterrupted)
    }
}

pub(super) fn antijoin(
    semantics: &dyn PrimitiveSemantics,
    derived: &mut dyn ScopedDerived,
    operation: &Operation,
    tree: &TreeRows<'_>,
    inputs: &[PositionInput<'_>],
    output: &LoweredShape,
    work: &mut PositionWork,
    counts: &WorkCounters,
) -> Result<Positions, OperatingError> {
    let Operation::Antijoin {
        left_on, right_on, ..
    } = operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    let left = inputs
        .first()
        .copied()
        .ok_or(OperatingError::MissingInput { position: 1 })?;
    let right = inputs
        .get(1)
        .copied()
        .ok_or(OperatingError::MissingInput { position: 2 })?;
    let PositionWork { keys, probe, .. } = work;
    let mut keyed = Keys::default();
    for at in positions(count(right.rows)) {
        counts.found(1, 1);
        key_into(
            tree,
            &right.shape.columns,
            row(right.rows, at),
            right_on,
            &mut keyed,
        );
    }
    std::mem::swap(&mut keys.0, &mut keyed.0);
    let mut slots = Slots::default();
    let mut probe_keys = std::mem::take(probe);
    let result = derived.with_raw_order(&keys.0, right_on.len(), &mut |order: &RawOrder| {
        let mut driver = AntiDriver {
            tree,
            left: left.rows,
            columns: &left.shape.columns,
            left_on,
            order,
            probe: &mut probe_keys,
            out: &mut slots,
            right_count: count(right.rows),
            next: RowIndex(0),
            current: None,
            failure: FailureLatch::default(),
            counts,
        };
        let answer = semantics.operators(OperatorCall::SetRow {
            input: SetInput::Positions {
                kind: SetKind::Antijoin,
                driver: &mut driver,
            },
        });
        written_result(answer, driver.failure.take())
    });
    *probe = probe_keys;
    result?;
    Ok(Positions {
        width: output.width,
        slots,
    })
}
