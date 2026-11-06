//! Content-only results that may survive one source revision.
use super::constants;
use std::collections::HashMap;

/// The exact input text a memo entry is keyed by.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct MemoKey(pub String);

impl std::borrow::Borrow<str> for MemoKey {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// Input text to look a memo entry up by, without owning it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Lookup<'a>(pub &'a str);

/// Reusable input assembly storage, bounded independently of active file data.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Scratch(pub String);

/// The static name of the parser type a memo belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ParserName(pub &'static str);

/// The static name of the grammar dialect a memo belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DialectName(pub &'static str);

/// Static parser type identity, including the caller's grammar marker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MemoGrammar {
    /// The parser implementation.
    pub parser: ParserName,
    /// The dialect it was asked for.
    pub dialect: DialectName,
}

/// Whether a memo was produced by the parser implementation and dialect asked about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GrammarState {
    /// The memo was made by this exact parser and dialect.
    Current,
    /// Another parser or dialect made it; its contents must not be reused.
    Stale,
}

/// Whether an ordered lookup could grow by one entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reservation {
    /// The room was reserved.
    Granted,
    /// The room could not be reserved; the lookup was discarded.
    Refused,
}

/// One result held for the following source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Entry<V> {
    /// The application-owned grammar result.
    pub value: V,
}

/// An entry taken out of a memo with the key it was stored under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Taken<V> {
    /// The input text the entry was keyed by.
    pub key: MemoKey,
    /// The result stored for it.
    pub value: V,
}

/// A bounded, single-previous-source memo, separate from active file outputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GrammarMemo<V> {
    /// Exact input text; a memo belongs to one dialect and parser type.
    pub entries: HashMap<MemoKey, Entry<V>>,
    /// Which parser and dialect made the entries.
    pub grammar: MemoGrammar,
    /// Reusable input assembly storage.
    pub scratch: Scratch,
}

/// A number of memo entries.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct EntryCount(pub usize);

impl<V> Default for GrammarMemo<V> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            grammar: MemoGrammar {
                parser: ParserName(constants::EMPTY),
                dialect: DialectName(constants::EMPTY),
            },
            scratch: Scratch::default(),
        }
    }
}
