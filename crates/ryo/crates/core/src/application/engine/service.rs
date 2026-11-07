//! The engine's compilation and execution as the use cases call them: the kernel's implementations, held here where an application
//! service may name them, so a use case builds none.

use ryo_execution::inbound::use_cases::execution::interface::ExecutionUseCase as _;
use super::types::KindRefused;
use crate::outbound::language::interface::LanguagePort;
use ryo_execution::inbound::use_cases::metadata::interface::{MetadataUseCase as _, RuleIndex};
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use std::sync::Arc;

use ryo_execution::inbound::use_cases::execution::interface::PreparedExecutionProgram;
use ryo_execution::inbound::use_cases::execution::interface::RequestExecution;
use ryo_execution::inbound::use_cases::prepare::interface::RuleCompilationError;
use ryo_execution::inbound::use_cases::prepare::interface::{
    CompiledPlans, CompiledQueries, ExecutionCompilation, PreparedQueries as _, QueryCompilation,
};
use ryo_execution::inbound::use_cases::prepare::interface::PreparedRuleIdentities;
use ryo_execution::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use ryo_execution::inbound::use_cases::prepare::interface::PrepareUseCase;
use ryo_execution::inbound::use_cases::prepare::types::PrepareRequest;
use ryo_execution::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_execution::outbound::derived::interface::DerivedPort;
use ryo_execution::outbound::digest::interface::DigestPort;
use ryo_execution::outbound::operators::interface::OperatorsPort;
use ryo_execution::outbound::query_language::interface::QueryLanguagePort;
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;

/// The engine's rule compilation, with the plans it keeps.
pub struct Compilation(QueryCompilation);

impl std::fmt::Debug for Compilation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl Compilation {
    /// Compilation over the query language the edge injects.
    #[must_use]
    pub fn new(compiler: Arc<dyn QueryLanguagePort>) -> Self {
        Self(QueryCompilation::new(compiler))
    }
}

impl Compilation {
    /// The rules compiled, and refused before any file is read when one writes a word for a kind no linked language has: the
    /// inner answer is that refusal, the outer one the compilation's.
    pub(crate) fn checked(
        &self,
        rules: &[ryo_values::domain::rule_requests::types::RuleSpec],
        hosted: &[ryo_values::domain::hosted::types::HostRelation],
        digest: &dyn DigestPort,
        language: &dyn LanguagePort,
    ) -> Result<Result<CompiledQueries, KindRefused>, RuleCompilationError> {
        let compiled = self.0.compile(rules, hosted, digest)?;
        // Rules that write no word for a kind need no vocabulary.
        if compiled
            .plans()
            .iter()
            .zip(compiled.bindings())
            .all(|(plan, bound)| MetadataUseCaseImpl.kinds_asked(plan, bound).is_empty())
        {
            return Ok(Ok(compiled));
        }
        let vocabularies = language.kind_vocabularies();
        Ok(match MetadataUseCaseImpl.first_unknown(
            compiled.plans(),
            compiled.bindings(),
            &vocabularies,
            &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl::new(),
        ) {
            Some(unknown) => Err(KindRefused {
                rule: RuleIndex(unknown.rule.0),
                reason: MetadataUseCaseImpl
                    .unknown_kind_message(&unknown.word, &vocabularies)
                    .message,
            }),
            None => Ok(compiled),
        })
    }
}

impl ExecutionCompilation for Compilation {
    fn compiling_into(self, cache: CompiledPlans) -> Self {
        Self(self.0.compiling_into(cache))
    }

    fn compile(
        &self,
        rules: &[ryo_values::domain::rule_requests::types::RuleSpec],
        hosted: &[ryo_values::domain::hosted::types::HostRelation],
        digest: &dyn DigestPort,
    ) -> Result<CompiledQueries, RuleCompilationError> {
        self.0.compile(rules, hosted, digest)
    }

    fn identify(
        &self,
        queries: &CompiledQueries,
        digest: &dyn DigestPort,
        rules: Vec<ryo_values::domain::stores::types::StoredRule>,
    ) -> Result<PreparedRuleIdentities, RuleCompilationError> {
        self.0.identify(queries, digest, rules)
    }
}

/// The engine's preparation of a request and the beginning of its execution.
pub struct Execution(PrepareUseCaseImpl);

impl std::fmt::Debug for Execution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl Execution {
    /// Execution over the operators and the derived views the edge injects.
    #[must_use]
    pub fn new(operators: Arc<dyn OperatorsPort>, derived: Arc<dyn DerivedPort>) -> Self {
        Self(PrepareUseCaseImpl::new(operators, derived))
    }

    /// The same, wiring only the strategies `routes` names, for a differential run.
    #[cfg(feature = "fixtures")]
    #[must_use]
    pub fn routing(
        self,
        routes: ryo_execution::inbound::use_cases::prepare::interface::RoutePolicy,
    ) -> Self {
        Self(self.0.routing(routes))
    }

    /// Begins the execution of a prepared program over the corpus it runs on.
    #[must_use]
    pub fn begin(
        &self,
        program: &PreparedExecutionProgram,
        corpus: &[ryo_values::domain::files::types::CorpusInput],
    ) -> RequestExecution {
        ExecutionUseCaseImpl::new().execution(program, corpus)
    }
}

impl PrepareUseCase for Execution {
    fn prepare(&self, request: PrepareRequest) -> Result<PreparedExecutionProgram, OperatingError> {
        self.0.prepare(request)
    }
}
