//! The `facts` service's cases: a base word keeps its code, a file's own text is coded after the base words and once, the words
//! of the last file are forgotten, and a filler's rows reach the engine as columns through the fact source.

use super::service::{base_words, clear_words, file_words, intern, text};
use super::types::CellWord;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::terms::types::NameText;

/// What a row asks of the words.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// Interning a text the run's base words hold.
    BaseText,
    /// Interning a text of the file, twice.
    OwnText,
    /// The text of a file's own word.
    TextOfOwn,
    /// Interning after the last file's words were cleared.
    Cleared,
}

type Input = Scenario;
/// Up to two numbers a row reads, the rest zero.
type Value = [u64; 2];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_base_text_keeps_its_base_code"),
        word: Word::Succeeds,
        input: || Scenario::BaseText,
        expect: Expect::Value([1, 0]),
    },
    Case {
        name: CaseName("a_file_text_is_coded_after_the_base_words_once"),
        word: Word::Succeeds,
        input: || Scenario::OwnText,
        expect: Expect::Value([2, 2]),
    },
    Case {
        name: CaseName("a_file_word_reads_back_as_its_text"),
        word: Word::Succeeds,
        input: || Scenario::TextOfOwn,
        expect: Expect::Value([1, 1]),
    },
    Case {
        name: CaseName("the_last_file_words_are_forgotten"),
        word: Word::Succeeds,
        input: || Scenario::Cleared,
        expect: Expect::Value([2, 0]),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let base = base_words(&[NameText("print"), NameText("eval"), NameText("print")]);
    let mut words = file_words(&base);
    Ok(match scenario {
        Scenario::BaseText => [intern(&mut words, NameText("eval")).0, 0],
        Scenario::OwnText => [
            intern(&mut words, NameText("local")).0,
            intern(&mut words, NameText("local")).0,
        ],
        Scenario::TextOfOwn => {
            let own = intern(&mut words, NameText("local"));
            [
                u64::from(text(&words, own) == NameText("local")),
                u64::from(text(&words, CellWord(9)).0.is_empty()),
            ]
        }
        Scenario::Cleared => {
            let _first = intern(&mut words, NameText("first"));
            clear_words(&mut words);
            [intern(&mut words, NameText("second")).0, 0]
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
