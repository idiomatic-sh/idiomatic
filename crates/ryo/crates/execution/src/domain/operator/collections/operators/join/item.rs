//! Join semantics over engine-acquired typed key/native/adjacency views.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::InputShape;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_model::{
    service::cell,
    types::{Addition },
};
use crate::domain::table_reach::{service::reach_all, types::Edges};
use crate::domain::tree_join::service::source_column;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, Side};
use ryo_values::domain::plans::types::{Operation, OperationKind};
mod ordinary;
pub(crate) struct JoinItem;
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
            return ordinary::evaluate(
                left,
                left_on,
                output,
                view,
                !filter.is_empty(),
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
                    for partition in lease.views.resources.closure() {
                        let partition = partition?;
                        for source in partition.sources() {
                            // Bind partition/source before traversal whenever the logical key names them.
                            if !right_on.iter().zip(&key).all(|(column, value)| {
                                let Some(logical) = reach.columns.get(column.0 as usize) else {
                                    return false;
                                };
                                let at = logical.0 as usize;
                                if at < partition.key().len() {
                                    partition.key().get(at) == Some(value)
                                } else if at == partition.key().len() {
                                    source == value
                                } else {
                                    true
                                }
                            }) {
                                continue;
                            }
                            reach_all(
                                &Edges {
                                    table: partition.table(),
                                    order: partition.index(),
                                    from: &[ColumnIndex(0)],
                                    to: ColumnIndex(1),
                                },
                                *source,
                                &mut walk,
                            );
                            for reached in &walk.reached {
                                right.clear();
                                for column in &reach.columns {
                                    let at = column.0 as usize;
                                    right.push(if at < partition.key().len() {
                                        *partition
                                            .key()
                                            .get(at)
                                            .ok_or(OperatingError::ShapeMismatch)?
                                    } else if at == partition.key().len() {
                                        *source
                                    } else if at == partition.key().len() + 1 {
                                        *reached
                                    } else {
                                        return Err(OperatingError::ShapeMismatch);
                                    });
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

use ryo_values::domain::query_bindings::types::{ParameterBinding};
