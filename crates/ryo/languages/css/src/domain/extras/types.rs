//! The types of the `extras` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// What a CSS syntax tree keeps besides the standard's nodes and tokens, so that its tokens spell the file. One closed list, each name
/// once; each says why the standard has no name for it.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    strum::EnumIter,
    strum::IntoStaticStr,
    strum::FromRepr,
    strum::EnumCount,
)]
#[strum(serialize_all = "kebab-case")]
#[repr(u16)]
pub(crate) enum Extra {
    /// A comment, `/* .. */`: the standard's tokenizer consumes comments and outputs no token for them (§4.3.2).
    Comment,
    /// The byte order mark a file may start with: the standard's decoding drops it before the tokenizer reads (§3.2).
    ByteOrderMark,
    /// Bytes that are not UTF-8: the standard decodes them as U+FFFD before it tokenizes (§3.2); the language reads UTF-8 and keeps
    /// them as they are.
    Error,
}

#[cfg(any(test, feature = "fixtures"))]
/// What an extra is to the readers: text a relation reads (a comment), or bytes it only keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Content {
    /// Text a relation reads.
    Read,
    /// Bytes the tree keeps so that its tokens spell the file.
    Kept,
}
