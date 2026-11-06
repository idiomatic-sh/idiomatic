//! What a caller hands matching: authored patterns, the text they are searched in, and what a search reports.

/// The authored text of a regex.
#[derive(Clone, Copy, Debug)]
pub struct RegexSource<'a>(pub &'a str);

/// The text a regex or a text glob is matched against.
#[derive(Clone, Copy, Debug)]
pub struct SearchText<'a>(pub &'a str);

/// The name of a regex capture group.
#[derive(Clone, Copy, Debug)]
pub struct GroupName<'a>(pub &'a str);

/// The authored text of a path glob or a text glob.
#[derive(Clone, Copy, Debug)]
pub struct GlobPattern<'a>(pub &'a str);

/// A span of bytes of the searched text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextSpan(pub std::ops::Range<usize>);

/// The authored indices of the globs that matched, in authored order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GlobHits(pub Vec<usize>);

/// Whether a pattern matches a text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchOutcome {
    /// The text matches.
    Matches,
    /// The text does not match.
    Differs,
}

impl From<crate::domain::matching::types::RegexMatch> for MatchOutcome {
    fn from(outcome: crate::domain::matching::types::RegexMatch) -> Self {
        match outcome {
            crate::domain::matching::types::RegexMatch::Matches => Self::Matches,
            crate::domain::matching::types::RegexMatch::Differs => Self::Differs,
        }
    }
}

impl From<crate::domain::globbing::types::GlobMatch> for MatchOutcome {
    fn from(outcome: crate::domain::globbing::types::GlobMatch) -> Self {
        match outcome {
            crate::domain::globbing::types::GlobMatch::Matches => Self::Matches,
            crate::domain::globbing::types::GlobMatch::Differs => Self::Differs,
        }
    }
}
