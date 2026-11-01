//! The errors of the `query` concept: why RyoQL text is not correct, which `query validate` answers and a lint's rule carries.

use super::types::QuerySpan;

/// A logical source location in a query bundle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryOrigin {
    /// The root or imported module that owns the source span.
    pub source_id: String,
    /// The byte range in that source.
    pub span: QuerySpan,
    /// The import and fragment-call path from the root to this source.
    pub trace: Vec<QueryTraceSite>,
}

/// One import or fragment-call edge in a query origin trace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryTraceSite {
    /// The source containing the import or call.
    pub source_id: String,
    /// The imported module or called fragment.
    pub target_id: String,
    /// The byte range of the import or call.
    pub span: QuerySpan,
}

impl QueryOrigin {
    pub(crate) fn from_internal(origin: ryo_values::domain::program::types::QueryOrigin) -> Self {
        Self {
            source_id: origin.source_id.0.into(),
            span: QuerySpan {
                start: super::types::ByteOffset(origin.span.start.0),
                end: super::types::ByteOffset(origin.span.end.0),
            },
            trace: origin
                .trace
                .into_iter()
                .map(|site| QueryTraceSite {
                    source_id: site.source_id.0.into(),
                    target_id: site.target_id.0.into(),
                    span: QuerySpan {
                        start: super::types::ByteOffset(site.span.start.0),
                        end: super::types::ByteOffset(site.span.end.0),
                    },
                })
                .collect(),
        }
    }
}

/// Why an SDK query bundle could not be constructed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum QueryBundleError {
    /// A source identity is blank.
    #[error("query source id is blank")]
    BlankSourceId,
    /// The root query text is blank.
    #[error("root query text is blank")]
    BlankRootText,
    /// Two sources use the same identity, or a module duplicates the root identity.
    #[error("query source id `{id}` is duplicated")]
    DuplicateSourceId {
        /// The repeated logical identity.
        id: String,
    },
}

impl QueryBundleError {
    pub(crate) fn from_internal(
        error: ryo_values::domain::program::errors::QueryBundleError,
    ) -> Self {
        use ryo_values::domain::program::errors::QueryBundleError as Internal;
        match error {
            Internal::BlankSourceId => Self::BlankSourceId,
            Internal::BlankRootText => Self::BlankRootText,
            Internal::DuplicateSourceId { id } => Self::DuplicateSourceId { id: id.0.into() },
        }
    }
}

/// Why `validate` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ValidateError {
    /// A refusal in an imported module, attributed to its original source and trace.
    #[error(
        "{error} in query source `{source}` at byte {offset}",
        source = origin.source_id,
        offset = origin.span.start.0
    )]
    Located {
        /// The original source and import/call path.
        origin: QueryOrigin,
        /// The underlying refusal.
        error: Box<ValidateError>,
    },
    /// The text is not RyoQL.
    #[error("{message} at byte {}", span.start.0)]
    Syntax {
        /// Where the text stops making sense.
        span: QuerySpan,
        /// What was expected there.
        message: String,
    },
    /// The query reads a relation ryo does not own.
    #[error("no relation named `{name}`")]
    UnknownRelation {
        /// The relation's name.
        name: String,
        /// Where the query reads it.
        span: QuerySpan,
    },
    /// A comparison puts unlike types side by side.
    #[error("{message}")]
    Type {
        /// Where the comparison is.
        span: QuerySpan,
        /// Which types.
        message: String,
    },
    /// The query is wrong in another way: an unknown step, parameter or operation, a missing argument.
    #[error("{message}")]
    Invalid {
        /// Where.
        span: QuerySpan,
        /// What is wrong.
        message: String,
    },
    /// The catalog of relations could not be read.
    #[error("the relation catalog failed: {reason}")]
    Catalog {
        /// What the catalog said.
        reason: String,
    },
}
