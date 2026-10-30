//! The names of the `siblings` concept: the keywords and the punctuation that tell the fields of a node apart.

use super::types::Word;

/// The keyword before a loop's or a `try`'s `orelse`, and before an `if` expression's.
pub(crate) const ELSE: Word<'static> = Word("else");

/// The keyword before a `try`'s `finalbody`.
pub(crate) const FINALLY: Word<'static> = Word("finally");

/// The keyword before a `raise`'s `cause`.
pub(crate) const FROM: Word<'static> = Word("from");

/// The keyword before an alias's `asname`.
pub(crate) const AS: Word<'static> = Word("as");

/// The keyword before an `if` expression's `test` and a comprehension's `ifs`.
pub(crate) const IF: Word<'static> = Word("if");

/// The keyword before a comprehension's `iter`.
pub(crate) const IN: Word<'static> = Word("in");

/// The keyword of an asynchronous comprehension, its `is_async`.
pub(crate) const ASYNC: Word<'static> = Word(crate::domain::definitions::constants::ASYNC);

/// The punctuation before a type variable's `bound`.
pub(crate) const COLON: Word<'static> = Word(":");

/// The punctuation before a type parameter's `default_value`.
pub(crate) const EQUAL: Word<'static> = Word("=");
