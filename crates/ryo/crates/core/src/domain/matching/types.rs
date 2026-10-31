//! Reusable compiled regular expressions.

/// A query's regular-expression pattern.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct RegexPattern(pub String);

impl std::borrow::Borrow<str> for RegexPattern {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// Compiled patterns, indexed by their exact source text.
#[derive(Debug, Default)]
pub(crate) struct Patterns(pub hashbrown::HashMap<RegexPattern, regex_automata::meta::Regex>);
