//! The types of the `query` concept: RyoQL text, and a stretch of it.

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
    pub(crate) id: ryo_values::domain::program::types::QuerySourceId,
    pub(crate) text: Box<str>,
}

impl QueryModuleSource {
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

/// RyoQL text: text that is not blank. Made through `RyoQlQuery::try_from`; `query::validate` says whether it is correct.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RyoQlQuery(ryo_values::domain::program::types::QueryBundle);

// ignore: idiomatic.arch/idiomatic-rust/no-bare-text - the newtype's own text crosses only to the schemas of the SDK's entries, which check it on the way in (`query validate`'s `TryFrom`, the one place validation is allowed) and hand it to the core on the way out; the field stays private so no caller outside the crate makes an unchecked one, which a `pub(crate)` field would allow every module of it
impl RyoQlQuery {
    /// A query of `text`, which a schema has checked: what the entries' schemas build, never a caller.
    pub(crate) const fn from_bundle(
        bundle: ryo_values::domain::program::types::QueryBundle,
    ) -> Self {
        Self(bundle)
    }

    /// The immutable core bundle, handed to validation, compilation and execution.
    pub(crate) fn into_bundle(self) -> ryo_values::domain::program::types::QueryBundle {
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
