//! Complete compact graph/domain acquisition under one unchanged per-file byte allowance.

use crate::application::file_dag::derived::backend::CacheBackend;
use crate::domain::operator::types::PreparedPool;
#[cfg(test)]
use crate::domain::{
    operating::errors::OperatingError,
    stored_graph::types::{GraphAccess, GraphDomain},
    table_model::{service::kinds_of, types::Rows},
};
use ryo_language::domain::relations::types::Relation;
#[cfg(test)]
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plans::types::{Operation, Plan, StepScope};
#[cfg(test)]
use std::sync::Arc;

/// Root and nearest answers cannot silently grow into a retained all-pairs closure.
pub(crate) const MEMO_LIMIT: usize = 2 * 1024 * 1024;

/// Enable graph access only for an active physically eligible per-file consumer.
pub(in crate::application::file_dag::derived) fn reserve(
    cache: &mut CacheBackend,
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
#[cfg(test)]
pub(in crate::application::file_dag) fn acquire(
    input: Input,
    rows: &Rows<'_>,
    endpoints: [ColumnIndex; 2],
    keys: &[ColumnIndex],
    cache: Option<&mut CacheBackend>,
) -> Result<Option<GraphAccess>, OperatingError> {
    let plan = Arc::new(crate::domain::derived_views::service::prepare(
        crate::domain::derived_views::types::DerivedKind::Adjacency,
        &crate::domain::derived_views::types::ViewRequest::Adjacency {
            endpoints,
            keys: keys.to_vec(),
        },
        &[crate::domain::operation_schema::types::InputSchema {
            kinds: kinds_of(rows),
            tree: None,
        }],
    )?);
    super::super::graph::acquire(&plan, None, input, rows, cache)
}
#[cfg(test)]
pub(in crate::application::file_dag) fn domain(
    access: &GraphAccess,
    input: Input,
    rows: &Rows<'_>,
    column: ColumnIndex,
    cache: Option<&mut CacheBackend>,
) -> Result<Arc<GraphDomain>, OperatingError> {
    let plan = Arc::new(crate::domain::derived_views::service::prepare(
        crate::domain::derived_views::types::DerivedKind::Membership,
        &crate::domain::derived_views::types::ViewRequest::Membership { column },
        &[crate::domain::operation_schema::types::InputSchema {
            kinds: kinds_of(rows),
            tree: None,
        }],
    )?);
    super::super::graph::domain(&plan, access, input, rows, cache)
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
pub(in crate::application::file_dag::derived) fn room(
    cache: &CacheBackend,
    cost: usize,
) -> Option<usize> {
    let total = cache.bytes.checked_add(cost)?;
    (total <= cache.limit).then_some(total)
}
