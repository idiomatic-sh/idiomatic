//! The `validate` use case: it checks the text through the query language, then, when a language is linked, the kinds the query
//! names against the kinds the linked languages have.

use ryo_query_language::inbound::use_cases::typecheck::interface::TypecheckUseCase;
use ryo_query_language::inbound::use_cases::typecheck::service::refused;
use ryo_query_language::inbound::use_cases::typecheck::errors::TypecheckError;
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
        // A word written for a kind no linked language has is refused here as a run refuses it before it reads a file; with no
        // language linked there is no kind to check against, and the words are not asked.
        self.typecheck
            .typecheck_kinds(asked, &self.language.kind_vocabularies())
            .map_err(|error| match error {
                TypecheckError::UnknownKind { reason, origin } => ValidateError::UnknownKind { reason, origin },
                other => ValidateError::Refused { error: refused(other) },
            })
    }
}
