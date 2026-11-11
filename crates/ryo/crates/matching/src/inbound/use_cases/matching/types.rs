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

/// A compiled regex whose storage and lifetime belong to its caller.
///
/// # Examples
///
/// ```
/// use ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase;
/// use ryo_matching::inbound::use_cases::matching::types::{MatchOutcome, RegexSource, SearchText};
/// use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
///
/// let matching = MatchingUseCaseImpl::new();
/// let regex = matching.compile_regex(RegexSource("a+b")).expect("a valid regex");
/// assert_eq!(matching.matches_regex(&regex, SearchText("xaab")), MatchOutcome::Matches);
/// ```
#[derive(Clone, Debug)]
pub struct CompiledRegex {
    pub(crate) regex: regex_automata::meta::Regex,
}

/// A compiled set of globs in authored order.
///
/// # Examples
///
/// ```
/// use ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase;
/// use ryo_matching::inbound::use_cases::matching::types::{GlobHits, GlobPattern};
/// use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
///
/// let matching = MatchingUseCaseImpl::new();
/// let globs = matching
///     .compile_globs(&[GlobPattern("*.md"), GlobPattern("src/**")])
///     .expect("valid globs");
/// let mut hits = GlobHits::default();
/// matching.match_globs(&globs, std::path::Path::new("src/lib.rs"), &mut hits);
/// assert_eq!(hits, GlobHits(vec![1]));
/// ```
#[derive(Clone, Debug)]
pub struct CompiledGlobs {
    pub(crate) set: globset::GlobSet,
}

/// Compiled text-glob semantics with private exact-compiler or interpreter strategy.
///
/// # Examples
///
/// ```
/// use ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase;
/// use ryo_matching::inbound::use_cases::matching::types::{GlobPattern, MatchOutcome, SearchText};
/// use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
///
/// let matching = MatchingUseCaseImpl::new();
/// let globs = matching.compile_text_globs(&[GlobPattern("TODO*")]);
/// assert_eq!(matching.matches_text_globs(&globs, SearchText("TODO: x")), MatchOutcome::Matches);
/// ```
/// Engine owners retain this value; it contains no cache key, permit or scheduling authority.
#[derive(Debug)]
pub struct CompiledTextGlobs {
    pub(crate) program: crate::domain::globbing::types::TextGlobProgram,
}

impl Default for CompiledTextGlobs {
    fn default() -> Self {
        Self {
            program: crate::domain::globbing::types::TextGlobProgram::Interpreted(Vec::new()),
        }
    }
}
