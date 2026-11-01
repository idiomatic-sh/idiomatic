//! Lazy demanded-root execution; fallback reconstructs the exact original closure and projection.

use crate::domain::graph_cache::types::roots::value::DemandAnswers;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, OperatorContext};
use crate::domain::operator::collections::operators::closure::item::ClosureItem;
use crate::domain::operator::collections::operators::interface::OperatorsItem as _;
use crate::domain::operator::collections::operators::project::item::ProjectItem;
use crate::domain::stored_graph::types::{GraphAccess, GraphWalk};
use crate::domain::table_model::errors::TableModelError;
use crate::domain::table_model::service::{cell, index, key_of, matching, row_in};
use crate::domain::table_model::types::{KeyIndex, On, Rows, Table};
use ryo_language::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, JoinColumn, ReachLookup, RepeatedRows, Side,
};
use ryo_values::domain::plans::types::Operation;

/// Original logical closure rows and their ordinary stable-order join index.
struct LegacyLookup {
    table: Table,
    order: KeyIndex,
}

/// One backend is prepared only on the first actual left row.
enum Backend {
    Unprepared,
    Compact(GraphAccess),
    Ordinary(Box<LegacyLookup>),
}

/// Reusable buffers are invocation-owned, never borrowed from the mutable file cache during emission.
pub(crate) struct ReachJoin<'a, 'f> {
    /// Logical join metadata remains unchanged.
    operation: &'a Operation,
    /// Physical endpoints/partitions and logical RHS map.
    reach: &'a ReachLookup,
    /// Physical graph input was already held through the ordinary provider boundary.
    right: &'a Rows<'f>,
    /// Lazy compact or ordinary preparation.
    backend: Backend,
    /// Join-key positions binding partition fields followed by source; absent forces ordinary fallback.
    bindings: Option<Vec<usize>>,
    /// Ordered left keys.
    key: Vec<Value>,
    /// Partition values followed by source, reused for every left row.
    request: Vec<Value>,
    /// Exact joined output, reused without per-hit allocation.
    output: Vec<Value>,
    /// Sparse traversal state outside the context that nested emission may mutate.
    walk: GraphWalk,
    /// Actual left rows queried.
    probes: u64,
    /// Actual output multiplicity.
    emitted: u64,
}

impl<'a, 'f: 'a> ReachJoin<'a, 'f> {
    /// Validate physical metadata without evaluating a row predicate or emitting output.
    pub(crate) fn new(
        operation: &'a Operation,
        right: &'a Rows<'f>,
        walk: GraphWalk,
        key: Vec<Value>,
        output: Vec<Value>,
    ) -> Result<Self, OperatingError> {
        let Operation::Join {
            reach: Some(reach),
            left_on,
            right_on,
            filter,
            view,
            ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        if !filter.is_empty() || !view.is_empty() {
            return Err(OperatingError::WrongOperation);
        }
        drop(crate::domain::operation_schema::service::reach_kinds(
            crate::domain::table_model::service::kinds_of(right),
            reach,
        )?);
        let bindings = if left_on.len() == right_on.len() {
            (0..=reach.keys.len())
                .map(|logical| {
                    right_on.iter().position(|column| {
                        reach.columns.get(column.0 as usize)
                            == Some(&ColumnIndex(u32::try_from(logical).unwrap_or(u32::MAX)))
                    })
                })
                .collect()
        } else {
            None
        };
        Ok(Self {
            operation,
            reach,
            right,
            backend: Backend::Unprepared,
            bindings,
            key,
            output,
            walk,
            request: Vec::new(),
            probes: 0,
            emitted: 0,
        })
    }

    /// Complete graph/index preparation precedes the first emission; mutable cache borrows end before emit.
    pub(crate) fn row(
        &mut self,
        left: &[Value],
        context: &mut OperatorContext<'_>,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        self.prepare(context)?;
        self.probes += 1;
        let Operation::Join {
            left_on,
            right_on,
            output,
            ..
        } = self.operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        key_of(left, On::Columns(left_on), &mut self.key)?;
        match &self.backend {
            Backend::Compact(access) => {
                self.request.clear();
                for position in self.bindings.as_ref().expect("compact keys were validated") {
                    self.request.push(self.key[*position]);
                }
                let source = self.request[self.reach.keys.len()];
                let answer = DemandAnswers::new(context.stored_indexes.as_deref_mut()).reach(
                    access,
                    &self.request[..self.reach.keys.len()],
                    source,
                    &mut self.walk,
                );
                let reached = answer
                    .as_deref()
                    .map_or(self.walk.reached.as_slice(), Vec::as_slice);
                for vertex in reached {
                    let destination = access.graph.vertices[*vertex as usize];
                    if !right_on.iter().zip(&self.key).all(|(column, value)| {
                        rhs(self.reach, &self.request, destination, *column) == Some(*value)
                    }) {
                        continue;
                    }
                    joined(
                        self.reach,
                        output,
                        left,
                        &self.request,
                        destination,
                        &mut self.output,
                    )?;
                    emit.row(context, &self.output)?;
                    self.emitted += 1;
                }
            }
            Backend::Ordinary(legacy) => {
                for hit in matching(
                    &legacy.table,
                    On::Columns(right_on),
                    &legacy.order,
                    &self.key,
                ) {
                    let Some(right) = row_in(&legacy.table, *hit) else {
                        continue;
                    };
                    self.output.clear();
                    for column in output {
                        self.output.push(cell(
                            match column.side {
                                Side::Left => left,
                                Side::Right => right,
                            },
                            column.column,
                        )?);
                    }
                    emit.row(context, &self.output)?;
                    self.emitted += 1;
                }
            }
            Backend::Unprepared => return Err(OperatingError::WrongOperation),
        }
        Ok(())
    }

    /// Return every reusable allocation even when the caller's emission failed.
    pub(crate) fn finish(self) -> (GraphWalk, Vec<Value>, Vec<Value>) {
        tracing::trace!(target: "ryo::diagnostic::graph", event = "reach_join", operator = "join",
            compact = matches!(self.backend, Backend::Compact(_)), probes = self.probes, emitted = self.emitted,
            avoided_full_closure = matches!(self.backend, Backend::Compact(_)));
        (self.walk, self.key, self.output)
    }

    /// Cache ineligibility/budget/representation must not invent an unavailable result or new error.
    fn prepare(&mut self, context: &mut OperatorContext<'_>) -> Result<(), OperatingError> {
        if !matches!(self.backend, Backend::Unprepared) {
            return Ok(());
        }
        let Operation::Join {
            right, right_on, ..
        } = self.operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        if self.bindings.is_some()
            && let Some(access) = crate::domain::graph_cache::service::acquire(
                *right,
                self.right,
                [self.reach.source, self.reach.destination],
                &self.reach.keys,
                context.stored_indexes.as_deref_mut(),
            )?
        {
            self.backend = Backend::Compact(access);
            return Ok(());
        }
        let closure = Operation::Closure {
            edges: *right,
            source: self.reach.source,
            destination: self.reach.destination,
            keys: self.reach.keys.clone(),
        };
        let mut full = Table::default();
        ClosureItem.compute(&closure, &[self.right.clone()], context, &mut full)?;
        let identity = self
            .reach
            .columns
            .iter()
            .enumerate()
            .all(|(index, column)| column.0 as usize == index)
            && self.reach.columns.len() == full.kinds.len();
        let table = if identity {
            full
        } else {
            let mut projected = Table::default();
            ProjectItem.compute(
                &Operation::Project {
                    from: *right,
                    columns: self.reach.columns.clone(),
                    distinct: RepeatedRows::Kept,
                },
                &[Rows::Computed(&full)],
                context,
                &mut projected,
            )?;
            projected
        };
        let mut order = KeyIndex::default();
        index(&table, On::Columns(right_on), &mut order)?;
        self.backend = Backend::Ordinary(Box::new(LegacyLookup { table, order }));
        Ok(())
    }
}

/// A virtual closure row is partition keys, source and the current sorted destination.
fn rhs(
    reach: &ReachLookup,
    request: &[Value],
    destination: Value,
    logical: ColumnIndex,
) -> Option<Value> {
    let column = reach.columns.get(logical.0 as usize)?.0 as usize;
    if column == reach.keys.len() + 1 {
        Some(destination)
    } else {
        request.get(column).copied()
    }
}

/// Project directly from logical closure values rather than materializing reachable-pair tables.
fn joined(
    reach: &ReachLookup,
    output: &[JoinColumn],
    left: &[Value],
    request: &[Value],
    destination: Value,
    into: &mut Vec<Value>,
) -> Result<(), OperatingError> {
    into.clear();
    for column in output {
        into.push(match column.side {
            Side::Left => cell(left, column.column)?,
            Side::Right => rhs(reach, request, destination, column.column).ok_or(
                TableModelError::NoSuchColumn {
                    index: column.column.0,
                },
            )?,
        });
    }
    Ok(())
}
