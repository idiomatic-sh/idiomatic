//! Compile rule programs once per bounded shared session and bind every request independently.
use super::constants::COMPILED_PLANS;
use super::errors::RuleCompilationError;
use super::types::{CachedPlan, CompiledPlans};
use crate::domain::prepared_request::types::Compiled;
use crate::outbound::query_language::errors::QueryLanguageError;
use crate::outbound::query_language::interface::QueryLanguagePort;
use ryo_values::domain::hosted::service::declarations;
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::rule_requests::service::{bind, check_ids};
use ryo_values::domain::rule_requests::types::RuleSpec;
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
        let schedule = crate::domain::file_program::service::observations::schedule(rules)?;
        if let crate::domain::file_program::types::RuleSchedule::Dependencies { order, .. } =
            &schedule
        {
            let mut plans: Vec<Option<ryo_values::domain::plans::types::Plan>> =
                vec![None; rules.len()];
            let mut bindings = vec![None; rules.len()];
            let mut observations = vec![Vec::new(); rules.len()];
            for index in order {
                let rule = &rules[index.0];
                let schemas = crate::domain::file_program::service::observations::schemas(
                    rule, rules, &plans,
                )?;
                let plan = self.compile_one(rule, &declared, &schemas.schemas)?;
                observations[index.0] = schemas.bindings;
                bindings[index.0] = Some(bind(&plan, rule)?);
                plans[index.0] = Some(plan);
            }
            let plans = plans
                .into_iter()
                .map(|plan| {
                    plan.expect("topological preparation visits every original rule exactly once")
                })
                .collect();
            let bindings = bindings
                .into_iter()
                .map(|bound| bound.expect("each prepared rule has its own checked bindings"))
                .collect();
            return Ok(Compiled {
                plans,
                bindings,
                schedule,
                observations,
            });
        }
        let (mut plans, mut bindings) = (Vec::new(), Vec::new());
        for rule in rules {
            let plan = self.compile_one(rule, &declared, &[])?;
            bindings.push(bind(&plan, rule)?);
            plans.push(plan);
        }
        Ok(Compiled {
            plans,
            bindings,
            schedule,
            // Independent scheduling still declares one exact binding vector per producer rule.
            observations: vec![Vec::new(); rules.len()],
        })
    }

    /// Compile immutable declarations, then link only this request's actual observed roots.
    pub(crate) fn prepare(
        &self,
        rules: &[RuleSpec],
        hosted: &[HostRelation],
    ) -> Result<Compiled, RuleCompilationError> {
        let mut compiled = self.compile(rules, hosted)?;
        let linked = crate::domain::file_program::service::link::link(
            std::mem::take(&mut compiled.plans),
            std::mem::take(&mut compiled.observations),
        )?;
        compiled.plans = linked.plans;
        compiled.observations = linked.bindings;
        // Scheduling independence does not erase the per-rule producer declarations.
        if compiled.observations.iter().all(Vec::is_empty) {
            compiled.schedule = crate::domain::file_program::types::RuleSchedule::Independent;
        }
        Ok(compiled)
    }

    /// One immutable all-root plan, cached only under its full typed compilation contract.
    fn compile_one(
        &self,
        rule: &RuleSpec,
        declared: &[ryo_values::domain::hosted::types::HostedRelation],
        observations: &[ryo_values::domain::observations::types::BoundObservationInput],
    ) -> Result<ryo_values::domain::plans::types::Plan, RuleCompilationError> {
        if let Some(plan) = self
            .compiled
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .plans
            .get(&rule.id)
            .filter(|cached| {
                cached.query == rule.query
                    && cached.hosted == declared
                    && cached.observation_bindings == rule.observations
                    && cached.observation_inputs == observations
            })
            .map(|cached| cached.plan.clone())
        {
            return Ok(plan);
        }
        let plan = self
            .query_language
            .compile(&rule.query, declared, observations)
            .map_err(
                |QueryLanguageError::Refused { error }| RuleCompilationError::Query {
                    id: rule.id.0.to_string(),
                    error,
                },
            )?;
        self.cache_plan(rule, declared, observations, &plan);
        Ok(plan)
    }

    /// The existing bounded session owns cached declarations; a request owns its cloned plans.
    fn cache_plan(
        &self,
        rule: &RuleSpec,
        declared: &[ryo_values::domain::hosted::types::HostedRelation],
        observations: &[ryo_values::domain::observations::types::BoundObservationInput],
        plan: &ryo_values::domain::plans::types::Plan,
    ) {
        let cached = CachedPlan {
            query: rule.query.clone(),
            hosted: declared.to_vec(),
            observation_bindings: rule.observations.clone(),
            observation_inputs: observations.to_vec(),
            plan: plan.clone(),
        };
        let mut kept = self
            .compiled
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if kept.plans.insert(rule.id.clone(), cached).is_none() {
            kept.order.push_back(rule.id.clone());
        }
        while kept.plans.len() > COMPILED_PLANS {
            let Some(oldest) = kept.order.pop_front() else {
                break;
            };
            drop(kept.plans.remove(&oldest));
        }
    }
}

/// Observe completed compilation at its existing caller-controlled phase boundary.
pub(crate) fn observed(
    rules: &[RuleSpec],
    plans: &[ryo_values::domain::plans::types::Plan],
    bindings: &[Vec<ryo_values::domain::query_bindings::types::ParameterBinding>],
    hosted: &[HostRelation],
) {
    super::observing::compiled(rules, plans, bindings, hosted);
}
