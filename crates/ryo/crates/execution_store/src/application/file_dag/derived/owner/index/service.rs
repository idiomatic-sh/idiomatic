//! Budgeted acquisition of complete typed indexes, with invocation-local fallback.
use super::types::ByteLimit;
use crate::application::file_dag::derived::ledger::types::ByteCount;
use crate::application::file_dag::derived::owner::cache::types::CacheBackend;
use ryo_execution_domain::domain::coverage::types::Partial;
use ryo_relations::domain::relations::types::Relation;

/// Per-file retained index budget; invocation-local fallback retains its existing policy.
pub(crate) const LIMIT: usize = 16 * 1024 * 1024;

/// Start one generation from the actual physical read state, not execution-mode coverage.
pub(crate) fn new(
    wanted: &[Relation],
    partials: &[Partial],
    limit: ByteLimit,
) -> Option<CacheBackend> {
    let requested = wanted
        .len()
        .checked_mul(size_of::<Relation>())?
        .checked_add(size_of::<CacheBackend>())?;
    if requested > limit.0 {
        return None;
    }
    let mut eligible = Vec::new();
    if eligible.try_reserve_exact(wanted.len()).is_err() {
        return None;
    }
    for relation in wanted {
        if !partials.iter().any(|partial| partial.relation == *relation)
            && !eligible.contains(relation)
        {
            eligible.push(*relation);
        }
    }
    let bytes =
        crate::application::file_dag::derived::ledger::capacity::headed::<CacheBackend, Relation>(
            crate::application::file_dag::derived::ledger::types::SlotCount(eligible.capacity()),
        )?
        .0;
    (bytes <= limit.0).then_some(CacheBackend {
        graph_enabled:
            crate::application::file_dag::derived::owner::cache::types::GraphAdmission::Disabled,
        graph_bytes: ByteCount(0),
        memo_bytes: ByteCount(0),
        graphs: Vec::new(),
        graph_domains: Vec::new(),
        nearest: Vec::new(),
        graph_counts: ryo_execution_domain::domain::stored_graph::types::GraphCounts {
            peak_bytes: ryo_execution_domain::domain::stored_graph::types::PeakBytes(bytes),
            ..Default::default()
        },
        eligible,
        bytes: ByteCount(bytes),
        limit,
        builds: Default::default(),
        reuses: Default::default(),
        refusals: Default::default(),
    })
}
