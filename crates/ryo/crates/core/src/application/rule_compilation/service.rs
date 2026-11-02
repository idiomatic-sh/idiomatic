//! Compile rule programs once per bounded shared session and bind every request independently.
use super::constants::COMPILED_PLANS;
use super::errors::RuleCompilationError;
use super::types::{CachedPlan, Compiled, CompiledPlans};
use crate::domain::rules::service::{bind, check_ids};
use crate::domain::rules::types::RuleSpec;
use crate::outbound::query_language::errors::QueryLanguageError;
use crate::outbound::query_language::interface::QueryLanguagePort;
use ryo_values::domain::hosted::service::declarations;
use ryo_values::domain::hosted::types::HostRelation;
use std::sync::{Arc, PoisonError};

/// One cohesive compiler operation, sharing only bounded immutable plans across invocations.
pub(crate) struct RuleCompilation {
    /// The injected query compiler; compilation never holds the cache lock.
    query_language: Arc<dyn QueryLanguagePort>,
    /// Session-owned plan state; a run retains its own cloned plans after eviction.
    compiled: CompiledPlans,
}

impl std::fmt::Debug for RuleCompilation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuleCompilation").finish_non_exhaustive()
    }
}

impl RuleCompilation {
    /// Own the supplied compiler and a new bounded session.
    #[must_use]
    pub(crate) fn new(query_language: Arc<dyn QueryLanguagePort>) -> Self {
        Self {
            query_language,
            compiled: CompiledPlans::default(),
        }
    }

    /// Share an explicitly owned session cache with other compilation operations.
    #[must_use]
    pub(crate) fn compiling_into(mut self, plans: CompiledPlans) -> Self {
        self.compiled = plans;
        self
    }

    /// The plans of the rules, compiled once while this lives, and what each binds its parameters to.
    ///
    /// # Errors
    /// Refuses invalid rule IDs, compiler diagnostics and missing request bindings without caching a failed compile.
    pub(crate) fn compile(
        &self,
        rules: &[RuleSpec],
        hosted: &[HostRelation],
    ) -> Result<Compiled, RuleCompilationError> {
        check_ids(rules)?;
        let declared = declarations(hosted);
        let (mut plans, mut bindings) = (Vec::new(), Vec::new());
        for rule in rules {
            let known = self
                .compiled
                .0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .plans
                .get(&rule.id)
                .filter(|cached| cached.query == rule.query && cached.hosted == declared)
                .map(|cached| cached.plan.clone());
            let plan = if let Some(plan) = known {
                plan
            } else {
                let plan = self
                    .query_language
                    .compile(&rule.query, &declared)
                    .map_err(|QueryLanguageError::Refused { error }| {
                        RuleCompilationError::Query {
                            id: rule.id.0.to_string(),
                            error,
                        }
                    })?;
                let cached = CachedPlan {
                    query: rule.query.clone(),
                    hosted: declared.clone(),
                    plan: plan.clone(),
                };
                let mut kept = self
                    .compiled
                    .0
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner);
                // Past the cap the plan kept longest is dropped, whichever run it was kept for and whatever this request names: a
                // session that lints under new rule ids again and again stays bounded, and runs sharing the engine lose one plan at
                // a time, never all of theirs at once. A run holds its own plans, so a dropped one is only compiled again later.
                if kept.plans.insert(rule.id.clone(), cached).is_none() {
                    kept.order.push_back(rule.id.clone());
                }
                while kept.plans.len() > COMPILED_PLANS {
                    let Some(oldest) = kept.order.pop_front() else {
                        break;
                    };
                    drop(kept.plans.remove(&oldest));
                }
                drop(kept);
                plan
            };
            bindings.push(bind(&plan, rule)?);
            plans.push(plan);
        }
        Ok(Compiled { plans, bindings })
    }
}

/// Observe completed compilation at its existing caller-controlled phase boundary.
pub(crate) fn observed(
    rules: &[RuleSpec],
    plans: &[ryo_values::domain::plans::types::Plan],
    bindings: &[Vec<crate::domain::table_model::types::ParameterBinding>],
    hosted: &[HostRelation],
) {
    super::observing::compiled(rules, plans, bindings, hosted);
}
