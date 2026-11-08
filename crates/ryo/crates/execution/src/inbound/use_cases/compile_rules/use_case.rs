//! Thin public orchestration over the private rule compilation.
use super::interface::{CompileRulesUseCase, CompiledRules, RuleCompilationError, CompiledPlans};
use super::types::CompileRequest;
use crate::application::rule_compilation::service::{RuleCompilation, observed};
use crate::outbound::query_language::interface::QueryLanguagePort;
use ryo_execution_domain::domain::kind_check::service::{first_unknown, kinds_asked, unknown_message};
use ryo_execution_domain::domain::matching::types::PatternMatcher;
use ryo_execution_runtime::outbound::digest::interface::DigestPort;
use std::sync::Arc;

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
    fn compile_rules(&self, request: CompileRequest<'_>) -> Result<CompiledRules, RuleCompilationError> {
        let CompileRequest {
            rules,
            hosted,
            vocabularies,
        } = request;
        let inner = self.compilation.prepare(rules, hosted)?;
        observed(rules, &inner.plans, &inner.bindings, hosted, self.digest.as_ref());
        // Rules that write no word for a kind need no vocabulary.
        if inner
            .plans
            .iter()
            .zip(&inner.bindings)
            .all(|(plan, bound)| kinds_asked(plan, bound).is_empty())
        {
            return Ok(CompiledRules { inner });
        }
        let matching = ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl::new();
        match first_unknown(&inner.plans, &inner.bindings, vocabularies, PatternMatcher(&matching)) {
            Some(unknown) => Err(RuleCompilationError::UnknownKind {
                id: rules
                    .get(unknown.rule.0)
                    .map(|rule| rule.id.clone())
                    .unwrap_or_default(),
                reason: unknown_message(&unknown.word, vocabularies).message,
            }),
            None => Ok(CompiledRules { inner }),
        }
    }
}
