//! Join semantics over engine-acquired typed key/native/adjacency views.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::InputShape;
use crate::domain::operator::contract::{
    join::closure_cell,
    resources::Filtering,
    types::{OperatorContract, PreparedInputs, PrimitiveLease, RowSink, join_result},
    views::{IndexedRows, KeyProbe},
};
use crate::domain::table_model::types::Verdict;
use crate::domain::table_reach::{service::reach_all, types::Edges};
use crate::domain::tree_join::{
    collections::strategies::{
        entity_vector::item::EntityVectorItem, span_index::item::SpanIndexItem, types::NodeColumns,
    },
    service::{driver::tree_value, source_column},
};
use ryo_derived::domain::contract::types::Finding;
use ryo_operators::domain::equality_join::errors::EqualityJoinError;
use ryo_operators::domain::equality_join::types::{Advance, JoinDriver};
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_relations::domain::relations::types::{Position, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, Side};
use ryo_values::domain::rows::types::RowIndex;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_operators::types::OperationKind;
use {crate::domain::table_model::types::Addition, ryo_values::domain::rows::service::cell};
pub(crate) struct JoinItem;
/// Where the walk over the native candidates has reached.
struct PositionCursor(usize);
/// Whether the native candidates of a left row have been found.
#[derive(PartialEq, Eq)]
enum Started {
    Yes,
    No,
}
fn joined(
    output: &[JoinColumn],
    left: &[Value],
    right: &[Value],
    view: &[ColumnIndex],
    answer: &mut Vec<Value>,
) -> Result<(), OperatingError> {
    answer.clear();
    for column in output {
        answer.push(match column.side {
            Side::Left => cell(left, column.column)?,
            Side::Right => cell(right, source_column(view, column.column))?,
        });
    }
    Ok(())
}
impl OperatorsItem for JoinItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[InputShape<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::Join,
            operation,
            inputs,
            bindings,
        )
    }
    fn evaluate(
        &self,
        operation: &Operation,
        inputs: PreparedInputs<'_>,
        lease: &mut PrimitiveLease<'_>,
        sink: &mut dyn RowSink,
    ) -> Result<(), OperatingError> {
        let Operation::Join {
            left_on,
            right_on,
            output,
            view,
            reach,
            filter,
            ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let PreparedInputs::Row { row: left, .. } = inputs else {
            return Err(OperatingError::WrongOperation);
        };
        if reach.is_none() {
            return evaluate_equality(
                left,
                left_on,
                output,
                view,
                if filter.is_empty() {
                    Filtering::Unfiltered
                } else {
                    Filtering::Filtered
                },
                lease,
                sink,
            );
        }
        let mut answer = std::mem::take(&mut lease.workspace.other);
        let mut key = std::mem::take(&mut lease.workspace.row);
        let mut right = std::mem::take(&mut lease.workspace.captured);
        let outcome = (|| {
            key.clear();
            for column in left_on {
                key.push(cell(left, *column)?);
            }
            if let (Some(reach), Some(graph)) = (reach, lease.views.cached_reach) {
                for destination in graph.destinations {
                    right.clear();
                    for column in &reach.columns {
                        let at = column.0 as usize;
                        right.push(if at < reach.keys.len() + 1 {
                            *graph.request.get(at).ok_or(OperatingError::ShapeMismatch)?
                        } else if at == reach.keys.len() + 1 {
                            *destination
                        } else {
                            return Err(OperatingError::ShapeMismatch);
                        });
                    }
                    if !right_on
                        .iter()
                        .zip(&key)
                        .all(|(column, value)| right.get(column.0 as usize) == Some(value))
                    {
                        continue;
                    }
                    let distinct = lease.distinct()?;
                    if crate::domain::table_dedup::service::first_time(
                        &mut distinct.rows,
                        &mut distinct.seen,
                        &right,
                    ) != Addition::Added
                    {
                        continue;
                    }
                    joined(output, left, &right, &[], &mut answer)?;
                    sink.row(lease, &answer)?;
                }
                return Ok(());
            }
            if let Some(reach) = reach {
                let mut walk = lease
                    .workspace
                    .closure_walk
                    .take()
                    .ok_or(OperatingError::NotRun)?;
                let result = (|| {
                    for partition in lease.views.resources.closure()? {
                        let partition = partition?;
                        for source in &partition.sources {
                            // Bind partition/source before traversal whenever the logical key names them.
                            if !right_on.iter().zip(&key).all(|(column, value)| {
                                let Some(logical) = reach.columns.get(column.0 as usize) else {
                                    return false;
                                };
                                let at = logical.0 as usize;
                                if at < partition.key.len() {
                                    partition.key.get(at) == Some(value)
                                } else if at == partition.key.len() {
                                    source == value
                                } else {
                                    true
                                }
                            }) {
                                continue;
                            }
                            reach_all(
                                &Edges {
                                    table: &partition.table,
                                    order: &partition.index,
                                    from: &[ColumnIndex(0)],
                                    to: ColumnIndex(1),
                                },
                                *source,
                                &mut walk,
                            );
                            for reached in &walk.reached {
                                right.clear();
                                for column in &reach.columns {
                                    right.push(closure_cell(
                                        &partition.key,
                                        *column,
                                        source,
                                        reached,
                                    )?);
                                }
                                if !right_on.iter().zip(&key).all(|(column, value)| {
                                    right.get(column.0 as usize) == Some(value)
                                }) {
                                    continue;
                                }
                                // Projection of a logical closure has set semantics. The engine resets
                                // this bounded distinct workspace at the next left-row invocation.
                                let distinct = lease.distinct()?;
                                if crate::domain::table_dedup::service::first_time(
                                    &mut distinct.rows,
                                    &mut distinct.seen,
                                    &right,
                                ) != Addition::Added
                                {
                                    continue;
                                }
                                joined(output, left, &right, &[], &mut answer)?;
                                sink.row(lease, &answer)?;
                            }
                        }
                    }
                    Ok(())
                })();
                lease.workspace.closure_walk = Some(walk);
                return result;
            }
            Err(OperatingError::WrongOperation)
        })();
        lease.workspace.other = answer;
        lease.workspace.row = key;
        lease.workspace.captured = right;
        outcome
    }
}

struct Driver<'a, 'f> {
    lease: &'a mut PrimitiveLease<'f>,
    sink: &'a mut dyn RowSink,
    left: &'a [Value],
    index: Option<IndexedRows<'f>>,
    probe: Option<KeyProbe>,
    current: Option<RowIndex>,
    positions: Vec<Position>,
    at: PositionCursor,
    started: Started,
    position: Option<Position>,
    filtered: Filtering,
    error: Option<OperatingError>,
}
impl Driver<'_, '_> {
    fn advance(&mut self, key: &[Value]) -> Result<Advance, OperatingError> {
        if let Some(native) = self.lease.views.resources.native_join() {
            if self.started == Started::No {
                self.started = Started::Yes;
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
            while let Some(position) = self.positions.get(self.at.0).copied() {
                self.at.0 += 1;
                self.lease.found(ryo_values::domain::plan_parts::types::InputNumber(1), crate::domain::operator::contract::types::FoundRows(1));
                #[cfg(feature = "diagnostic-row-progress")]
                self.lease.progress(
                    crate::domain::operator::contract::views::ProgressCounter::JoinCandidatePairs,
                    crate::domain::operator::contract::views::ProgressAmount(1),
                );
                if self.filtered == Filtering::Filtered {
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
                return Ok(Advance::Admitted);
            }
            self.position = None;
            return Ok(Advance::Exhausted);
        }
        let index = self.index.ok_or(OperatingError::NotRun)?;
        let probe = self.probe.get_or_insert_with(|| index.probe(key));
        self.current = index.next_match(key, probe);
        if self.current.is_some() {
            self.lease.found(ryo_values::domain::plan_parts::types::InputNumber(1), crate::domain::operator::contract::types::FoundRows(1));
        }
        #[cfg(feature = "diagnostic-row-progress")]
        if self.current.is_some() {
            self.lease.progress(
                crate::domain::operator::contract::views::ProgressCounter::JoinCandidatePairs,
                crate::domain::operator::contract::views::ProgressAmount(1),
            );
        }
        Ok(Advance::from(self.current.is_some()))
    }
}
impl JoinDriver for Driver<'_, '_> {
    fn next(&mut self, key: &[Value]) -> Result<Advance, EqualityJoinError> {
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
fn evaluate_equality(
    left: &[Value],
    left_on: &[ColumnIndex],
    output: &[JoinColumn],
    view: &[ColumnIndex],
    filtered: Filtering,
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
        at: PositionCursor(0),
        started: Started::No,
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

use ryo_values::domain::query_bindings::types::ParameterBinding;
