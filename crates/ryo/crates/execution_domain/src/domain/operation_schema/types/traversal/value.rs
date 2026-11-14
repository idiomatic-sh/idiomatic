//! Traversal schemas: checked endpoint, partition, order and native-tree projection metadata.

use crate::domain::operating::{errors::OperatingError, types::InputPlace};
use crate::domain::operation_schema::projection::{
    service::joined, service::projected, service::schema_input,
};
use crate::domain::operation_schema::types::InputShape;
use crate::domain::operation_schema::types::bindings::value::BindingShape;
use ryo_operators::domain::tree_cells::types::TreeForm;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction, Side};
use ryo_values::domain::plans::service::kind_of;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_operators::types::OperationKind;
use {
    crate::domain::table_model::service::check_columns,
    ryo_values::domain::rows::service::column_in,
};

/// The input schemas of one graph/ordered traversal, with no adjacency or row storage.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TraversalShape<'a> {
    /// Ordered input metadata, without graph payloads.
    inputs: &'a [InputShape<'a>],
}

impl<'a> TraversalShape<'a> {
    /// Borrow graph input metadata.
    pub(crate) const fn new(inputs: &'a [InputShape<'a>]) -> Self {
        Self { inputs }
    }
    /// Validate and derive a traversal's output kinds without traversing it.
    pub(crate) fn of(
        &self,
        operation: &Operation,
        bindings: &[ParameterBinding],
    ) -> Result<Vec<CellKind>, OperatingError> {
        match kind_of(operation) {
            OperationKind::Axis => axis_schema(operation, self.inputs, bindings),
            OperationKind::OrderedMatch => ordered_schema(operation, self.inputs),
            OperationKind::OrderedDfs => dfs_schema(operation, self.inputs),
            OperationKind::Select
            | OperationKind::Project
            | OperationKind::Extend
            | OperationKind::Join
            | OperationKind::Union
            | OperationKind::Intersection
            | OperationKind::Difference
            | OperationKind::Antijoin
            | OperationKind::GroupCount
            | OperationKind::GroupDigest
            | OperationKind::Capture
            | OperationKind::Rank
            | OperationKind::Replace
            | OperationKind::Words
            | OperationKind::SpanBounds
            | OperationKind::SourceMatches
            | OperationKind::RenderSyntax
            | OperationKind::FitsRendered
            | OperationKind::SpanBetween
            | OperationKind::Closure
            | OperationKind::Cycles
            | OperationKind::Rows
            | OperationKind::Hosted => Err(OperatingError::WrongOperation),
        }
    }
}

/// Numeric order and equal key kinds of an ordered match, without indexing rows.
fn ordered_schema(
    operation: &Operation,
    inputs: &[InputShape<'_>],
) -> Result<Vec<CellKind>, OperatingError> {
    let Operation::OrderedMatch {
        left_on,
        right_on,
        left_order,
        right_order,
        output,
        ..
    } = operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    let (left, right) = (
        schema_input(inputs, InputPlace(0))?,
        schema_input(inputs, InputPlace(1))?,
    );
    if left_on.len() != right_on.len()
        || column_in(left.kinds, *left_order) != Some(&CellKind::Number)
        || column_in(right.kinds, *right_order) != Some(&CellKind::Number)
        || left_on.iter().zip(right_on).any(|(l, r)| {
            column_in(left.kinds, *l).is_none()
                || column_in(left.kinds, *l) != column_in(right.kinds, *r)
        })
    {
        return Err(OperatingError::ShapeMismatch);
    }
    output
        .iter()
        .map(|column| {
            column_in(
                match column.side {
                    Side::Left => left.kinds,
                    Side::Right => right.kinds,
                },
                column.column,
            )
            .copied()
            .ok_or(OperatingError::ShapeMismatch)
        })
        .collect()
}

/// An ordered DFS's partition, endpoint and identity columns, without traversal storage.
fn dfs_schema(
    operation: &Operation,
    inputs: &[InputShape<'_>],
) -> Result<Vec<CellKind>, OperatingError> {
    let Operation::OrderedDfs {
        root,
        source,
        destination,
        order,
        edge,
        root_keys,
        edge_keys,
        ..
    } = operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    let (roots, edges) = (
        schema_input(inputs, InputPlace(0))?,
        schema_input(inputs, InputPlace(1))?,
    );
    check_columns(roots.kinds, &[*root])?;
    check_columns(roots.kinds, root_keys)?;
    check_columns(edges.kinds, &[*source, *destination, *order, *edge])?;
    check_columns(edges.kinds, edge_keys)?;
    let kind = column_in(roots.kinds, *root).ok_or(OperatingError::ShapeMismatch)?;
    if Some(kind) != column_in(edges.kinds, *source)
        || Some(kind) != column_in(edges.kinds, *destination)
        || column_in(edges.kinds, *order) != Some(&CellKind::Number)
        || root_keys.len() != edge_keys.len()
        || root_keys.iter().zip(edge_keys).any(|(root_key, edge_key)| {
            column_in(roots.kinds, *root_key) != column_in(edges.kinds, *edge_key)
        })
    {
        return Err(OperatingError::ShapeMismatch);
    }
    let mut answer = projected(roots.kinds, root_keys)?;
    answer.extend(projected(roots.kinds, &[*root])?);
    answer.extend(projected(edges.kinds, &[*source, *destination, *edge])?);
    answer.push(CellKind::Number);
    Ok(answer)
}

/// Preserve native tree projection and computed-edge restrictions using only representation metadata.
fn axis_schema(
    operation: &Operation,
    inputs: &[InputShape<'_>],
    bindings: &[ParameterBinding],
) -> Result<Vec<CellKind>, OperatingError> {
    let Operation::Axis {
        direction,
        id,
        parent,
        child,
        domain_id,
        domain_filter,
        domain_view,
        output,
        ..
    } = operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    let (from, edges, domain) = (
        schema_input(inputs, InputPlace(0))?,
        schema_input(inputs, InputPlace(1))?,
        schema_input(inputs, InputPlace(2))?,
    );
    check_columns(from.kinds, &[*id])?;
    let domain_entity = crate::domain::tree_walk::service::entity_of(domain_view, *domain_id);
    let native = edges.tree == Some(TreeForm::Edges)
        && (*parent, *child) == (ColumnIndex(0), ColumnIndex(1))
        && (domain_filter.is_empty() && domain_view.is_empty()
            || domain.tree == Some(TreeForm::Elements) && domain_entity == ColumnIndex(0));
    if native {
        check_columns(domain.kinds, &[domain_entity])?;
        BindingShape::new(bindings).predicates(domain_filter, domain.kinds)?;
        if !output.is_empty() {
            return joined(
                from.kinds,
                if domain.tree == Some(TreeForm::Elements) {
                    domain.kinds
                } else {
                    edges.kinds
                },
                None,
                output,
            );
        }
    } else {
        if !domain_filter.is_empty()
            || !domain_view.is_empty()
            || !output.is_empty()
            || matches!(
                direction,
                Direction::Deep | Direction::PreviousSibling | Direction::NextSibling
            )
        {
            return Err(OperatingError::WrongOperation);
        }
        check_columns(edges.kinds, &[*parent, *child])?;
        check_columns(domain.kinds, &[*domain_id])?;
    }
    let mut answer = from.kinds.to_vec();
    answer.push(CellKind::Node);
    Ok(answer)
}
