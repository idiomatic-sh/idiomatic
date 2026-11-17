//! The types of the `captures` concept: plain data, public fields and derives, with no constructor and no inherent impl.
//! What a pattern's `{name}`s took, and text with them replaced.

/// The values a pattern's captures took, by name.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Captures(pub std::collections::BTreeMap<CaptureName, CaptureValue>);

/// The name of a capture: the `name` of `{name}`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CaptureName(pub Box<str>);

/// The value a capture took.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CaptureValue(pub Box<str>);

/// Values interpolated into a pattern's `{name}`s instead of captured: a spec instance's params.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Interpolations(pub std::collections::BTreeMap<CaptureName, CaptureValue>);

/// Text whose `{name}` and `{*name}` are replaced by values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Template<'a>(pub &'a str);

/// A capture's name and the value it is replaced by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptureBinding<'a> {
    /// The name.
    pub name: &'a CaptureName,
    /// Its value.
    pub value: &'a CaptureValue,
}

/// Text with its names replaced by their values.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Substituted(pub String);

impl std::ops::Deref for Substituted {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Substituted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<Substituted> for Box<str> {
    fn from(text: Substituted) -> Self {
        text.0.into()
    }
}

/// Captures as a message writes them: `name = value, name = value`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapturesText(pub String);

impl std::ops::Deref for CapturesText {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CapturesText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<CapturesText> for Box<str> {
    fn from(text: CapturesText) -> Self {
        text.0.into()
    }
}

/// One piece of a path pattern's text, as the pattern grammar reads it: a character written as itself (escaped or plain),
/// a wildcard, or a capture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatternPiece<'a> {
    /// A character that stands for itself.
    Literal(PatternChar),
    /// `*`: any text within a segment.
    Star,
    /// `?`: one character.
    Question,
    /// `{name}`: a capture within a segment.
    Capture(PatternName<'a>),
    /// `{*name}`: a capture of whole segments.
    Span(PatternName<'a>),
    /// `{name:dir}`: a capture within a segment whose value names a directory beside the file, holding a file.
    Directory(PatternName<'a>),
}

/// A character of a path pattern that stands for itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PatternChar(pub char);

/// The name a capture of a path pattern is written with: the `name` of `{name}`, `{*name}` or `{name:dir}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PatternName<'a>(pub &'a str);

/// Text that names itself, to be written into a path pattern: a file or directory's own name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlainText<'a>(pub &'a str);

/// Text as a path pattern writes it: each character the pattern gives a meaning escaped.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PatternWritten(pub String);

/// The path a pattern with nothing left open names, its escapes taken away.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NamedPath(pub String);

/// Whether a path pattern holds a wildcard that no escape makes text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wildcards {
    /// It holds a `*` or a `?`.
    Held,
    /// It holds none.
    None,
}
