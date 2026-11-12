//! Immutable matcher metadata prepared from the finite authored request.

pub mod prepared;

use ryo_matching::inbound::use_cases::matching::errors::MatchingError;
use ryo_matching::inbound::use_cases::matching::types::{
    CompiledRegex, CompiledTextGlobs, GlobPattern, GroupName, MatchOutcome, RegexSource,
    SearchText, TextSpan,
};

/// Seals [`PatternSemantics`]: the trait is public so the engine crates can lend it, and its one implementation is the blanket
/// bridge in the application layer. The bridge covers every implementation of the kernel's `MatchingUseCase`, in any crate, so
/// a type gains `PatternSemantics` only by implementing `MatchingUseCase`, never directly.
pub(crate) mod sealed {
    /// Implemented only by the application bridge's blanket impl over `MatchingUseCase`: no crate implements it, or
    /// `PatternSemantics`, for a type directly, so every matching semantics is a `MatchingUseCase`'s.
    #[expect(
        unnameable_types,
        reason = "the seal: a supertrait other crates can reach but never name, so they cannot implement PatternSemantics"
    )]
    pub trait Sealed {}
}

/// The regex and text-glob semantics the domain compiles and matches through: a port its parent hands down, so the domain names
/// no use case. It is sealed: every `MatchingUseCase` implementation, in this crate or another, answers it through this
/// crate's blanket bridge, and no type implements it in any other way.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::matching::types::{PatternMatcher, PatternSemantics};
/// use ryo_matching::inbound::use_cases::matching::types::{MatchOutcome, RegexSource, SearchText};
/// use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
/// let matcher = PatternMatcher(&MatchingUseCaseImpl);
/// assert!(matcher.0.compile_regex(RegexSource("(")).is_err());
/// let regex = matcher.0.compile_regex(RegexSource("^ab+$")).expect("valid syntax compiles");
/// assert_eq!(matcher.0.matches_regex(&regex, SearchText("abb")), MatchOutcome::Matches);
/// assert_eq!(matcher.0.matches_regex(&regex, SearchText("ba")), MatchOutcome::Differs);
/// ```
pub trait PatternSemantics: sealed::Sealed {
    /// Compiles a regex; invalid syntax is a refusal, not an empty match.
    ///
    /// # Errors
    ///
    /// [`MatchingError::InvalidRegex`] when the pattern is not valid regex syntax.
    fn compile_regex(&self, pattern: RegexSource<'_>) -> Result<CompiledRegex, MatchingError>;
    /// Searches text with a compiled regex.
    fn matches_regex(&self, regex: &CompiledRegex, text: SearchText<'_>) -> MatchOutcome;
    /// Visits the matches of a named capture in source order, refusing a name the regex lacks before reading any text.
    ///
    /// # Errors
    ///
    /// [`MatchingError::InvalidRegex`] when `name` is not a group of the regex; the visitor is not called.
    fn visit_regex_captures(
        &self,
        regex: &CompiledRegex,
        text: SearchText<'_>,
        name: GroupName<'_>,
        each: &mut dyn FnMut(TextSpan, TextSpan) -> std::ops::ControlFlow<()>,
    ) -> Result<std::ops::ControlFlow<()>, MatchingError>;
    /// Compiles text globs together.
    fn compile_text_globs(&self, patterns: &[GlobPattern<'_>]) -> CompiledTextGlobs;
    /// Whether any compiled text glob matches the text.
    fn matches_text_globs(&self, globs: &CompiledTextGlobs, text: SearchText<'_>) -> MatchOutcome;
    /// Whether one uncompiled text glob matches the text.
    fn matches_text_glob(&self, pattern: GlobPattern<'_>, text: SearchText<'_>) -> MatchOutcome;
}

/// The matching semantics a parent hands down: the domain compiles and matches through it, and builds none.
#[derive(Clone, Copy)]
pub struct PatternMatcher<'a>(pub &'a dyn PatternSemantics);

impl std::fmt::Debug for PatternMatcher<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
