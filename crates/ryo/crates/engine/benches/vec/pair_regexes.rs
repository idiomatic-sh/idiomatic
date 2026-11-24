//! File witnesses use the same literal glob semantics as the prepared select.
use ryo_matching::inbound::use_cases::matching::{
    interface::MatchingUseCase,
    types::{CompiledTextGlobs, GlobPattern, MatchOutcome, SearchText},
    use_case::MatchingUseCaseImpl,
};
use std::cell::OnceCell;

pub(in super::super) enum Matcher {
    Regex(regex::bytes::Regex),
    Globs(CompiledTextGlobs),
}
impl Matcher {
    pub(in super::super) fn is_match(&self, text: &[u8]) -> bool {
        match self {
            Self::Regex(regex) => regex.is_match(text),
            Self::Globs(patterns) => {
                MatchingUseCaseImpl::new()
                    .matches_text_globs(patterns, SearchText(&String::from_utf8_lossy(text)))
                    == MatchOutcome::Matches
            }
        }
    }
}
pub(in super::super) struct Pattern {
    text: Box<str>,
    matcher: OnceCell<Option<Matcher>>,
}
impl Pattern {
    pub(super) fn new(s: &str) -> Self {
        Self {
            text: s.into(),
            matcher: OnceCell::new(),
        }
    }
    pub(super) fn globs(patterns: &[String]) -> Self {
        let globs = patterns.iter().map(|s| GlobPattern(s)).collect::<Vec<_>>();
        Self {
            text: format!("{patterns:?}").into(),
            matcher: OnceCell::from(Some(Matcher::Globs(
                MatchingUseCaseImpl::new().compile_text_globs(&globs),
            ))),
        }
    }
    pub(in super::super) fn as_str(&self) -> &str {
        &self.text
    }
    pub(in super::super) fn initialized(&self) -> bool {
        self.matcher.get().is_some()
    }
    pub(in super::super) fn compiled(&self) -> Option<&Matcher> {
        self.matcher
            .get_or_init(|| {
                regex::bytes::Regex::new(&self.text)
                    .ok()
                    .map(Matcher::Regex)
            })
            .as_ref()
    }
    pub(in super::super) fn is_match(&self, text: &[u8]) -> bool {
        self.compiled().is_none_or(|r| r.is_match(text))
    }
}
