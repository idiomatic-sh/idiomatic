//! File witnesses use the same literal glob semantics as the prepared select.
use ryo_matching::inbound::use_cases::matching::{
    interface::MatchingUseCase,
    types::{CompiledTextGlobs, GlobPattern, MatchOutcome, SearchText},
    use_case::MatchingUseCaseImpl,
};
use std::cell::OnceCell;

pub(in super::super) enum Matcher {
    Regex(regex::bytes::Regex),
    Globs(CompiledTextGlobs, Option<regex::bytes::Regex>),
}
impl Matcher {
    pub(in super::super) fn is_match(&self, text: &[u8]) -> bool {
        match self {
            Self::Regex(regex) => regex.is_match(text),
            Self::Globs(patterns, gate) => {
                gate.as_ref().is_none_or(|regex| regex.is_match(text))
                    && MatchingUseCaseImpl::new()
                        .matches_text_globs(patterns, SearchText(&String::from_utf8_lossy(text)))
                        == MatchOutcome::Matches
            }
        }
    }
}
/// A brace-free glob must contain each literal run outside a character class.
/// Decline unfamiliar syntax. The matching service decides all surviving values.
fn required_literal(glob: &str) -> Option<&str> {
    if !glob.is_ascii() || glob.contains(['{', '}', '\\']) || glob.contains("**/") {
        return None;
    }
    let mut longest = "";
    let mut rest = glob;
    while !rest.is_empty() {
        let n = rest.find(['?', '*', '[', ']']).unwrap_or(rest.len());
        if n > longest.len() {
            longest = &rest[..n];
        }
        rest = &rest[n..];
        if let Some(class) = rest.strip_prefix('[') {
            let contents = class.strip_prefix(['!', '^']).unwrap_or(class);
            if contents.starts_with(']') {
                return None;
            }
            let (inside, tail) = contents.split_once(']')?;
            if inside.is_empty() || inside.contains('[') {
                return None;
            }
            rest = tail;
        } else if rest.starts_with(']') {
            return None;
        } else if !rest.is_empty() {
            rest = &rest[1..];
        }
    }
    (!longest.is_empty()).then_some(longest)
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
        let literals = patterns
            .iter()
            .map(|s| required_literal(s))
            .collect::<Option<Vec<_>>>();
        let gate = literals.and_then(|literals| {
            let mut branches: Vec<_> = literals.into_iter().map(regex::escape).collect();
            branches.sort_unstable();
            branches.dedup();
            regex::bytes::Regex::new(&branches.join("|")).ok()
        });
        Self {
            text: format!("{patterns:?}").into(),
            matcher: OnceCell::from(Some(Matcher::Globs(
                MatchingUseCaseImpl::new().compile_text_globs(&globs),
                gate,
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
