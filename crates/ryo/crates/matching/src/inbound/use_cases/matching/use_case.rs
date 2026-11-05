//! Pure matching orchestration without cache ownership or engine dependencies.

use super::compiled::{CompiledGlobs, CompiledRegex, CompiledTextGlobs};
use super::errors::MatchingError;
use super::interface::MatchingUseCase;
use crate::domain::matching::service;

/// Executes matching semantics using only values supplied by the caller.
#[derive(Clone, Copy, Debug, Default)]
pub struct MatchingUseCaseImpl;

impl MatchingUseCaseImpl {
    /// Construct stateless matching semantics.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl MatchingUseCase for MatchingUseCaseImpl {
    fn compile_regex(&self, pattern: &str) -> Result<CompiledRegex, MatchingError> {
        service::compile_regex(pattern)
            .map(|regex| CompiledRegex { regex })
            .map_err(|reason| MatchingError::InvalidRegex { reason })
    }

    fn matches_regex(&self, regex: &CompiledRegex, text: &str) -> bool {
        service::matches_regex(&regex.regex, text)
    }

    fn visit_regex_matches(
        &self,
        regex: &CompiledRegex,
        text: &str,
        each: &mut dyn FnMut(std::ops::Range<usize>) -> std::ops::ControlFlow<()>,
    ) -> std::ops::ControlFlow<()> {
        service::visit_regex_matches(&regex.regex, text, each)
    }

    fn visit_regex_captures(
        &self,
        regex: &CompiledRegex,
        text: &str,
        name: &str,
        each: &mut dyn FnMut(
            std::ops::Range<usize>,
            std::ops::Range<usize>,
        ) -> std::ops::ControlFlow<()>,
    ) -> Result<std::ops::ControlFlow<()>, MatchingError> {
        service::visit_regex_captures(&regex.regex, text, name, each)
            .map_err(|reason| MatchingError::InvalidRegex { reason })
    }

    fn compile_text_globs(&self, patterns: &[&str]) -> CompiledTextGlobs {
        let borrowed: Vec<_> = patterns
            .iter()
            .map(|pattern| crate::domain::globbing::types::GlobText(pattern))
            .collect();
        CompiledTextGlobs {
            program: crate::domain::globbing::service::compile(&borrowed),
        }
    }

    fn matches_text_globs(&self, globs: &CompiledTextGlobs, text: &str) -> bool {
        crate::domain::globbing::service::matches_compiled(
            &globs.program,
            crate::domain::globbing::types::Subject(text),
        ) == crate::domain::globbing::types::GlobMatch::Matches
    }

    fn matches_text_glob(&self, pattern: &str, text: &str) -> bool {
        crate::domain::globbing::service::matches(
            crate::domain::globbing::types::GlobText(pattern),
            crate::domain::globbing::types::Subject(text),
        ) == crate::domain::globbing::types::GlobMatch::Matches
    }

    fn compile_globs(&self, globs: &[&str]) -> Result<CompiledGlobs, MatchingError> {
        service::compile_globs(globs)
            .map(|set| CompiledGlobs { set })
            .map_err(|(glob, reason)| MatchingError::BadGlob { glob, reason })
    }

    fn parse_glob(&self, glob: &str) -> Result<(), MatchingError> {
        service::parse_glob(glob).map_err(|reason| MatchingError::BadGlob {
            glob: glob.to_owned(),
            reason,
        })
    }

    fn match_globs(&self, globs: &CompiledGlobs, path: &std::path::Path, hits: &mut Vec<usize>) {
        service::match_globs(&globs.set, path, hits);
    }
}
