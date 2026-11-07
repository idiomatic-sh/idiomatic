//! Borrowed physical cursor for the ordinary Join capability; no retained ownership.
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::{
    types::{PrimitiveLease, RowSink, join_result},
    views::{IndexedRows, KeyProbe},
};

use crate::domain::table_model::types::Verdict;
use crate::domain::tree_join::{
    collections::strategies::{
        entity_vector::item::EntityVectorItem, span_index::item::SpanIndexItem, types::NodeColumns,
    },
    service::driver::tree_value,
};
use ryo_derived::domain::contract::types::Finding;
use ryo_operators::domain::equality_join::types::JoinDriver;
use ryo_operators::domain::equality_join::errors::EqualityJoinError;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_relations::domain::relations::types::{Position, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn};

struct Driver<'a, 'f> {
    lease: &'a mut PrimitiveLease<'f>,
    sink: &'a mut dyn RowSink,
    left: &'a [Value],
    index: Option<IndexedRows<'f>>,
    probe: Option<KeyProbe>,
    current: Option<RowIndex>,
    positions: Vec<Position>,
    at: usize,
    started: bool,
    position: Option<Position>,
    filtered: bool,
    error: Option<OperatingError>,
}
impl Driver<'_, '_> {
    fn advance(&mut self, key: &[Value]) -> Result<bool, OperatingError> {
        if let Some(native) = self.lease.views.resources.native_join() {
            if !self.started {
                self.started = true;
                self.positions.clear();
                match native.finding {
                    Finding::Node(column, key) => EntityVectorItem::find(
                        &native.tree,
                        self.left,
                        NodeColumns { left: column, key },
                        &mut self.positions,
                    )?,
                    Finding::Stretch(..) | Finding::Span(_) => SpanIndexItem::find(
                        &native.tree,
                        self.left,
                        native.finding,
                        &mut self.positions,
                    )?,
                }
            }
            while let Some(position) = self.positions.get(self.at).copied() {
                self.at += 1;
                self.lease.found(1, 1);
                #[cfg(feature = "diagnostic-row-progress")]
                self.lease.progress(
                    crate::domain::operator::contract::views::ProgressCounter::JoinCandidatePairs,
                    1,
                );
                if self.filtered {
                    let row = std::array::from_fn::<_, 5, _>(|at| {
                        tree_value(
                            &native.tree,
                            position,
                            ColumnIndex(u32::try_from(at).unwrap_or(u32::MAX)),
                        )
                    });
                    if self.lease.holds(&row)? == Verdict::Fails {
                        continue;
                    }
                }
                self.position = Some(position);
                return Ok(true);
            }
            self.position = None;
            return Ok(false);
        }
        let index = self.index.ok_or(OperatingError::NotRun)?;
        let probe = self.probe.get_or_insert_with(|| index.probe(key));
        self.current = index.next_match(key, probe);
        if self.current.is_some() {
            self.lease.found(1, 1);
        }
        #[cfg(feature = "diagnostic-row-progress")]
        if self.current.is_some() {
            self.lease.progress(
                crate::domain::operator::contract::views::ProgressCounter::JoinCandidatePairs,
                1,
            );
        }
        Ok(self.current.is_some())
    }
}
impl JoinDriver for Driver<'_, '_> {
    fn next(&mut self, key: &[Value]) -> Result<bool, EqualityJoinError> {
        if self.error.is_some() {
            return Err(EqualityJoinError::Interrupted);
        }
        self.advance(key).map_err(|error| {
            self.error = Some(error);
            EqualityJoinError::Interrupted
        })
    }
    fn right_cell(&self, column: ColumnIndex) -> Option<Value> {
        if self.error.is_some() {
            return None;
        }
        if let (Some(native), Some(position)) =
            (self.lease.views.resources.native_join(), self.position)
        {
            Some(tree_value(&native.tree, position, column))
        } else {
            self.current
                .and_then(|at| self.index?.rows.cell(at, column))
        }
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), EqualityJoinError> {
        if self.error.is_some() {
            return Err(EqualityJoinError::Interrupted);
        }
        self.sink.row(self.lease, row).map_err(|error| {
            self.error = Some(error);
            EqualityJoinError::Interrupted
        })
    }
}
pub(super) fn evaluate(
    left: &[Value],
    left_on: &[ColumnIndex],
    output: &[JoinColumn],
    view: &[ColumnIndex],
    filtered: bool,
    lease: &mut PrimitiveLease<'_>,
    sink: &mut dyn RowSink,
) -> Result<(), OperatingError> {
    let semantics = lease.semantics();
    // Indexed resources already expose the requested logical RHS view. Native
    // candidates expose physical tree columns and need the declared remapping.
    let view = if lease.views.resources.native_join().is_some() {
        view
    } else {
        &[]
    };
    let mut key = std::mem::take(&mut lease.workspace.row);
    let mut target = std::mem::take(&mut lease.workspace.other);
    let positions = std::mem::take(&mut lease.workspace.positions);
    let index = lease.views.resources.index();
    let mut driver = Driver {
        lease,
        sink,
        left,
        index,
        probe: None,
        current: None,
        positions,
        at: 0,
        started: false,
        position: None,
        filtered,
        error: None,
    };
    let answer = semantics.operators(OperatorCall::EqualityJoin {
        input: ryo_operators::domain::equality_join::types::JoinInput::Values {
            left: ryo_operators::domain::equality_join::types::JoinRow::Values(left),
            left_on,
            output,
            view,
            driver: &mut driver,
            key: &mut key,
            target: &mut target,
        },
    });
    let error = driver.error.take();
    driver.lease.workspace.positions = std::mem::take(&mut driver.positions);
    driver.lease.workspace.row = key;
    driver.lease.workspace.other = target;
    join_result(answer, error)
}

use ryo_values::domain::rows::types::RowIndex;
