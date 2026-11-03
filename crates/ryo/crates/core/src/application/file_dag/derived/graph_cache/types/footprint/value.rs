//! Actual vector capacities and conservative hash/Arc metadata for bounded graph admission.

use crate::application::file_dag::derived::backend::CacheBackend;
use crate::domain::stored_graph::types::{CompactGraph, GraphDomain, Population};
use crate::domain::table_model::types::{ParameterBinding, SelectionIdentity};
use ryo_language::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{Operand, QueryLiteral};
use std::sync::Arc;

/// Read-only allocation ownership ledger of the current generation.
pub(crate) struct GraphFootprint<'a> {
    /// No entry is evicted while a consumer may still hold its payload.
    cache: &'a CacheBackend,
}

impl<'a> GraphFootprint<'a> {
    /// Account only allocations newly retained by an admission.
    pub(crate) fn new(cache: &'a CacheBackend) -> Self {
        Self { cache }
    }

    /// Entire immutable graph; partition Arc payloads appear in the vector and map but are charged once.
    pub(crate) fn graph(&self, graph: &CompactGraph) -> Option<usize> {
        let mut bytes = arc_bytes::<CompactGraph>()
            .checked_add(vector(&graph.vertices))?
            .checked_add(vector(&graph.vertex_partitions))?
            .checked_add(vector(&graph.partitions))?
            .checked_add(vector(&graph.identity.keys))?
            .checked_add(table::<(Arc<[Value]>, u32)>(graph.partition_ids.capacity()))?
            .checked_add(table::<((u32, Value), u32)>(graph.vertex_ids.capacity()))?
            .checked_add(self.population(&graph.identity.population)?)?;
        for orientation in 0..2 {
            bytes = bytes
                .checked_add(vector(&graph.offsets[orientation]))?
                .checked_add(vector(&graph.neighbors[orientation]))?;
            if let Some(parents) = &graph.forests[orientation] {
                bytes = bytes.checked_add(vector(parents))?;
            }
        }
        for partition in &graph.partitions {
            bytes = bytes
                .checked_add(partition.len().checked_mul(size_of::<Value>())?)?
                .checked_add(2 * size_of::<usize>() + align_of::<Value>() - 1)?;
        }
        Some(bytes)
    }

    /// A stable membership context owns only its bool vector and newly retained population metadata.
    pub(crate) fn domain(&self, domain: &GraphDomain) -> Option<usize> {
        arc_bytes::<GraphDomain>()
            .checked_add(vector(&domain.members))?
            .checked_add(
                domain
                    .identity
                    .as_ref()
                    .map_or(Some(0), |identity| self.population(&identity.population))?,
            )
    }

    /// Shared row-ID allocations are charged once even when a graph retains an existing selected population.
    pub(crate) fn rows(&self, rows: &Arc<Vec<u32>>) -> Option<usize> {
        let known = self
            .cache
            .selections
            .iter()
            .any(|entry| Arc::ptr_eq(&entry.rows, rows))
            || self.cache.graphs.iter().any(|entry| {
                entry
                    .graph
                    .identity
                    .population
                    .positions
                    .as_ref()
                    .is_some_and(|known| Arc::ptr_eq(known, rows))
                    || entry
                        .roots
                        .iter()
                        .any(|root| Arc::ptr_eq(&root.reached, rows))
            })
            || self.cache.graph_domains.iter().any(|entry| {
                entry
                    .domain
                    .identity
                    .as_ref()
                    .and_then(|identity| identity.population.positions.as_ref())
                    .is_some_and(|known| Arc::ptr_eq(known, rows))
            });
        if known {
            Some(0)
        } else {
            arc_bytes::<Vec<u32>>().checked_add(vector(rows))
        }
    }

    /// Predicate/binding data already owned by prepared selectors needs no second payload allocation.
    fn identity(&self, identity: &Arc<SelectionIdentity>) -> Option<usize> {
        let known = self
            .cache
            .selectors
            .iter()
            .any(|selector| Arc::ptr_eq(&selector.identity, identity))
            || self.cache.graphs.iter().any(|entry| {
                entry
                    .graph
                    .identity
                    .population
                    .selected
                    .as_ref()
                    .is_some_and(|known| Arc::ptr_eq(known, identity))
            })
            || self.cache.graph_domains.iter().any(|entry| {
                entry
                    .domain
                    .identity
                    .as_ref()
                    .and_then(|identity| identity.population.selected.as_ref())
                    .is_some_and(|known| Arc::ptr_eq(known, identity))
            });
        if known {
            return Some(0);
        }
        let mut bytes = arc_bytes::<SelectionIdentity>()
            .checked_add(vector(&identity.comparisons))?
            .checked_add(vector(&identity.bindings))?;
        for comparison in &identity.comparisons {
            bytes = bytes
                .checked_add(operand(&comparison.left)?)?
                .checked_add(operand(&comparison.right)?)?;
        }
        for binding in &identity.bindings {
            bytes = bytes.checked_add(bound(binding)?)?;
        }
        Some(bytes)
    }

    /// Source references themselves are inline metadata; retained owned Arc payloads need explicit accounting.
    fn population(&self, population: &Population) -> Option<usize> {
        population
            .positions
            .as_ref()
            .map_or(Some(0), |rows| self.rows(rows))?
            .checked_add(
                population
                    .selected
                    .as_ref()
                    .map_or(Some(0), |identity| self.identity(identity))?,
            )
    }
}

/// Account actual capacity rather than live length.
pub(crate) fn vector<T>(values: &Vec<T>) -> usize {
    values.capacity().saturating_mul(size_of::<T>())
}

/// Arc allocation header and payload alignment, excluding separately owned vectors.
pub(crate) const fn arc_bytes<T>() -> usize {
    size_of::<T>() + 2 * size_of::<usize>() + align_of::<T>() - 1
}

/// Conservative bucket/control/alignment bound; hashbrown capacity is not allocated bucket count.
fn table<T>(capacity: usize) -> usize {
    if capacity == 0 {
        0
    } else {
        capacity
            .saturating_add(1)
            .checked_next_power_of_two()
            .unwrap_or(usize::MAX)
            .saturating_mul(size_of::<T>() + 1)
            .saturating_add(64)
    }
}

/// Owned literal/function operand data, including any nested argument vectors.
fn operand(value: &Operand) -> Option<usize> {
    match value {
        Operand::Column(_) | Operand::Parameter(_) | Operand::Literal(QueryLiteral::Number(_)) => {
            Some(0)
        }
        Operand::Literal(QueryLiteral::Text(text)) => Some(text.0.len()),
        Operand::Function(function) => function
            .arguments
            .iter()
            .try_fold(vector(&function.arguments), |bytes, argument| {
                bytes.checked_add(operand(argument)?)
            }),
    }
}

/// Request binding vectors/strings are counted by actual owned storage, never by a semantic hash.
fn bound(binding: &ParameterBinding) -> Option<usize> {
    match binding {
        ParameterBinding::Number(_) | ParameterBinding::Flag(_) => Some(0),
        ParameterBinding::Text(text) => Some(text.0.len()),
        ParameterBinding::Numbers(values) => Some(vector(values)),
        ParameterBinding::Texts(values) => values.iter().try_fold(vector(values), |bytes, text| {
            bytes.checked_add(text.0.len())
        }),
        ParameterBinding::TextSets(sets) => sets.iter().try_fold(vector(sets), |bytes, values| {
            values
                .iter()
                .try_fold(bytes.checked_add(vector(values))?, |bytes, text| {
                    bytes.checked_add(text.0.len())
                })
        }),
        ParameterBinding::Tables(values) => values.iter().try_fold(vector(values), |bytes, row| {
            row.0
                .iter()
                .try_fold(bytes.checked_add(vector(&row.0))?, |bytes, cell| {
                    bytes
                        .checked_add(cell.key.0.len())?
                        .checked_add(cell.value.0.len())
                })
        }),
    }
}
