//! The `notes` service's cases, on values alone.

use super::service::{has_noted_words, note_of, note_word, set_note};
use super::types::Noted;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::symbols::service::clear_symbols;
use crate::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{Bits, NameNote};
use ryo_relations::domain::terms::types::Symbol;

/// A scenario: what a row asks of the service.
type Input = fn() -> usize;
/// What a scenario answers: a count.
type Value = usize;
type Failure = ();

/// A noted number is read back for its name, a name noted nothing of reads `0`, and clearing the names clears the notes.
fn a_note_is_kept_for_its_name_until_the_names_are_cleared() -> usize {
    let mut symbols = Symbols::default();
    set_note(&mut symbols, Symbol(3), NameNote(7));
    let kept = note_of(&symbols, Symbol(3)) == NameNote(7)
        && note_of(&symbols, Symbol(1)) == NameNote(0)
        && note_of(&symbols, Symbol(9)) == NameNote(0);
    clear_symbols(&mut symbols);
    usize::from(kept && note_of(&symbols, Symbol(3)) == NameNote(0))
}

/// A noted word is seen as words, and clearing the names takes it back.
fn a_noted_word_is_seen_until_the_names_are_cleared() -> usize {
    let mut symbols = Symbols::default();
    let before = has_noted_words(&symbols) == Noted::Nothing;
    note_word(&mut symbols, Bits(5));
    let after = has_noted_words(&symbols) == Noted::Words;
    clear_symbols(&mut symbols);
    usize::from(before && after && has_noted_words(&symbols) == Noted::Nothing)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_note_is_kept_for_its_name_until_the_names_are_cleared"),
        word: Word::Succeeds,
        input: || a_note_is_kept_for_its_name_until_the_names_are_cleared,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_noted_word_is_seen_until_the_names_are_cleared"),
        word: Word::Succeeds,
        input: || a_noted_word_is_seen_until_the_names_are_cleared,
        expect: Expect::Value(1),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
