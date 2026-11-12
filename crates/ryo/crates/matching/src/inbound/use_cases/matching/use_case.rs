//! Pure matching orchestration without cache ownership or engine dependencies.

use super::errors::MatchingError;
use super::interface::MatchingUseCase;
use super::types::{
    CompiledGlobs, CompiledRegex, CompiledTextGlobs, GlobHits, GlobPattern, GroupName,
    MatchOutcome, RegexSource, SearchText, TextSpan,
};
use crate::domain::matching::service;
use crate::domain::matching::types::{
    AuthoredIndices, AuthoredRegex, CaptureName, Haystack, PathGlob,
};

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
    fn compile_regex(&self, pattern: RegexSource<'_>) -> Result<CompiledRegex, MatchingError> {
        service::compile_regex(AuthoredRegex(pattern.0))
            .map(|regex| CompiledRegex { regex })
            .map_err(|reason| MatchingError::InvalidRegex { reason: reason.0 })
    }

    fn matches_regex(&self, regex: &CompiledRegex, text: SearchText<'_>) -> MatchOutcome {
        service::matches_regex(&regex.regex, Haystack(text.0)).into()
    }

    fn visit_regex_matches(
        &self,
        regex: &CompiledRegex,
        text: SearchText<'_>,
        each: &mut dyn FnMut(TextSpan) -> std::ops::ControlFlow<()>,
    ) -> std::ops::ControlFlow<()> {
        service::visit_regex_matches(&regex.regex, Haystack(text.0), &mut |span| {
            each(TextSpan(span.0))
        })
    }

    fn visit_regex_captures(
        &self,
        regex: &CompiledRegex,
        text: SearchText<'_>,
        name: GroupName<'_>,
        each: &mut dyn FnMut(TextSpan, TextSpan) -> std::ops::ControlFlow<()>,
    ) -> Result<std::ops::ControlFlow<()>, MatchingError> {
        service::visit_regex_captures(
            &regex.regex,
            Haystack(text.0),
            CaptureName(name.0),
            &mut |whole, group| each(TextSpan(whole.0), TextSpan(group.0)),
        )
        .map_err(|reason| MatchingError::InvalidRegex { reason: reason.0 })
    }

    fn compile_text_globs(&self, patterns: &[GlobPattern<'_>]) -> CompiledTextGlobs {
        let borrowed: Vec<_> = patterns
            .iter()
            .map(|pattern| crate::domain::globbing::types::GlobText(pattern.0))
            .collect();
        CompiledTextGlobs {
            program: crate::domain::globbing::service::compile(&borrowed),
        }
    }

    fn matches_text_globs(&self, globs: &CompiledTextGlobs, text: SearchText<'_>) -> MatchOutcome {
        crate::domain::globbing::service::matches_compiled(
            &globs.program,
            crate::domain::globbing::types::Subject(text.0),
        )
        .into()
    }

    fn matches_text_glob(&self, pattern: GlobPattern<'_>, text: SearchText<'_>) -> MatchOutcome {
        crate::domain::globbing::service::matches(
            crate::domain::globbing::types::GlobText(pattern.0),
            crate::domain::globbing::types::Subject(text.0),
        )
        .into()
    }

    fn compile_globs(&self, globs: &[GlobPattern<'_>]) -> Result<CompiledGlobs, MatchingError> {
        let authored: Vec<_> = globs.iter().map(|glob| PathGlob(glob.0)).collect();
        service::compile_globs(&authored)
            .map(|set| CompiledGlobs { set })
            .map_err(|refusal| MatchingError::BadGlob {
                glob: refusal.glob.0,
                reason: refusal.reason.0,
            })
    }

    fn parse_glob(&self, glob: GlobPattern<'_>) -> Result<(), MatchingError> {
        service::parse_glob(PathGlob(glob.0)).map_err(|reason| MatchingError::BadGlob {
            glob: glob.0.to_owned(),
            reason: reason.0,
        })
    }

    fn match_globs(&self, globs: &CompiledGlobs, path: &std::path::Path, hits: &mut GlobHits) {
        let mut authored = AuthoredIndices(std::mem::take(&mut hits.0));
        service::match_globs(&globs.set, path, &mut authored);
        hits.0 = authored.0;
    }
}
