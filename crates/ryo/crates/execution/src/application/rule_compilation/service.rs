//! Compile rule programs once per bounded shared session and bind every request independently.
use super::constants::{
    COMPILED_PLANS, EVERY_RULE_BOUND, EVERY_RULE_PLANNED, RULE_COMPILATION_NAME,
};
use super::errors::RuleCompilationError;
use super::types::PreparedRuleIdentities;
use super::types::{CachedPlan, PlanCache};
use crate::outbound::query_language::errors::QueryLanguageError;
use crate::outbound::query_language::interface::QueryLanguagePort;
use ryo_execution_domain::domain::prepared_request::types::Compiled;
use ryo_execution_runtime::outbound::digest::interface::DigestPort;
use ryo_values::domain::hosted::service::declarations;
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::rule_requests::service::{bind, check_ids};
use ryo_values::domain::rule_requests::types::RuleSpec;
use std::sync::{Arc, Mutex, PoisonError};

/// The plan compiled so far for each rule id, shared by every use case built over them: a program that lints many projects with the
/// same rules compiles each rule once. A rule id keeps one plan: the one of its latest query; past `COMPILED_PLANS` the plan that
/// has been kept longest is dropped, whatever the request in progress names: a run holds the plans it compiled or found, so
/// dropping one never changes an answer.
#[derive(Clone, Debug, Default)]
pub struct CompiledPlans(pub(super) Arc<Mutex<PlanCache>>);

/// The rules of a request compiled: their plans and bindings may be read and their stored identities named, nothing else of the
/// compilation.
#[derive(Debug)]
pub struct CompiledRules {
    pub(crate) inner: Compiled,
}

impl CompiledRules {
    /// The plans, in the order of the rules. They are shared with the compilation's cache, never copied out of it.
    pub fn plans(&self) -> impl ExactSizeIterator<Item = &ryo_values::domain::plans::types::Plan> {
        self.inner.plans.iter().map(AsRef::as_ref)
    }
    /// What each rule binds its parameters to, in the order of the plan's parameters.
    #[must_use]
    pub fn bindings(&self) -> &[Vec<ryo_values::domain::query_bindings::types::ParameterBinding>] {
        &self.inner.bindings
    }
    /// The identity of each stored rule and the observation layout of the request, from the plans and producers compiled.
    ///
    /// # Errors
    ///
    /// Refuses rules whose producers do not agree with the compiled schedule.
    pub fn identify(
        &self,
        digest: &dyn DigestPort,
        rules: Vec<ryo_values::domain::stores::types::StoredRule>,
    ) -> Result<PreparedRuleIdentities, RuleCompilationError> {
        super::identity::service::with_producers(
            digest,
            rules,
            &self.inner.plans,
            &self.inner.schedule,
            &self.inner.observations,
        )
    }
}

/// One cohesive compiler operation, sharing only bounded immutable plans across invocations.
pub(crate) struct RuleCompilation {
    /// The injected query compiler; compilation never holds the cache lock.
    query_language: Arc<dyn QueryLanguagePort>,
    /// Session-owned plan state; a run retains its own cloned plans after eviction.
    compiled: CompiledPlans,
}

impl std::fmt::Debug for RuleCompilation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(RULE_COMPILATION_NAME)
            .finish_non_exhaustive()
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
        let schedule =
            ryo_execution_domain::domain::observation_schedule::service::schedule(rules)?;
        if let ryo_execution_domain::domain::observation_schedule::types::RuleSchedule::Dependencies {
            order,
            ..
        } = &schedule
        {
            let mut plans: Vec<Option<Arc<ryo_values::domain::plans::types::Plan>>> =
                vec![None; rules.len()];
            let mut bindings = vec![None; rules.len()];
            let mut observations = vec![Vec::new(); rules.len()];
            for index in order {
                let rule = &rules[index.0];
                let schemas =
                    ryo_execution_domain::domain::observation_schedule::service::schemas(
                        rule, rules, &plans,
                    )?;
                let plan = self.compile_one(rule, &declared, &schemas.schemas)?;
                observations[index.0] = schemas.bindings;
                bindings[index.0] = Some(bind(&plan, rule)?);
                plans[index.0] = Some(plan);
            }
            #[expect(
                clippy::expect_used,
                reason = "schedule refuses a cycle and otherwise orders every rule exactly once, so the loop above fills every slot"
            )]
            let plans = plans
                .into_iter()
                .map(|plan| plan.expect(EVERY_RULE_PLANNED))
                .collect();
            #[expect(
                clippy::expect_used,
                reason = "schedule refuses a cycle and otherwise orders every rule exactly once, so the loop above fills every slot"
            )]
            let bindings = bindings
                .into_iter()
                .map(|bound| bound.expect(EVERY_RULE_BOUND))
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
        let linked =
            ryo_execution_domain::domain::observation_schedule::service::link::service::link(
                std::mem::take(&mut compiled.plans),
                std::mem::take(&mut compiled.observations),
            )?;
        compiled.plans = linked.plans;
        compiled.observations = linked.bindings;
        // Scheduling independence does not erase the per-rule producer declarations.
        if compiled.observations.iter().all(Vec::is_empty) {
            compiled.schedule =
                ryo_execution_domain::domain::observation_schedule::types::RuleSchedule::Independent;
        }
        Ok(compiled)
    }

    /// One immutable all-root plan, cached only under its full typed compilation contract.
    fn compile_one(
        &self,
        rule: &RuleSpec,
        declared: &[ryo_values::domain::hosted::types::HostedRelation],
        observations: &[ryo_values::domain::observations::types::BoundObservationInput],
    ) -> Result<Arc<ryo_values::domain::plans::types::Plan>, RuleCompilationError> {
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
            .map(|cached| Arc::clone(&cached.plan))
        {
            return Ok(plan);
        }
        let mut modules = self.held_modules(&rule.query);
        let first = modules.0.is_empty();
        let plan = self
            .query_language
            .compile_parsed(&rule.query, &mut modules, declared, observations)
            .map_err(
                |QueryLanguageError::Refused { error }| RuleCompilationError::Query {
                    id: rule.id.0.to_string(),
                    error,
                },
            )?;
        if first && !modules.0.is_empty() {
            let mut kept = self
                .compiled
                .0
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            kept.catalog = ryo_values::domain::query_tokens::types::ModuleCatalog(Arc::clone(
                &rule.query.modules,
            ));
            kept.parsed = modules;
        }
        let plan = Arc::new(plan);
        self.cache_plan(rule, declared, observations, &plan);
        Ok(plan)
    }

    /// The modules of the bundle's catalog as this session parsed them, when an earlier rule over the same catalog compiled: empty on
    /// the first, which parses them with its root and refuses as a lone compile does, so a module that does not parse refuses the run.
    fn held_modules(
        &self,
        bundle: &ryo_values::domain::query_tokens::types::QueryBundle,
    ) -> ryo_values::domain::program::types::ParsedModules {
        let kept = self
            .compiled
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if kept.catalog.0 == bundle.modules {
            kept.parsed.clone()
        } else {
            ryo_values::domain::program::types::ParsedModules::default()
        }
    }

    /// The existing bounded session owns cached declarations; a request owns its cloned plans.
    fn cache_plan(
        &self,
        rule: &RuleSpec,
        declared: &[ryo_values::domain::hosted::types::HostedRelation],
        observations: &[ryo_values::domain::observations::types::BoundObservationInput],
        plan: &Arc<ryo_values::domain::plans::types::Plan>,
    ) {
        let cached = CachedPlan {
            query: rule.query.clone(),
            hosted: declared.to_vec(),
            observation_bindings: rule.observations.clone(),
            observation_inputs: observations.to_vec(),
            plan: Arc::clone(plan),
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
    plans: &[Arc<ryo_values::domain::plans::types::Plan>],
    bindings: &[Vec<ryo_values::domain::query_bindings::types::ParameterBinding>],
    hosted: &[HostRelation],
    digest: &dyn DigestPort,
) {
    super::observing::service::compiled(rules, plans, bindings, hosted, digest);
}

/// The rule at `place` of the request, or the refusal for a place the compiler reported past its rules.
pub(crate) fn named(
    rules: &[RuleSpec],
    place: ryo_execution_domain::domain::kind_check::types::RulePlace,
) -> Result<&RuleSpec, RuleCompilationError> {
    rules
        .get(place.0)
        .ok_or(RuleCompilationError::UnknownRuleIndex {
            index: place.0,
            rules: rules.len(),
        })
}
