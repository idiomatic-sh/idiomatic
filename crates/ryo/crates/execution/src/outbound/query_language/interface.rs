//! What the `query_language` port asks of the outside: query text in, whether it is correct and the plan it means out.

use super::errors::QueryLanguageError;
use ryo_values::domain::descriptions::types::ValidateReport;
use ryo_values::domain::hosted::types::HostedRelation;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::program::types::ParsedModules;
use ryo_values::domain::query_tokens::types::QueryBundle;

/// What every `query_language` adapter implements.
pub trait QueryLanguagePort: Send + Sync {
    /// Checks that RyoQL text is correct: well formed, reading only relations ryo owns or the host declared, and well typed; and says
    /// what it uses.
    ///
    /// # Errors
    ///
    /// `Refused` with the reason and the place when the text is not a correct query.
    fn validate(
        &self,
        bundle: &QueryBundle,
        hosted: &[HostedRelation],
        observations: &[ryo_values::domain::observations::types::BoundObservationInput],
    ) -> Result<ValidateReport, QueryLanguageError>;

    /// Compiles RyoQL text into a plan, a query naming the relations the host declared as it names ryo's own.
    ///
    /// # Errors
    ///
    /// `Refused` with the reason when the text is not a correct query.
    fn compile(
        &self,
        bundle: &QueryBundle,
        hosted: &[HostedRelation],
        observations: &[ryo_values::domain::observations::types::BoundObservationInput],
    ) -> Result<Plan, QueryLanguageError>;

    /// `compile` inside a compilation session: `modules` holds the bundle's modules as an earlier compile of the session parsed them
    /// from the same catalog, or is empty on the first, which leaves them in it as parsed for the session to hand the next rule over
    /// that catalog. An adapter that parses each rule's modules with its root leaves it empty. The plan and every refusal are the
    /// ones `compile` gives.
    ///
    /// # Errors
    ///
    /// `Refused` with the reason when the text is not a correct query.
    fn compile_parsed(
        &self,
        bundle: &QueryBundle,
        _modules: &mut ParsedModules,
        hosted: &[HostedRelation],
        observations: &[ryo_values::domain::observations::types::BoundObservationInput],
    ) -> Result<Plan, QueryLanguageError> {
        self.compile(bundle, hosted, observations)
    }
}
