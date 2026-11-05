//! Ordered DFS traversal over immutable canonical graph resources.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use {ryo_values::domain::rows::service::cell, ryo_values::domain::rows::service::column_in, crate::domain::table_model::service::row_in};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use crate::domain::table_reach::{
    service::{begin_ordered, next_ordered},
    types::Edges,
};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plans::types::{Operation, OperationKind};
pub(crate) struct OrderedDfsItem;
impl OperatorsItem for OrderedDfsItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputShape<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::OrderedDfs,
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
        emit: &mut dyn RowSink,
    ) -> Result<(), OperatingError> {
        let Operation::OrderedDfs {
            root,
            source,
            destination,
            edge,
            root_keys,
            ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        if !matches!(inputs, PreparedInputs::Views(_)) {
            return Err(OperatingError::WrongOperation);
        }
        let prepared = lease
            .views
            .resources
            .dfs()?
            .ok_or(OperatingError::WrongBinding)?;
        let graph = prepared;
        let columns = graph
            .edge_columns()
            .get(
                ..graph
                    .edge_columns()
                    .len()
                    .checked_sub(1)
                    .ok_or(OperatingError::ShapeMismatch)?,
            )
            .ok_or(OperatingError::ShapeMismatch)?;
        let target = *graph
            .edge_columns()
            .last()
            .ok_or(OperatingError::ShapeMismatch)?;
        let canonical_root = *graph
            .root_columns()
            .last()
            .ok_or(OperatingError::ShapeMismatch)?;
        let walked = Edges {
            table: prepared.edges(),
            order: prepared.edge_index(),
            from: columns,
            to: target,
        };
        let mut key = std::mem::take(&mut lease.workspace.captured);
        let mut prefix = std::mem::take(&mut lease.workspace.other);
        let mut joined = std::mem::take(&mut lease.workspace.row);
        let result = (|| {
            let work = lease
                .workspace
                .dfs
                .as_mut()
                .ok_or(OperatingError::WrongBinding)?;
            work.expanded = 0;
            work.examined = 0;
            let mut begin = 0;
            while begin < prepared.root_index().sorted().len() {
                let at = *prepared
                    .root_index()
                    .sorted()
                    .get(begin)
                    .ok_or(OperatingError::ShapeMismatch)?;
                let root_row = row_in(prepared.roots(), at).ok_or(OperatingError::ShapeMismatch)?;
                key.clear();
                for column in graph.root_columns() {
                    key.push(cell(root_row, *column)?);
                }
                prefix.clear();
                for column in root_keys.iter().copied().chain([*root]) {
                    prefix.push(cell(root_row, column)?);
                }
                let partition = key
                    .get(..root_keys.len())
                    .ok_or(OperatingError::ShapeMismatch)?;
                let work = lease
                    .workspace
                    .dfs
                    .as_mut()
                    .ok_or(OperatingError::WrongBinding)?;
                begin_ordered(
                    &walked,
                    graph,
                    partition,
                    cell(root_row, canonical_root)?,
                    &mut work.walk,
                );
                loop {
                    let work = lease
                        .workspace
                        .dfs
                        .as_mut()
                        .ok_or(OperatingError::WrongBinding)?;
                    let Some((edge_at, ordinal)) =
                        next_ordered(&walked, graph, partition, &mut work.walk)?
                    else {
                        break;
                    };
                    let edge_row =
                        row_in(prepared.edges(), edge_at).ok_or(OperatingError::ShapeMismatch)?;
                    joined.clear();
                    joined.extend_from_slice(&prefix);
                    for column in [*source, *destination, *edge] {
                        joined.push(cell(edge_row, column)?);
                    }
                    joined.push(Value::Number(ordinal));
                    emit.row(lease, &joined)?;
                }
                let work = lease
                    .workspace
                    .dfs
                    .as_mut()
                    .ok_or(OperatingError::WrongBinding)?;
                work.expanded += work.walk.expanded;
                work.examined += work.walk.examined;
                begin += 1;
                while begin < prepared.root_index().sorted().len() {
                    let at = *prepared
                        .root_index()
                        .sorted()
                        .get(begin)
                        .ok_or(OperatingError::ShapeMismatch)?;
                    let next = row_in(prepared.roots(), at).ok_or(OperatingError::ShapeMismatch)?;
                    if !graph
                        .root_columns()
                        .iter()
                        .zip(&key)
                        .all(|(column, value)| column_in(next, *column) == Some(value))
                    {
                        break;
                    }
                    begin += 1;
                }
            }
            Ok(())
        })();
        lease.workspace.captured = key;
        lease.workspace.other = prefix;
        lease.workspace.row = joined;
        result
    }
}
