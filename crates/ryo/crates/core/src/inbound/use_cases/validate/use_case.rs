//! The `validate` use case: it checks the text through the query language, then, when a language is linked, the kinds the query
//! names against the kinds the linked languages have.

use crate::application::metadata::service::kernel_metadata;
use ryo_execution::inbound::use_cases::metadata::interface::{KindOrigin, MetadataUseCase};
use ryo_query_language::inbound::use_cases::typecheck::interface::TypecheckUseCase;
use ryo_query_language::inbound::use_cases::typecheck::service::refused;
use ryo_query_language::inbound::use_cases::typecheck::types::Checked;
use ryo_values::domain::program::types::ParsedModules;
use ryo_values::domain::query_requests::types::QueryRequest;
use std::sync::Arc;

use super::errors::ValidateError;
use super::interface::ValidateUseCase;
use super::types::ValidateRequest;
use crate::outbound::language::interface::LanguagePort;
use ryo_values::domain::descriptions::types::ValidateReport;

/// `validate`, over the query language's checks and the language port.
pub struct ValidateUseCaseImpl {
    /// Checks the text, and says what the plan that runs makes of it.
    typecheck: Arc<dyn TypecheckUseCase>,
    /// Says which kinds the linked languages have.
    language: Arc<dyn LanguagePort>,
}

impl std::fmt::Debug for ValidateUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl ValidateUseCaseImpl {
    /// `validate` over the use case and the port the edge injects.
    #[must_use]
    pub fn new(typecheck: Arc<dyn TypecheckUseCase>, language: Arc<dyn LanguagePort>) -> Self {
        Self { typecheck, language }
    }
}

impl ValidateUseCase for ValidateUseCaseImpl {
    fn validate(&self, request: ValidateRequest) -> Result<ValidateReport, ValidateError> {
        let asked = QueryRequest {
            text: request.text,
            hosted: request.hosted,
            modules: ParsedModules::default(),
            observations: request.observations,
        };
        let report = self
            .typecheck
            .typecheck(asked.clone())
            .map_err(|error| ValidateError::Refused { error: refused(error) })?;
        // A word written for a kind no linked language has is refused here as a run refuses it before it reads a file; with no
        // language linked there is no kind to check against, and the words are not asked.
        let vocabularies = self.language.kind_vocabularies();
        if vocabularies.0.is_empty() {
            return Ok(report);
        }
        let plan = match self
            .typecheck
            .check(asked)
            .map_err(|error| ValidateError::Uncompilable { error: refused(error) })?
        {
            Checked::Plan(plan) => plan,
            // The checks' plan cannot hold the query: the plan that runs has judged it, and the words are asked of the run.
            Checked::Deferred(_) => return Ok(report),
        };
        match kernel_metadata()
            .kind_refusal(
                &plan,
                &vocabularies,
                &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl::new(),
            )
            .0
        {
            Some(message) => Err(ValidateError::UnknownKind {
                reason: message.message.0.into(),
                origin: match message.origin {
                    KindOrigin::Recorded(origin) => Some(origin),
                    KindOrigin::Unrecorded => None,
                },
            }),
            None => Ok(report),
        }
    }
}
