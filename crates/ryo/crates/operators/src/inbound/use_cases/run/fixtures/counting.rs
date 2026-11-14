//! A matching port that counts the regexes it compiles, over the matching crate's own use case: the double that proves a
//! pattern is compiled once per worker per node.

use crate::outbound::matching::interface::MatchingPort;
use ryo_matching::inbound::use_cases::matching::errors::MatchingError;
use ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase;
use ryo_matching::inbound::use_cases::matching::types::{
    CompiledGlobs, CompiledRegex, CompiledTextGlobs, GlobHits, GlobPattern, GroupName,
    MatchOutcome, RegexSource, SearchText, TextSpan,
};
use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The matching crate's use case, counting every regex compiled through it.
#[derive(Debug, Default)]
pub struct CountingMatching {
    /// How many regexes were compiled.
    compiles: AtomicUsize,
}

impl CountingMatching {
    /// How many regexes were compiled so far.
    #[must_use]
    pub fn compiles(&self) -> usize {
        self.compiles.load(Ordering::Relaxed)
    }
}

impl MatchingUseCase for CountingMatching {
    fn compile_regex(&self, pattern: RegexSource<'_>) -> Result<CompiledRegex, MatchingError> {
        let _before = self.compiles.fetch_add(1, Ordering::Relaxed);
        MatchingUseCaseImpl::new().compile_regex(pattern)
    }
    fn matches_regex(&self, regex: &CompiledRegex, text: SearchText<'_>) -> MatchOutcome {
        MatchingUseCaseImpl::new().matches_regex(regex, text)
    }
    fn visit_regex_matches(
        &self,
        regex: &CompiledRegex,
        text: SearchText<'_>,
        each: &mut dyn FnMut(TextSpan) -> std::ops::ControlFlow<()>,
    ) -> std::ops::ControlFlow<()> {
        MatchingUseCaseImpl::new().visit_regex_matches(regex, text, each)
    }
    fn visit_regex_captures(
        &self,
        regex: &CompiledRegex,
        text: SearchText<'_>,
        name: GroupName<'_>,
        each: &mut dyn FnMut(TextSpan, TextSpan) -> std::ops::ControlFlow<()>,
    ) -> Result<std::ops::ControlFlow<()>, MatchingError> {
        MatchingUseCaseImpl::new().visit_regex_captures(regex, text, name, each)
    }
    fn compile_text_globs(&self, patterns: &[GlobPattern<'_>]) -> CompiledTextGlobs {
        MatchingUseCaseImpl::new().compile_text_globs(patterns)
    }
    fn matches_text_globs(&self, globs: &CompiledTextGlobs, text: SearchText<'_>) -> MatchOutcome {
        MatchingUseCaseImpl::new().matches_text_globs(globs, text)
    }
    fn matches_text_glob(&self, pattern: GlobPattern<'_>, text: SearchText<'_>) -> MatchOutcome {
        MatchingUseCaseImpl::new().matches_text_glob(pattern, text)
    }
    fn compile_globs(&self, globs: &[GlobPattern<'_>]) -> Result<CompiledGlobs, MatchingError> {
        MatchingUseCaseImpl::new().compile_globs(globs)
    }
    fn parse_glob(&self, glob: GlobPattern<'_>) -> Result<(), MatchingError> {
        MatchingUseCaseImpl::new().parse_glob(glob)
    }
    fn match_globs(&self, globs: &CompiledGlobs, path: &std::path::Path, hits: &mut GlobHits) {
        MatchingUseCaseImpl::new().match_globs(globs, path, hits);
    }
}

impl MatchingPort for CountingMatching {
    fn visit_regex_matches(
        &self,
        regex: &CompiledRegex,
        text: SearchText<'_>,
        each: &mut dyn FnMut(TextSpan) -> std::ops::ControlFlow<()>,
    ) -> std::ops::ControlFlow<()> {
        MatchingUseCase::visit_regex_matches(self, regex, text, each)
    }
    fn visit_regex_captures(
        &self,
        regex: &CompiledRegex,
        text: SearchText<'_>,
        name: GroupName<'_>,
        each: &mut dyn FnMut(TextSpan, TextSpan) -> std::ops::ControlFlow<()>,
    ) -> Result<std::ops::ControlFlow<()>, MatchingError> {
        MatchingUseCase::visit_regex_captures(self, regex, text, name, each)
    }
    fn semantics(&self) -> &dyn MatchingUseCase {
        self
    }
}
