//! The types of the `query` concept: RyoQL text, and a stretch of it.

use ryo_values::domain::query_tokens::types::{QueryDocumentSource, QuerySourceId};
use std::{collections::BTreeMap, sync::Arc};

/// A place in the query text, counted in bytes from its start.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ByteOffset(pub usize);

/// A stretch of the query text, in bytes: not a stretch of a source file, which is a language's `Span`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuerySpan {
    /// Where it starts.
    pub start: ByteOffset,
    /// Where it ends.
    pub end: ByteOffset,
}

/// A named query module source that the compiler receives in memory.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QueryModuleSource {
    /// The validated logical identity used by imports in the bundle.
    id: QuerySourceId,
    /// Exact module source bytes interpreted as UTF-8 text.
    text: Arc<str>,
}

impl QueryModuleSource {
    /// Construct a module value from already checked components.
    pub(crate) const fn from_parts(id: QuerySourceId, text: Arc<str>) -> Self {
        Self { id, text }
    }

    /// Transfer already checked identity and shared bytes into the neutral catalog.
    pub(crate) fn into_document(self) -> QueryDocumentSource {
        QueryDocumentSource {
            id: self.id,
            text: self.text.into(),
        }
    }

    /// The stable logical module id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id.0
    }

    /// The exact source text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// One checked immutable module catalog, reusable by any number of root queries.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QueryModuleCatalog(Arc<BTreeMap<QuerySourceId, QueryDocumentSource>>);

impl QueryModuleCatalog {
    /// Construct from documents checked at the SDK schema boundary.
    pub(crate) const fn from_catalog(
        catalog: Arc<BTreeMap<QuerySourceId, QueryDocumentSource>>,
    ) -> Self {
        Self(catalog)
    }

    /// Borrow the checked immutable catalog for a root's checked construction.
    pub(crate) fn catalog(&self) -> &Arc<BTreeMap<QuerySourceId, QueryDocumentSource>> {
        &self.0
    }

    /// Whether this catalog declares no modules.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// RyoQL text: text that is not blank. Made through `RyoQlQuery::try_from`; `query::validate` says whether it is correct.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RyoQlQuery(ryo_values::domain::query_tokens::types::QueryBundle);

// the newtype's own text crosses only to the schemas of the SDK's entries, which check it on the way in (`query validate`'s `TryFrom`, the one place validation is allowed) and hand it to the core on the way out; the field stays private so no caller outside the crate makes an unchecked one, which a `pub(crate)` field would allow every module of it
impl RyoQlQuery {
    /// A query of `text`, which a schema has checked: what the entries' schemas build, never a caller.
    pub(crate) const fn from_bundle(
        bundle: ryo_values::domain::query_tokens::types::QueryBundle,
    ) -> Self {
        Self(bundle)
    }

    /// The immutable core bundle, handed to validation, compilation and execution.
    pub(crate) fn into_bundle(self) -> ryo_values::domain::query_tokens::types::QueryBundle {
        self.0
    }

    /// The root query text, preserving the standalone API's view.
    #[must_use]
    pub fn root_text(&self) -> &str {
        ryo_values::domain::program::service::root_text(&self.0)
    }
}

impl std::fmt::Display for RyoQlQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.root_text())
    }
}

impl AsRef<str> for RyoQlQuery {
    fn as_ref(&self) -> &str {
        self.root_text()
    }
}
