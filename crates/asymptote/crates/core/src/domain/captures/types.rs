//! The types of the `captures` concept: plain data, public fields and derives, with no constructor and no inherent
//! impl.

use std::collections::BTreeMap;

/// Text as the manifest writes it: an argument of a command line, an input, or a segment of one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Written<'a>(pub &'a str);

/// The name of a capture: the `name` of `{name}`, a letter or `_` and then letters, digits, `_` and `-`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CaptureName(pub Box<str>);

/// The value a capture took: one path segment, never empty.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CaptureValue(pub Box<str>);

/// The values a tool's captures took, by name.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Captures(pub BTreeMap<CaptureName, CaptureValue>);

/// Text of a pattern that is matched, and written, as it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Literal(pub Box<str>);

/// One piece of a segment as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    /// Text matched as it is written.
    Text(Literal),
    /// A capture: one or more characters of the segment.
    Capture(CaptureName),
}

/// One segment of an input as written, in its pieces.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SegmentPattern(pub Vec<Piece>);

/// Text with each bound capture replaced by its value.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Substituted(pub String);

/// The command line of a tool a capture is written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandKey {
    /// `run`.
    Run,
    /// `fix`.
    Fix,
}

/// A capture a tool's command line writes that none of its inputs does, so no match gives it a value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Uncaptured {
    /// The command line it is written in.
    pub key: CommandKey,
    /// The capture.
    pub name: CaptureName,
}
