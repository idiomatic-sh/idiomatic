//! Any matching use case answers the domain's `PatternSemantics`: the parent hands the use case down, the domain sees only its
//! port. The crate exports nothing from here: the impl is the whole of it.

use crate::domain::matching::types::PatternSemantics;
use ryo_matching::inbound::use_cases::matching::compiled::{CompiledRegex, CompiledTextGlobs};
use ryo_matching::inbound::use_cases::matching::errors::MatchingError;
use ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase;
use ryo_matching::inbound::use_cases::matching::types::{
    GlobPattern, MatchOutcome, RegexSource, SearchText,
};

impl<Matching: MatchingUseCase + ?Sized> PatternSemantics for Matching {
    fn compile_regex(&self, pattern: RegexSource<'_>) -> Result<CompiledRegex, MatchingError> {
        MatchingUseCase::compile_regex(self, pattern)
    }
    fn matches_regex(&self, regex: &CompiledRegex, text: SearchText<'_>) -> MatchOutcome {
        MatchingUseCase::matches_regex(self, regex, text)
    }
    fn compile_text_globs(&self, patterns: &[GlobPattern<'_>]) -> CompiledTextGlobs {
        MatchingUseCase::compile_text_globs(self, patterns)
    }
    fn matches_text_globs(&self, globs: &CompiledTextGlobs, text: SearchText<'_>) -> MatchOutcome {
        MatchingUseCase::matches_text_globs(self, globs, text)
    }
    fn matches_text_glob(&self, pattern: GlobPattern<'_>, text: SearchText<'_>) -> MatchOutcome {
        MatchingUseCase::matches_text_glob(self, pattern, text)
    }
}
