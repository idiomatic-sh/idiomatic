//! Immutable matcher metadata prepared from the finite authored request.

pub mod prepared;

use ryo_matching::inbound::use_cases::matching::compiled::{CompiledRegex, CompiledTextGlobs};
use ryo_matching::inbound::use_cases::matching::errors::MatchingError;
use ryo_matching::inbound::use_cases::matching::types::{
    GlobPattern, MatchOutcome, RegexSource, SearchText,
};

/// The regex and text-glob semantics the domain compiles and matches through: a port its parent hands down, so the domain names
/// no use case.
pub trait PatternSemantics {
    /// Compiles a regex; invalid syntax is a refusal, not an empty match.
    fn compile_regex(&self, pattern: RegexSource<'_>) -> Result<CompiledRegex, MatchingError>;
    /// Searches text with a compiled regex.
    fn matches_regex(&self, regex: &CompiledRegex, text: SearchText<'_>) -> MatchOutcome;
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
