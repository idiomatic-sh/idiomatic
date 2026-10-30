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

/// RyoQL text: text that is not blank. Made through `RyoQlQuery::try_from`; `query::validate` says whether it is correct.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RyoQlQuery(Box<str>);

// ignore: idiomatic.arch/idiomatic-rust/no-bare-text - the newtype's own text crosses only to the schemas of the SDK's entries, which check it on the way in (`query validate`'s `TryFrom`, the one place validation is allowed) and hand it to the core on the way out; the field stays private so no caller outside the crate makes an unchecked one, which a `pub(crate)` field would allow every module of it
impl RyoQlQuery {
    /// A query of `text`, which a schema has checked: what the entries' schemas build, never a caller.
    pub(crate) const fn from_text(text: Box<str>) -> Self {
        Self(text)
    }

    /// The text, handed on to the core.
    pub(crate) fn into_text(self) -> Box<str> {
        self.0
    }
}

impl std::fmt::Display for RyoQlQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for RyoQlQuery {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
