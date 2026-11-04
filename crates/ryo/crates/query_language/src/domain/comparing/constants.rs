//! The names of the `comparing` concept.

use ryo_values::domain::program::types::WrittenFunction;

/// What separates the words of a closed set when a refusal lists them.
pub(crate) const WORD_SEPARATOR: &str = ", ";

/// The article a refusal names a column with when its name starts with a vowel.
pub(crate) const ARTICLE_BEFORE_VOWEL: &str = "an";

/// The article a refusal names any other column with.
pub(crate) const ARTICLE: &str = "a";

/// The letters a name takes `an` before.
pub(crate) const VOWELS: [char; 5] = ['a', 'e', 'i', 'o', 'u'];

/// The article a refusal names `column` with: `an` before a vowel, else `a`.
#[must_use]
pub(crate) fn article(column: &str) -> &'static str {
    if column.starts_with(VOWELS) {
        ARTICLE_BEFORE_VOWEL
    } else {
        ARTICLE
    }
}

/// The name of a function, as a refusal writes it.
#[must_use]
pub(crate) fn function_name(function: WrittenFunction) -> &'static str {
    match function {
        WrittenFunction::Concat => "concat",
        WrittenFunction::Pascal => "pascal",
        WrittenFunction::Camel => "camel",
        WrittenFunction::Snake => "snake",
        WrittenFunction::Kebab => "kebab",
        WrittenFunction::Lower => "lower",
    }
}

/// How many arguments a function takes, as a refusal writes it, and whether `count` is that many.
#[must_use]
pub(crate) fn arity(function: WrittenFunction, count: usize) -> (&'static str, bool) {
    match function {
        WrittenFunction::Concat => ("two or more arguments", count >= 2),
        WrittenFunction::Pascal
        | WrittenFunction::Camel
        | WrittenFunction::Snake
        | WrittenFunction::Kebab => ("one argument", count == 1),
        WrittenFunction::Lower => ("one argument", count == 1),
    }
}

/// The name of a test that takes two texts, as a refusal writes it.
pub(crate) const STARTS_WITH: &str = "starts_with";

/// The name of a test that takes two texts, as a refusal writes it.
pub(crate) const ENDS_WITH: &str = "ends_with";

/// The regular-expression text predicate's query name.
pub(crate) const MATCHES: &str = "matches";
