//! Thin public orchestration over the private rule compilation.
use super::interface::{CompileRulesUseCase, CompiledPlans, CompiledRules, RuleCompilationError};
use super::types::CompileRequest;
use crate::application::rule_compilation::service::{RuleCompilation, named, observed};
use crate::outbound::query_language::interface::QueryLanguagePort;
use ryo_execution_domain::domain::kind_check::service::{
    first_unknown, kinds_asked, unknown_message,
};
use ryo_execution_runtime::outbound::digest::interface::DigestPort;
use ryo_operators::domain::patterns::types::PatternMatcher;
use std::sync::Arc;

/// Spike (ryo-v1np): the per-rule plans of one compile, for a standalone engine prototype.
#[derive(Debug)]
pub struct TappedPlans {
    /// Rule ids in rule order.
    pub ids: Vec<String>,
    /// Plans in rule order.
    pub plans: Vec<std::sync::Arc<ryo_values::domain::plans::types::Plan>>,
    /// Bindings in rule order.
    pub bindings: Vec<Vec<ryo_values::domain::query_bindings::types::ParameterBinding>>,
}
/// Spike (ryo-v1np): the panic payload that stops a tapped run.
#[derive(Debug)]
pub struct TappedPlansStop;
/// Spike (ryo-v1np): armed only by the bake-off bench.
pub static PLAN_TAP: std::sync::OnceLock<std::sync::Mutex<Option<TappedPlans>>> = std::sync::OnceLock::new();

/// Compiles rules over the query compiler and digest it is given, keeping their plans in a bounded cache.
pub struct CompileRulesUseCaseImpl {
    compilation: RuleCompilation,
    digest: Arc<dyn DigestPort>,
}

impl std::fmt::Debug for CompileRulesUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl CompileRulesUseCaseImpl {
    /// A use case over the query compiler and the digest, with a plan cache of its own.
    #[must_use]
    pub fn new(query_language: Arc<dyn QueryLanguagePort>, digest: Arc<dyn DigestPort>) -> Self {
        Self {
            compilation: RuleCompilation::new(query_language),
            digest,
        }
    }

    /// The same, keeping the plans it compiles in `plans`, which other use cases share.
    #[must_use]
    pub fn compiling_into(mut self, plans: CompiledPlans) -> Self {
        self.compilation = self.compilation.compiling_into(plans);
        self
    }
}

impl CompileRulesUseCase for CompileRulesUseCaseImpl {
    fn compile_rules(
        &self,
        request: CompileRequest<'_>,
    ) -> Result<CompiledRules, RuleCompilationError> {
        let CompileRequest {
            rules,
            hosted,
            vocabularies,
        } = request;
        let inner = self.compilation.prepare(rules, hosted)?;
        // Spike (ryo-v1np): a standalone engine bake-off takes the per-rule plans here and stops the run; nothing is tapped unless
        // its bench armed the tap.
        if let Some(tap) = PLAN_TAP.get() {
            let mut slot = tap.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            *slot = Some(TappedPlans {
                ids: rules.iter().map(|rule| rule.id.0.to_string()).collect(),
                plans: inner.plans.clone(),
                bindings: inner.bindings.clone(),
            });
            drop(slot);
            std::panic::panic_any(TappedPlansStop);
        }
        observed(
            rules,
            &inner.plans,
            &inner.bindings,
            hosted,
            self.digest.as_ref(),
        );
        // Rules that write no word for a kind need no vocabulary.
        if inner
            .plans
            .iter()
            .zip(&inner.bindings)
            .all(|(plan, bound)| kinds_asked(plan, bound).is_empty())
        {
            return Ok(CompiledRules { inner });
        }
        let matching =
            ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl::new();
        match first_unknown(
            &inner.plans,
            &inner.bindings,
            vocabularies,
            PatternMatcher(&matching),
        ) {
            Some(unknown) => {
                let rule = named(rules, unknown.rule)?;
                Err(RuleCompilationError::UnknownKind {
                    id: rule.id.clone(),
                    reason: unknown_message(&unknown.word, vocabularies).message,
                })
            }
            None => Ok(CompiledRules { inner }),
        }
    }
}
