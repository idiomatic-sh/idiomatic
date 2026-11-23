//! Proofs validate their regexes at load. Positive checks build a matcher at
//! the first surviving candidate. A failed positive matcher admits its input.
use std::cell::OnceCell;

pub(in super::super) struct Pattern {
    text: Box<str>,
    regex: OnceCell<Option<regex::bytes::Regex>>,
}
impl Pattern {
    pub(super) fn new(s: &str) -> Self {
        Self {
            text: s.into(),
            regex: OnceCell::new(),
        }
    }
    pub(in super::super) fn as_str(&self) -> &str {
        &self.text
    }
    pub(in super::super) fn initialized(&self) -> bool {
        self.regex.get().is_some()
    }
    pub(in super::super) fn compiled(&self) -> Option<&regex::bytes::Regex> {
        self.regex
            .get_or_init(|| regex::bytes::Regex::new(&self.text).ok())
            .as_ref()
    }
    pub(in super::super) fn is_match(&self, text: &[u8]) -> bool {
        self.compiled().is_none_or(|r| r.is_match(text))
    }
}
