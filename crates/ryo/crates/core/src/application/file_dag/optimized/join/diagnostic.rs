//! One opt-in aggregate observation per physical Join invocation.
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::contract::{resources::ResourceProgram, types::OperatorContract};
use crate::domain::table_model::{
    service::{count_of, kinds_of, rectangular},
    types::Rows,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};

#[derive(Default)]
pub(super) struct Counts {
    pub(super) enabled: bool,
    pub(super) builds: u64,
    pub(super) probes: u64,
    pub(super) hits: u64,
    pub(super) eligible: bool,
    pub(super) reuses: u64,
    pub(super) sparse_left: bool,
}
impl Counts {
    pub(super) fn derived(&mut self, before: (u64, u64), after: (u64, u64), eligible: bool) {
        if self.enabled {
            self.builds = self.builds.saturating_add(after.0.saturating_sub(before.0));
            self.reuses = self.reuses.saturating_add(after.1.saturating_sub(before.1));
            self.eligible |= eligible;
        }
    }
    pub(super) fn finish(
        &self,
        right: &Rows<'_>,
        keys: &[ColumnIndex],
        logical: Input,
        path: &str,
        complete: bool,
    ) {
        if !self.enabled {
            return;
        }
        tracing::event!(target:"ryo::diagnostic::stored_join", tracing::Level::TRACE,
            rhs_rows=count_of(right).0, rhs_width=kinds_of(right).len(), key_width=keys.len(),
            right_input=?logical, key_columns=?keys, rhs_kinds=?kinds_of(right),
            read_state_observed=self.eligible, complete_relation_eligible=self.eligible,
            cache_reuses=self.reuses, builds=self.builds, probes=self.probes,
            hits=self.hits, path, complete);
    }
}

/// The transport owns this per-invocation observation while streaming left
/// rows. It emits once after success or failure; normal execution allocates
/// nothing and never records per-hit events.
pub(crate) struct StreamJoinStats {
    counts: Counts,
    before_builds: u64,
    before_reuses: u64,
    had_cache: bool,
}
impl StreamJoinStats {
    pub(crate) fn begin(context: &OperatorContext<'_>) -> Self {
        let cache = context.stored_indexes.as_deref();
        Self {
            counts: Counts {
                enabled: tracing::enabled!(target:"ryo::diagnostic::stored_join",tracing::Level::TRACE),
                ..Counts::default()
            },
            before_builds: cache.map_or(0, |cache| cache.builds),
            before_reuses: cache.map_or(0, |cache| cache.reuses),
            had_cache: cache.is_some(),
        }
    }
    pub(crate) fn probe(&mut self) {
        if self.counts.enabled {
            self.counts.probes = self.counts.probes.saturating_add(1);
        }
    }
    pub(crate) fn hit(&mut self) {
        if self.counts.enabled {
            self.counts.hits = self.counts.hits.saturating_add(1);
        }
    }
    pub(crate) fn finish(
        mut self,
        contract: &OperatorContract,
        right: &Rows<'_>,
        context: &OperatorContext<'_>,
        success: bool,
    ) {
        if !self.counts.enabled {
            return;
        }
        let ResourceProgram::Join(join) = &contract.resources else {
            return;
        };
        if join.reach.is_some()
            || join.has_filter
            || !join.view.is_empty()
            || matches!(right, Rows::Tree(_))
        {
            return;
        }
        if let Some(cache) = context.stored_indexes.as_deref() {
            self.counts.builds = cache.builds.saturating_sub(self.before_builds);
            self.counts.reuses = cache.reuses.saturating_sub(self.before_reuses);
            self.counts.eligible=ryo_values::domain::plans::service::named(&contract.inputs)
                .get(1).is_some_and(|input|matches!(input,Input::Relation(relation) if cache.eligible.contains(relation)));
        } else if !self.had_cache {
            self.counts.builds = u64::from(self.counts.probes > 0 && count_of(right).0 > 0);
        }
        let Some(logical) = ryo_values::domain::plans::service::named(&contract.inputs)
            .get(1)
            .copied()
        else {
            return;
        };
        let path = if matches!(right, Rows::Selected(_))
            || matches!(right,Rows::Stored(view) if rectangular(*view))
        {
            "stored-key"
        } else {
            "materialized"
        };
        self.counts
            .finish(right, &join.right_on, logical, path, success);
    }
}
