//! What a caller hands matching: authored patterns, the text they are searched in, and what a search reports; and the
//! opaque compiled values it hands back, whose storage and lifetime are the caller's. No scheduling, preparation or cache
//! authority is carried here.

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

/// A compiled regex whose storage and lifetime belong to its caller. Its matcher is private: a caller holds the value and
/// hands it back to matching, and never reaches the regex inside.
///
/// ```compile_fail,E0616
/// use ryo_matching::inbound::use_cases::matching::{interface::MatchingUseCase, types::RegexSource, use_case::MatchingUseCaseImpl};
/// let compiled = MatchingUseCaseImpl::new().compile_regex(RegexSource("ok")).unwrap();
/// let _ = compiled.regex;
/// ```
///
/// # Examples
///
/// ```
/// use ryo_matching::inbound::use_cases::matching::{interface::MatchingUseCase, types::{CompiledRegex, MatchOutcome, RegexSource, SearchText}, use_case::MatchingUseCaseImpl};
/// let matching = MatchingUseCaseImpl::new();
/// let compiled: CompiledRegex = matching.compile_regex(RegexSource("^ok$")).unwrap();
/// assert_eq!(matching.matches_regex(&compiled, SearchText("ok")), MatchOutcome::Matches);
/// ```
#[derive(Clone, Debug)]
pub struct CompiledRegex {
    pub(crate) regex: regex_automata::meta::Regex,
}

/// A compiled set of globs in authored order. Its set is private: a caller holds the value and hands it back to matching.
///
/// ```compile_fail,E0616
/// use ryo_matching::inbound::use_cases::matching::{interface::MatchingUseCase, types::GlobPattern, use_case::MatchingUseCaseImpl};
/// let compiled = MatchingUseCaseImpl::new().compile_globs(&[GlobPattern("*.rs")]).unwrap();
/// let _ = compiled.set;
/// ```
///
/// # Examples
///
/// ```
/// use ryo_matching::inbound::use_cases::matching::{interface::MatchingUseCase, types::{CompiledGlobs, GlobHits, GlobPattern}, use_case::MatchingUseCaseImpl};
/// let matching = MatchingUseCaseImpl::new();
/// let compiled: CompiledGlobs = matching.compile_globs(&[GlobPattern("*.md"), GlobPattern("*.rs")]).unwrap();
/// let mut hits = GlobHits::default();
/// matching.match_globs(&compiled, std::path::Path::new("lib.rs"), &mut hits);
/// assert_eq!(hits, GlobHits(vec![1]));
/// ```
#[derive(Clone, Debug)]
pub struct CompiledGlobs {
    pub(crate) set: globset::GlobSet,
}

/// Compiled text-glob semantics with private exact-compiler or interpreter strategy.
/// Engine owners retain this value; it contains no cache key, permit or scheduling authority.
///
/// ```compile_fail,E0616
/// use ryo_matching::inbound::use_cases::matching::{interface::MatchingUseCase, types::GlobPattern, use_case::MatchingUseCaseImpl};
/// let compiled = MatchingUseCaseImpl::new().compile_text_globs(&[GlobPattern("a?c")]);
/// let _ = compiled.program;
/// ```
///
/// # Examples
///
/// ```
/// use ryo_matching::inbound::use_cases::matching::{interface::MatchingUseCase, types::{CompiledTextGlobs, GlobPattern, MatchOutcome, SearchText}, use_case::MatchingUseCaseImpl};
/// let matching = MatchingUseCaseImpl::new();
/// let compiled: CompiledTextGlobs = matching.compile_text_globs(&[GlobPattern("a?c")]);
/// assert_eq!(matching.matches_text_globs(&compiled, SearchText("abc")), MatchOutcome::Matches);
/// ```
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
