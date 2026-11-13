//! What the `compile_rules` use case promises its callers, in the core's own vocabulary.
use super::types::CompileRequest;

// The compiled rules, the shared plan cache and the refusal the contract is made of.
pub use crate::application::rule_compilation::errors::RuleCompilationError;
pub use crate::application::rule_compilation::service::CompiledPlans;
pub use crate::application::rule_compilation::service::CompiledRules;

/// What `compile_rules` offers its callers.
pub trait CompileRulesUseCase {
    /// Compiles the rules of a request, each once per shared plan cache, and refuses, before any file is read, a rule that writes
    /// a word for a kind no linked language has.
    ///
    /// # Errors
    ///
    /// Refuses invalid rule ids, compiler diagnostics, missing bindings, bad observation bindings and unknown kind words.
    fn compile_rules(
        &self,
        request: CompileRequest<'_>,
    ) -> Result<CompiledRules, RuleCompilationError>;
}
