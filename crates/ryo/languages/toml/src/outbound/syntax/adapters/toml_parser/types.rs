//! The types of the `toml_parser` adapter: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::syntax::types::Kind;
use ryo::languages::{Count, Offset};
use toml_parser::decoder::{Encoding, ScalarKind};
use toml_parser::parser::EventKind;

/// What the adapter reads a part of a file as: the library's own kinds stand here, and the schemas map each to the language's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Piece {
    /// An event of the parser.
    Event(EventKind),
    /// A string, by the encoding the decoder gives it.
    String(Encoding),
    /// An unquoted scalar, by the class the decoder gives it.
    Scalar(ScalarKind),
}

/// White space or text the parser could not read, met after a part of a key, written once the key goes on or is closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Held {
    /// What it is.
    pub kind: Kind,
    /// Where it starts.
    pub start: Offset,
    /// Where it ends.
    pub end: Offset,
}

/// What the adapter is reading: a header, whose key opens no key-value, or the entries below one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Reading {
    /// A header, `[a.b]` or `[[a.b]]`.
    Header,
    /// Entries.
    Entries,
}

/// Whether the library's decoder reported anything of a token it decoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Decoded {
    /// It read the token cleanly.
    Clean,
    /// It refused something in it.
    Refused,
}

/// What the decoder reads a token as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    /// A part of a key.
    Key,
    /// A value.
    Value,
}

/// Whether an array or an inline table was opened, or refused past the nesting the library reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Opened {
    /// It was opened.
    Opened,
    /// It was refused.
    Refused,
}

/// The text the library's decoder writes into, kept for the length of one file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Scratch(pub String);

/// What the library said is wrong at one place of a document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Report {
    /// What it said.
    pub message: ReportMessage,
    /// Where it starts, in bytes.
    pub start: Count,
    /// Where it ends.
    pub end: Count,
}

/// What the library said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ReportMessage(pub String);

/// The part of a file's bytes that is UTF-8 text, which the library reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TomlText<'a>(pub &'a str);

/// What the decoder made of a token: what it classified an unquoted scalar as, and whether it refused anything in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Decoding {
    /// The class of the scalar.
    pub classified: ScalarKind,
    /// Whether the decoder refused anything.
    pub said: Decoded,
}
