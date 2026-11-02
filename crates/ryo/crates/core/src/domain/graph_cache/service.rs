//! Complete compact graph/domain acquisition under one unchanged per-file byte allowance.

use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::types::PreparedPool;
use crate::domain::stored_graph::service::{build, domain_identity, source};
use crate::domain::stored_graph::types::footprint::value::GraphFootprint;
use crate::domain::stored_graph::types::{DomainEntry, GraphAccess, GraphDomain, GraphEntry};
use crate::domain::stored_indexes::types::FileIndexes;
use crate::domain::table_model::service::{cell_at, check_columns, count_of, kinds_of};
use crate::domain::table_model::types::Rows;
use ryo_language::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};
use ryo_values::domain::plans::types::{Operation, Plan, StepScope};
use std::sync::Arc;

/// Root and nearest answers cannot silently grow into a retained all-pairs closure.
pub(crate) const MEMO_LIMIT: usize = 2 * 1024 * 1024;

/// Enable graph access only for an active physically eligible per-file consumer.
pub(crate) fn reserve(
    cache: &mut FileIndexes,
    plans: &[Plan],
    pool: &PreparedPool,
    active: impl Fn(usize) -> bool,
) {
    let requested = plans
        .iter()
        .enumerate()
        .filter(|(rule, _)| active(*rule))
        .any(|(rule, plan)| {
            let prepared = pool.selection_plans.get(rule);
            plan.steps
                .iter()
                .filter(|step| step.scope == StepScope::PerFile)
                .any(|step| {
                    let edges = match &step.operation {
                        Operation::Join {
                            right,
                            reach: Some(_),
                            ..
                        } => *right,
                        Operation::Axis { edges, .. } => *edges,
                        Operation::Select { .. }
                        | Operation::Project { .. }
                        | Operation::Extend { .. }
                        | Operation::Join { .. }
                        | Operation::Union { .. }
                        | Operation::Intersection { .. }
                        | Operation::Difference { .. }
                        | Operation::Antijoin { .. }
                        | Operation::GroupCount { .. }
                        | Operation::GroupDigest { .. }
                        | Operation::Capture { .. }
                        | Operation::Rank { .. }
                        | Operation::Replace { .. }
                        | Operation::OrderedMatch { .. }
                        | Operation::Words { .. }
                        | Operation::Closure { .. }
                        | Operation::OrderedDfs { .. }
                        | Operation::Cycles { .. }
                        | Operation::Rows { .. }
                        | Operation::Hosted { .. }
                        | Operation::SpanBounds { .. }
                        | Operation::SourceMatches { .. }
                        | Operation::SpanBetween { .. } => return false,
                    };
                    let relation = match edges {
                        Input::Relation(relation) => Some(relation),
                        Input::Step(id) => prepared
                            .and_then(|plan| plan.steps.get(id.0 as usize))
                            .and_then(Option::as_ref)
                            .map(|selector| selector.relation),
                    };
                    relation.is_some_and(|relation| {
                        !matches!(relation, Relation::SyntaxTree | Relation::SyntaxChild)
                            && cache.eligible.contains(&relation)
                    })
                })
        });
    if requested {
        cache.graph_enabled = true;
    }
}

/// Acquire only after the operator's required provider/input holds have succeeded.
pub(crate) fn acquire(
    input: Input,
    rows: &Rows<'_>,
    endpoints: [ColumnIndex; 2],
    keys: &[ColumnIndex],
    cache: Option<&mut FileIndexes>,
) -> Result<Option<GraphAccess>, OperatingError> {
    let Some(cache) = cache.filter(|cache| cache.graph_enabled) else {
        return Ok(None);
    };
    let Some(source) = source(input, rows, endpoints, keys, &cache.eligible) else {
        return Ok(None);
    };
    check_columns(kinds_of(rows), &endpoints)?;
    check_columns(kinds_of(rows), keys)?;
    if let Some(entry) = cache
        .graphs
        .iter()
        .find(|entry| entry.graph.identity == source.identity)
    {
        cache.graph_counts.reuses += 1;
        return Ok(Some(GraphAccess {
            graph: Arc::clone(&entry.graph),
            orientation: source.orientation,
            shared: true,
        }));
    }
    let Some(graph) = build(source.identity, rows, source.endpoints, keys)? else {
        return Ok(None);
    };
    cache.graph_counts.builds += 1;
    let graph = Arc::new(graph);
    let mut shared = false;
    if let (Some(cost), Some((replacement, metadata))) = (
        GraphFootprint::new(cache).graph(&graph),
        slot(&cache.graphs),
    ) {
        if let Some(cost) = cost.checked_add(metadata)
            && super::types::reclamation::value::GraphAdmission::new(cache).prepare(cost)
            && let Some(bytes) = room(cache, cost)
        {
            replace(&mut cache.graphs, replacement);
            cache.graphs.push(GraphEntry {
                graph: Arc::clone(&graph),
                roots: Vec::new(),
            });
            cache.graph_bytes += bytes - cache.bytes;
            cache.bytes = bytes;
            cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(bytes);
            shared = true;
        }
    }
    if !shared {
        cache.graph_counts.refusals += 1;
    }
    tracing::trace!(target: "ryo::diagnostic::graph", event = "adjacency", reused = false, shared,
        relation = ?graph.identity.population.relation, vertices = graph.vertices.len(), edges = graph.neighbors[0].len(),
        graph_bytes = cache.graph_bytes, cache_bytes = cache.bytes, graph_enabled = cache.graph_enabled);
    Ok(Some(GraphAccess {
        graph,
        orientation: source.orientation,
        shared,
    }))
}

/// Domain duplicates remain membership, never extra emitted reach rows.
pub(crate) fn domain(
    access: &GraphAccess,
    input: Input,
    rows: &Rows<'_>,
    column: ColumnIndex,
    mut cache: Option<&mut FileIndexes>,
) -> Result<Arc<GraphDomain>, OperatingError> {
    check_columns(kinds_of(rows), &[column])?;
    let identity = cache
        .as_ref()
        .and_then(|cache| domain_identity(input, rows, column, &cache.eligible));
    if access.shared
        && let (Some(cache), Some(identity)) = (cache.as_ref(), identity.as_ref())
        && let Some(entry) = cache.graph_domains.iter().find(|entry| {
            entry.graph.identity == access.graph.identity
                && entry.domain.identity.as_ref() == Some(identity)
        })
    {
        return Ok(Arc::clone(&entry.domain));
    }
    let mut members = vec![false; access.graph.vertices.len()];
    for row in 0..count_of(rows).0 {
        let Some(value) = cell_at(
            rows,
            crate::domain::table_cells::types::RowIndex(row),
            column,
        ) else {
            continue;
        };
        if let Some(vertex) = access.graph.vertex_ids.get(&(0, value)) {
            members[*vertex as usize] = true;
        }
    }
    let domain = Arc::new(GraphDomain { identity, members });
    if access.shared
        && domain.identity.is_some()
        && let Some(cache) = cache.as_mut()
        && let (Some(cost), Some((replacement, metadata))) = (
            GraphFootprint::new(cache).domain(&domain),
            slot(&cache.graph_domains),
        )
        && let Some(bytes) = cost
            .checked_add(metadata)
            .and_then(|cost| room(cache, cost))
    {
        replace(&mut cache.graph_domains, replacement);
        cache.graph_domains.push(DomainEntry {
            graph: Arc::clone(&access.graph),
            domain: Arc::clone(&domain),
        });
        cache.graph_bytes += bytes - cache.bytes;
        cache.bytes = bytes;
        cache.graph_counts.peak_bytes = cache.graph_counts.peak_bytes.max(bytes);
    }
    tracing::trace!(target: "ryo::diagnostic::graph", event = "domain", vertices = domain.members.len(), source_rows = count_of(rows).0, stable = domain.identity.is_some());
    Ok(domain)
}

/// Replacement metadata is allocated separately, so refusal never grows retained capacity.
pub(crate) fn slot<T>(entries: &Vec<T>) -> Option<(Option<Vec<T>>, usize)> {
    if entries.len() < entries.capacity() {
        return Some((None, 0));
    }
    let mut replacement = Vec::new();
    replacement
        .try_reserve_exact(entries.len().checked_add(1)?)
        .ok()?;
    let more = replacement
        .capacity()
        .checked_sub(entries.capacity())?
        .checked_mul(size_of::<T>())?;
    Some((Some(replacement), more))
}

/// Move existing entries only after complete admission and byte accounting succeeds.
pub(crate) fn replace<T>(entries: &mut Vec<T>, replacement: Option<Vec<T>>) {
    if let Some(mut replacement) = replacement {
        replacement.append(entries);
        *entries = replacement;
    }
}

/// Account all retained payload under the original total; no fixed allowance is stranded.
pub(crate) fn room(cache: &FileIndexes, cost: usize) -> Option<usize> {
    let total = cache.bytes.checked_add(cost)?;
    (total <= cache.limit).then_some(total)
}
