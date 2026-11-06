//! The `linking` service: free functions named by verb over the located refusals of linking and the source ids it reads.

use super::errors::LinkingError;
use ryo_values::domain::program::types::ImportDeclaration;
use ryo_values::domain::query_tokens::types::QueryOrigin;
use ryo_values::domain::query_tokens::types::QuerySourceId;
use ryo_values::domain::query_tokens::types::QuerySpan;
use ryo_values::domain::query_tokens::types::QueryTraceSite;

/// Resolves the target module identifier written by an import.
pub(crate) fn source_id(
    source: &QuerySourceId,
    import: &ImportDeclaration,
) -> Result<QuerySourceId, LinkingError> {
    ryo_values::domain::program::service::source_id(
        ryo_values::domain::query_tokens::types::QuerySpelling(&import.module.0),
    )
    .map_err(|error| {
        located(
            source,
            import.span,
            Vec::new(),
            format!("invalid module id: {error}"),
        )
    })
}

/// Records one source-to-target import location in a diagnostic trace.
pub(crate) fn trace_site(
    source: &QuerySourceId,
    target: &QuerySourceId,
    span: QuerySpan,
) -> QueryTraceSite {
    QueryTraceSite {
        source_id: source.clone(),
        target_id: target.clone(),
        span,
    }
}

/// Builds a link error with its source and import trace.
pub(crate) fn located(
    source_id: &QuerySourceId,
    span: QuerySpan,
    trace: Vec<QueryTraceSite>,
    message: impl std::fmt::Display,
) -> LinkingError {
    LinkingError::Located {
        origin: QueryOrigin {
            source_id: source_id.clone(),
            span,
            trace,
        },
        message: message.to_string(),
    }
}
