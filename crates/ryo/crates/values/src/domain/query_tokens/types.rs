//! The types of the `query_tokens` concept: the spans, names and sources RyoQL text is written from, and where a node came from. Plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::plan_parts::types::QueryText;
use std::collections::BTreeMap;

/// A place in the query text, counted in bytes from its start.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteOffset(pub usize);

/// A stretch of the query text, in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QuerySpan {
    /// Where it starts.
    pub start: ByteOffset,
    /// Where it ends.
    pub end: ByteOffset,
}

/// RyoQL text as the port reads it: borrowed from the host's query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueryView<'a>(pub &'a str);

/// Stable logical identity of one source in a query bundle.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QuerySourceId(pub Box<str>);

/// One exact source in a query bundle, with its logical identity retained for diagnostics.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QueryDocumentSource {
    /// The stable logical identity used by imports and diagnostics.
    pub id: QuerySourceId,
    /// Exact immutable UTF-8 bytes, shared by snapshots without copying source text.
    pub text: QuerySourceText,
}

/// The exact text of one query source, shared by snapshots without copying.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QuerySourceText(pub std::sync::Arc<str>);

impl std::ops::Deref for QuerySourceText {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl From<&str> for QuerySourceText {
    fn from(text: &str) -> Self {
        Self(text.into())
    }
}

impl From<String> for QuerySourceText {
    fn from(text: String) -> Self {
        Self(text.into())
    }
}

impl From<Box<str>> for QuerySourceText {
    fn from(text: Box<str>) -> Self {
        Self(text.into())
    }
}

impl From<std::sync::Arc<str>> for QuerySourceText {
    fn from(text: std::sync::Arc<str>) -> Self {
        Self(text)
    }
}

/// Validated immutable module sources, shared by every query that names them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleCatalog(pub std::sync::Arc<BTreeMap<QuerySourceId, QueryDocumentSource>>);

/// The in-memory root and immutable named module sources for one query.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QueryBundle {
    /// The entry document compiled for this query.
    pub root: QueryDocumentSource,
    /// Explicitly registered immutable modules, shared by root queries and cache snapshots.
    pub modules: std::sync::Arc<BTreeMap<QuerySourceId, QueryDocumentSource>>,
}

impl From<QueryText> for QueryBundle {
    fn from(text: QueryText) -> Self {
        let root = QueryDocumentSource {
            id: QuerySourceId(super::constants::STANDALONE_SOURCE_ID.into()),
            text: text.0.into(),
        };
        Self {
            root,
            modules: Arc::new(BTreeMap::new()),
        }
    }
}

/// The name of a parameter, as the query writes it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WrittenParameter(pub Box<str>);

/// The type of a parameter, as the query writes it: `integer`, `list<string>`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypeName(pub Box<str>);

/// A stable identity for one step definition within a linked query bundle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StepIdentity(pub u64);

/// The name a `let` gives a step, or a step is read under; identity is separate from spelling.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StepName(pub StepSpelling, pub StepIdentified);

/// The text a step is written under.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StepSpelling(pub Box<str>);

impl std::ops::Deref for StepSpelling {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for StepSpelling {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for StepSpelling {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for StepSpelling {
    fn from(text: &str) -> Self {
        Self(text.into())
    }
}

impl From<String> for StepSpelling {
    fn from(text: String) -> Self {
        Self(text.into())
    }
}

impl From<Box<str>> for StepSpelling {
    fn from(text: Box<str>) -> Self {
        Self(text)
    }
}

/// Whether a step name carries the identity linking gave its definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StepIdentified {
    /// Linking assigned this identity.
    Assigned(StepIdentity),
    /// No identity is assigned yet.
    Unassigned,
}

/// The alias a source is given after `as`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AliasName(pub Box<str>);

/// The name of a relation, written in quotes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WrittenRelation(pub Box<str>);

/// The name of an operation a call applies.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OperationName(pub Box<str>);

/// The name of an argument of a call.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArgumentName(pub Box<str>);

/// Text the query wrote between quotes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WrittenLiteral(pub Box<str>);

/// A bare word the query wrote: a column, a step or a keyword such as `descendant`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WordText(pub Box<str>);

/// A name the query wrote, bare or in quotes: how a column or an argument is written.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WrittenName(pub Box<str>);

/// A bare word as the parsers read it: borrowed from the query text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BareWord<'a>(pub &'a str);

/// A name as the query spells it, borrowed from the program: a relation, a step, a column or a
/// parameter, whichever the program wrote in text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuerySpelling<'a>(pub &'a str);

/// Whether a statement or refusal has source attribution.
#[derive(Clone, Debug, PartialEq)]
pub enum Attribution {
    /// Where it came from, through any hygienic fragment expansion.
    Located(QueryOrigin),
    /// No attribution was recorded.
    Unlocated,
}

/// Whether a program's identities and imports have been linked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Linking {
    /// Source-local identities and imports are linked.
    Linked,
    /// The program is as its syntax gave it.
    Unlinked,
}

/// Where a linked syntax node came from, including the import and call path used to reach it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QueryOrigin {
    /// The source document that wrote the node.
    pub source_id: QuerySourceId,
    /// The original byte span in that document.
    pub span: QuerySpan,
    /// Import and call sites from the root to this source.
    pub trace: Vec<QueryTraceSite>,
}

/// One source boundary crossed while linking a fragment.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QueryTraceSite {
    /// The source containing this import or call.
    pub source_id: QuerySourceId,
    /// The imported or invoked source.
    pub target_id: QuerySourceId,
    /// The original byte span of the import or invocation.
    pub span: QuerySpan,
}
