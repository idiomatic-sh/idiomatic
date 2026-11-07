//! The `validate` use case: it checks the text through the query language, then, when a language is linked, the kinds the query
//! names against the kinds the linked languages have.

use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use std::sync::Arc;

use super::errors::ValidateError;
use super::interface::ValidateUseCase;
use super::types::ValidateRequest;
use crate::outbound::language::interface::LanguagePort;
use ryo_execution::outbound::query_language::errors::QueryLanguageError;
use ryo_execution::outbound::query_language::interface::QueryLanguagePort;
use ryo_values::domain::descriptions::types::ValidateReport;

/// `validate`, over the query language and the language port.
pub struct ValidateUseCaseImpl {
    /// Checks the text and compiles it.
    query_language: Arc<dyn QueryLanguagePort>,
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
    /// `validate` over the ports the edge injects.
    #[must_use]
    pub fn new(
        query_language: Arc<dyn QueryLanguagePort>,
        language: Arc<dyn LanguagePort>,
    ) -> Self {
        Self {
            query_language,
            language,
        }
    }
}

impl ValidateUseCase for ValidateUseCaseImpl {
    fn validate(&self, request: ValidateRequest) -> Result<ValidateReport, ValidateError> {
        let report = self
            .query_language
            .validate(&request.text, &request.hosted, &request.observations)
            .map_err(|error| match error {
                QueryLanguageError::Refused { error } => ValidateError::Refused { error },
            })?;
        // A word written for a kind no linked language has is refused here as a run refuses it before it reads a file; with no
        // language linked there is no kind to check against, and the words are not asked.
        let vocabularies = self.language.kind_vocabularies();
        if vocabularies.0.is_empty() {
            return Ok(report);
        }
        let plan = self
            .query_language
            .compile(&request.text, &request.hosted, &request.observations)
            .map_err(|error| match error {
                QueryLanguageError::Refused { error } => ValidateError::Uncompilable { error },
            })?;
        match MetadataUseCaseImpl.kind_refusal(&plan, &vocabularies) {
            Some(message) => Err(ValidateError::UnknownKind {
                reason: message.0.into(),
                origin: message.1,
            }),
            None => Ok(report),
        }
    }
}
