//! The types of the `matching` concept: plain data, public fields and derives, with no constructor and no inherent impl. The authored
//! patterns, the text they are searched in, and what a search reports.

/// The authored text of a regex.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AuthoredRegex<'a>(pub &'a str);

/// The text a regex is searched in.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Haystack<'a>(pub &'a str);

/// The name of a capture group.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CaptureName<'a>(pub &'a str);

/// The authored text of a path glob.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PathGlob<'a>(pub &'a str);

/// The compiler's reason for refusing an expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CompilerReason(pub String);

/// The authored text of a glob a set compilation refused, empty for a failure of the set as a whole.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RefusedGlob(pub String);

/// Why a glob set was not compiled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GlobRefusal {
    /// The glob refused.
    pub glob: RefusedGlob,
    /// Why it was refused.
    pub reason: CompilerReason,
}

/// Whether a regex matches a text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RegexMatch {
    /// The text matches.
    Matches,
    /// The text does not match.
    Differs,
}

/// A span of bytes of the searched text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MatchSpan(pub std::ops::Range<usize>);

/// The authored indices of the globs that matched, in authored order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct AuthoredIndices(pub Vec<usize>);
