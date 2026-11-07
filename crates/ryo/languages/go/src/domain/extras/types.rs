//! The types of the `extras` concept: plain data, public fields and derives, with no constructor and no inherent impl. An extra is a
//! kind of the language's trees that `go/ast` and `go/token` have no name for: the text between tokens, and the parts of a `go.mod` file
//! as `golang.org/x/mod/modfile` names them.

/// What a kind of a Go syntax tree is that the standard lacks. One closed list, each name once.
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
    /// The text between two tokens: spaces, tabs and line ends.
    Whitespace,
    /// Bytes that are not text the parser could read.
    Error,
    /// A `go.mod` file, `modfile.FileSyntax` of `golang.org/x/mod`: its directives in order.
    ModFile,
    /// A directive of a `go.mod` file, `modfile.Line`: a verb and its words on one line, or one line of a block.
    ModLine,
    /// A block of a `go.mod` file, `modfile.LineBlock`: a verb, then its lines between parentheses.
    ModLineBlock,
    /// A word of a `go.mod` line: a verb, a module path or a version, quoted or not.
    ModWord,
    /// The `(` that opens a block of a `go.mod` file.
    ModLParen,
    /// The `)` that closes a block of a `go.mod` file.
    ModRParen,
    /// A `//` comment of a `go.mod` file, to the end of its line.
    ModComment,
}
