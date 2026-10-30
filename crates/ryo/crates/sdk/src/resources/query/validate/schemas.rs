//! The shape of the data this entry does not control: each wire shape is a `<Name>Schema`, its checked conversion into the core's type is an `impl TryFrom<<Name>Schema>`, and each refusal of a conversion is a `<Name>SchemaError`.

use ryo_core::inbound::use_cases::validate::errors::ValidateError as CoreError;
use ryo_core::inbound::use_cases::validate::types::ValidateRequest as CoreRequest;
use ryo_language::domain::relations::types::Span as CoreSpan;
use ryo_values::domain::descriptions::types::ValidateReport as CoreReport;
use ryo_values::domain::plan_parts::types::{QueryText, RefusedKind, RefusedQuery};

use ryo_language::domain::relations::types::CellKind;
use ryo_values::domain::hosted::types::{
    ColumnLabel as CoreColumnLabel, HostedColumn as CoreHostedColumn, HostedName as CoreHostedName,
    HostedRelation as CoreHosted,
};

use super::types::{StepCount, ValidateReport, ValidateRequest};
use crate::domain::host::types::{HostDeclaration, HostKind};
use crate::domain::operator::types::OperatorName;
use crate::domain::query::errors::ValidateError;
use crate::domain::query::types::{ByteOffset, QuerySpan, RyoQlQuery};
use crate::domain::relation::types::RelationName;

/// Why a text is not a query.
#[expect(
    missing_copy_implementations,
    reason = "adding `Copy` breaks the SDK against its semver baseline (cargo-semver-checks `copy_impl_added`): it comes with the SDK's next version"
)]
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("a query is not blank")]
pub struct RyoQlQuerySchemaError;

impl TryFrom<&str> for RyoQlQuery {
    type Error = RyoQlQuerySchemaError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        if text.trim().is_empty() {
            return Err(RyoQlQuerySchemaError);
        }
        Ok(Self::from_text(text.into()))
    }
}

/// The relations the host declares, as the query language is told of them.
fn hosted(declared: &[HostDeclaration]) -> Vec<CoreHosted> {
    declared
        .iter()
        .map(|each| CoreHosted {
            name: CoreHostedName(each.name.0.clone()),
            columns: each
                .columns
                .iter()
                .map(|column| CoreHostedColumn {
                    name: CoreColumnLabel(column.name.0.clone()),
                    kind: match column.kind {
                        HostKind::File => CellKind::File,
                        HostKind::Text => CellKind::Symbol,
                        HostKind::Number => CellKind::Number,
                    },
                })
                .collect(),
        })
        .collect()
}

/// The failure of a query that is correct but writes a word for a kind no linked language has, or that cannot be compiled after it
/// was found correct: it is wrong as a whole, so the span is the text.
fn invalid(text: &str, message: String) -> ValidateError {
    ValidateError::Invalid {
        span: QuerySpan {
            start: ByteOffset(0),
            end: ByteOffset(text.len()),
        },
        message,
    }
}

/// A core span, offered to the conversion.
struct SpanSchema(CoreSpan);

/// A core failure of validating `text`, offered to the conversion.
struct ValidateErrorSchema<'text> {
    /// The query text that was validated.
    text: &'text str,
    /// What the core said of it.
    error: CoreError,
}

/// A core report, offered to the conversion.
struct ValidateReportSchema(CoreReport);

impl From<SpanSchema> for QuerySpan {
    fn from(schema: SpanSchema) -> Self {
        let span = schema.0;
        Self {
            start: ByteOffset(usize::try_from(span.start.0).unwrap_or(usize::MAX)),
            end: ByteOffset(usize::try_from(span.end.0).unwrap_or(usize::MAX)),
        }
    }
}

/// A refusal of the text, by the fault it names and where it is.
fn refused(error: RefusedQuery) -> ValidateError {
    let span = SpanSchema(error.span).into();
    let message = error.message.0;
    match error.kind {
        RefusedKind::Syntax => ValidateError::Syntax { span, message },
        RefusedKind::UnknownRelation => ValidateError::UnknownRelation {
            name: message,
            span,
        },
        RefusedKind::Type => ValidateError::Type { span, message },
        RefusedKind::Invalid => ValidateError::Invalid { span, message },
        RefusedKind::Catalog => ValidateError::Catalog { reason: message },
    }
}

impl From<ValidateErrorSchema<'_>> for ValidateError {
    fn from(schema: ValidateErrorSchema<'_>) -> Self {
        match schema.error {
            CoreError::Refused { error } => refused(error),
            CoreError::Uncompilable { error } => invalid(schema.text, error.to_string()),
            CoreError::UnknownKind { reason } => invalid(schema.text, reason),
        }
    }
}

impl From<ValidateReportSchema> for ValidateReport {
    fn from(schema: ValidateReportSchema) -> Self {
        let report = schema.0;
        Self {
            relations: report
                .relations
                .into_iter()
                .map(|name| RelationName(name.0))
                .collect(),
            operators: report
                .operators
                .into_iter()
                .map(|name| OperatorName(name.0))
                .collect(),
            steps: StepCount(report.steps.0),
        }
    }
}

/// The core's request for the caller's: the query's text and the relations the caller declares.
pub(super) fn request(request: ValidateRequest) -> CoreRequest {
    CoreRequest {
        hosted: hosted(&request.hosted),
        text: QueryText(request.query.into_text()),
    }
}

/// The caller's failure of validating `text` for the core's.
pub(super) fn error(text: &str, error: CoreError) -> ValidateError {
    ValidateErrorSchema { text, error }.into()
}

/// The caller's report for the core's.
pub(super) fn report(report: CoreReport) -> ValidateReport {
    ValidateReportSchema(report).into()
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
