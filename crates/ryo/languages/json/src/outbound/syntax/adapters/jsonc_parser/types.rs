//! The types of the `jsonc_parser` adapter: plain data, public fields and derives, with no constructor and no inherent impl.

/// What the adapter reads a part of a file as: the library's own kinds stand here, and the schemas map each to the language's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Piece {
    /// The whole file.
    File,
    /// An object.
    Object,
    /// A property of an object: its name, the colon and its value.
    Member,
    /// An array.
    Array,
    /// An opening brace.
    OpenBrace,
    /// A closing brace.
    CloseBrace,
    /// An opening bracket.
    OpenBracket,
    /// A closing bracket.
    CloseBracket,
    /// A comma.
    Comma,
    /// A colon.
    Colon,
    /// A string.
    String,
    /// A number.
    Number,
    /// The word `true`.
    True,
    /// The word `false`.
    False,
    /// The word `null`.
    Null,
    /// A word that is none of the above: the library scans it, and reads it as a name only in the dialects the adapter turns off.
    Word,
    /// A comment to the end of the line.
    LineComment,
    /// A comment between `/*` and `*/`.
    BlockComment,
    /// White space the library skips.
    Whitespace,
    /// Text the library could not read.
    Unread,
}

/// The text of a file the library reads: the bytes of the file up to the first that is not UTF-8.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct JsoncText<'t>(pub &'t str);

/// Whether text no token spans, between the tokens of a file the library read, is told as a diagnostic: when the library read the
/// file it said nothing of it, and when it did not, it said what it could.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Reporting {
    /// Tell it.
    Told,
    /// The library's own diagnostic says it.
    Silent,
}

/// What the adapter writes a tree with: the tree, and where its last token ends, and the text the library read.
pub(super) struct Writing<'b, 'w, 't> {
    /// The tree.
    pub tree: ryo::languages::Lossless<'b, 'w>,
    /// The text the library read.
    pub text: JsoncText<'t>,
    /// Whether the text no token spans is told.
    pub reporting: Reporting,
}
