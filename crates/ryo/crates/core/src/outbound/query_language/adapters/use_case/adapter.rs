//! The `query_language` port, implemented by the `query_language` crate's `typecheck` and `compile` use
//! cases. The report and the plan come back in the values the core names, so the adapter passes them through.

use std::sync::Arc;

use ryo_query_language::inbound::use_cases::compile::interface::CompileUseCase;
use ryo_query_language::inbound::use_cases::compile::types::CompileRequest;
use ryo_query_language::inbound::use_cases::typecheck::interface::TypecheckUseCase;
use ryo_query_language::inbound::use_cases::typecheck::types::TypecheckRequest;

use crate::outbound::query_language::errors::QueryLanguageError;
use crate::outbound::query_language::interface::QueryLanguagePort;
use ryo_language::domain::relations::types::{Offset, Span};
use ryo_values::domain::descriptions::types::ValidateReport;
use ryo_values::domain::hosted::types as hosted;
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plans::types as plans;

use super::types::{CompileCalled, TypecheckCalled};

/// The `use_case` `query_language` adapter.
#[derive(Debug)]
pub(crate) struct QueryLanguageUseCaseAdapter {
    /// The `compile` use case of the query language, which the edge built.
    compile: CompileCalled,
    /// The `typecheck` use case of the query language, which the edge built.
    typecheck: TypecheckCalled,
}

impl QueryLanguageUseCaseAdapter {
    /// The adapter over the `compile` and `typecheck` use cases the edge built.
    pub(crate) fn new(
        compile: Arc<dyn CompileUseCase>,
        typecheck: Arc<dyn TypecheckUseCase>,
    ) -> Self {
        Self {
            compile: CompileCalled(compile),
            typecheck: TypecheckCalled(typecheck),
        }
    }
}

impl QueryLanguagePort for QueryLanguageUseCaseAdapter {
    fn validate(
        &self,
        bundle: &ryo_values::domain::program::types::QueryBundle,
        hosted: &[hosted::HostedRelation],
    ) -> Result<ValidateReport, QueryLanguageError> {
        self.typecheck
            .0
            .typecheck(TypecheckRequest {
                text: bundle.clone(),
                hosted: hosted.to_vec(),
            })
            .map_err(|error| QueryLanguageError::Refused {
                error: invalid(error),
            })
    }

    fn compile(
        &self,
        bundle: &ryo_values::domain::program::types::QueryBundle,
        hosted: &[hosted::HostedRelation],
    ) -> Result<plans::Plan, QueryLanguageError> {
        let report = self
            .compile
            .0
            .compile(CompileRequest {
                text: bundle.clone(),
                hosted: hosted.to_vec(),
            })
            .map_err(|error| QueryLanguageError::Refused {
                error: refused(error),
            })?;
        Ok(report.plan)
    }
}

/// What the compiler said of a text it refused, as the core says it.
fn refused(
    error: ryo_query_language::inbound::use_cases::compile::errors::CompileError,
) -> parts::RefusedQuery {
    use ryo_query_language::inbound::use_cases::compile::errors::CompileError;
    let span = |at: ryo_values::domain::program::types::QuerySpan| Span {
        start: offset(at.start),
        end: offset(at.end),
    };
    let nowhere = Span {
        start: Offset(0),
        end: Offset(0),
    };
    let (kind, message, span) = match error {
        CompileError::Syntax { span: at, message } => {
            (parts::RefusedKind::Syntax, message, span(at))
        }
        CompileError::UnknownRelation { name, span: at } => {
            (parts::RefusedKind::UnknownRelation, name, span(at))
        }
        CompileError::Type { span: at, message } => (parts::RefusedKind::Type, message, span(at)),
        CompileError::Invalid { span: at, message } => {
            (parts::RefusedKind::Invalid, message, span(at))
        }
        CompileError::Catalog { reason } => (parts::RefusedKind::Catalog, reason, nowhere),
    };
    parts::RefusedQuery {
        kind,
        message: parts::RefusalMessage(message),
        span,
    }
}

/// What the typechecker said of a text it refused, as the core says it.
fn invalid(
    error: ryo_query_language::inbound::use_cases::typecheck::errors::TypecheckError,
) -> parts::RefusedQuery {
    use ryo_query_language::inbound::use_cases::typecheck::errors::TypecheckError;
    let span = |at: ryo_values::domain::program::types::QuerySpan| Span {
        start: offset(at.start),
        end: offset(at.end),
    };
    let nowhere = Span {
        start: Offset(0),
        end: Offset(0),
    };
    let (kind, message, span) = match error {
        TypecheckError::Syntax { span: at, message } => {
            (parts::RefusedKind::Syntax, message, span(at))
        }
        TypecheckError::UnknownRelation { name, span: at } => {
            (parts::RefusedKind::UnknownRelation, name, span(at))
        }
        TypecheckError::Type { span: at, message } => (parts::RefusedKind::Type, message, span(at)),
        TypecheckError::Invalid { span: at, message } => {
            (parts::RefusedKind::Invalid, message, span(at))
        }
        TypecheckError::Catalog { reason } => (parts::RefusedKind::Catalog, reason, nowhere),
    };
    parts::RefusedQuery {
        kind,
        message: parts::RefusalMessage(message),
        span,
    }
}

/// A byte offset of the text, as the engine names an offset: four bytes, the largest when the text is longer.
fn offset(at: ryo_values::domain::program::types::ByteOffset) -> Offset {
    Offset(u32::try_from(at.0).unwrap_or(u32::MAX))
}
