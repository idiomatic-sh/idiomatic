//! The `GraphLookups` value: the ordered keys and the cycle adjacency one evaluation acquired, the engine's
//! strategies behind the operators' ordered-match and cycle ports.

use super::super::row_delivery::value::RowDelivery;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::failure::types::LatchState;
use crate::domain::operator::contract::primitive::types::PreparedInputs;
use crate::domain::operator::contract::views::types::PreparedAdjacency;
use ryo_derived::domain::ordered::types::OrderedMatching;
use ryo_operators::domain::contract::types::PortableOperatorFault as Fault;
use ryo_operators::domain::cycles::types::CycleDriver;
use ryo_operators::domain::ordered_match::types::KeyOrder;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::rows::types::{RowCount, RowIndex};

/// The ordered keys and adjacency an evaluation acquired, with the buffers a pair is read into.
#[derive(Default)]
pub(crate) struct GraphLookups<'f> {
    ordered: Option<&'f OrderedMatching>,
    cycles: Option<PreparedAdjacency<'f>>,
    left_buffer: Vec<Value>,
    right_buffer: Vec<Value>,
}

impl<'f> GraphLookups<'f> {
    /// Acquires the cycle adjacency.
    pub(crate) fn cycles(
        &mut self,
        rows: &mut RowDelivery<'_, 'f>,
    ) -> Result<&dyn CycleDriver, Fault> {
        let graph = rows
            .resources()
            .cycles()
            .and_then(|graph| graph.ok_or(OperatingError::WrongBinding));
        let graph = rows.hold(graph)?;
        self.cycles = Some(PreparedAdjacency(graph));
        self.cycles
            .as_ref()
            .map(|graph| -> &dyn CycleDriver { graph })
            .ok_or(Fault::WrongBinding)
    }

    /// Checks the two ordered inputs and acquires their ordered keys.
    pub(crate) fn ordered_match(&mut self, rows: &mut RowDelivery<'_, 'f>) -> Result<(), Fault> {
        let checked = match rows.inputs() {
            PreparedInputs::Views(inputs) => inputs
                .first()
                .ok_or(OperatingError::MissingInput { position: 1 })
                .and_then(|_| {
                    inputs
                        .get(1)
                        .ok_or(OperatingError::MissingInput { position: 2 })
                })
                .and_then(|_| rows.resources().ordered())
                .and_then(|ordered| ordered.ok_or(OperatingError::WrongBinding)),
            PreparedInputs::Row { .. } | PreparedInputs::RequestSource(_) => {
                Err(OperatingError::WrongOperation)
            }
        };
        self.ordered = Some(rows.hold(checked)?);
        Ok(())
    }

    /// How many left rows the ordered keys pair.
    pub(crate) fn left_len(&self) -> RowCount {
        RowCount(self.ordered.map_or(0, |ordered| ordered.left.len()))
    }

    /// How many right rows the ordered keys pair.
    pub(crate) fn right_len(&self) -> RowCount {
        RowCount(self.ordered.map_or(0, |ordered| ordered.right.len()))
    }

    /// How a left row's keys compare with a right row's.
    pub(crate) fn compare_keys(&self, left: RowIndex, right: RowIndex) -> std::cmp::Ordering {
        self.ordered.map_or(std::cmp::Ordering::Equal, |ordered| {
            ordered.left[left.0].keys.cmp(&ordered.right[right.0].keys)
        })
    }

    /// Whether a right row comes before or with a left row in their order.
    pub(crate) fn right_order_before_or_equal(&self, left: RowIndex, right: RowIndex) -> KeyOrder {
        match self.ordered {
            Some(ordered) if ordered.right[right.0].order <= ordered.left[left.0].order => {
                KeyOrder::BeforeOrEqual
            }
            Some(_) | None => KeyOrder::After,
        }
    }

    /// Reads a left and a right row and hands them to `project`.
    pub(crate) fn visit_pair(
        &mut self,
        rows: &mut RowDelivery<'_, '_>,
        left: RowIndex,
        right: RowIndex,
        project: &mut dyn FnMut(&[Value], &[Value]) -> Result<(), Fault>,
    ) -> Result<(), Fault> {
        if rows.interrupted() == LatchState::Failed {
            return Err(Fault::Interrupted);
        }
        let (Some(ordered), PreparedInputs::Views(inputs)) = (self.ordered, rows.inputs()) else {
            return rows.hold(Err(OperatingError::WrongBinding));
        };
        let (Some(left_rows), Some(right_rows)) = (inputs.first(), inputs.get(1)) else {
            return rows.hold(Err(OperatingError::ShapeMismatch));
        };
        let (left_buffer, right_buffer) = (&mut self.left_buffer, &mut self.right_buffer);
        let pair = left_rows
            .read(ordered.left[left.0].row, left_buffer)
            .ok_or(OperatingError::ShapeMismatch)
            .and_then(|left| {
                right_rows
                    .read(ordered.right[right.0].row, right_buffer)
                    .ok_or(OperatingError::ShapeMismatch)
                    .map(|right| project(left, right))
            });
        rows.hold(pair)?
    }
}
