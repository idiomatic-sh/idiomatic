//! The `facts` service's cases: word zero is the empty text, a base word keeps its code, a file's own text is coded after the
//! base words and once, the words of the last file are forgotten, and a fixed name's word is its place after the empty text:
//! the same cell in every file of the run and the word its text interns to. A fixed name given twice or empty is refused where
//! the base words are made; a handle past the names fails the assertion of a test build.

use super::errors::FixedNamesError;
use super::service::{base_words, clear_words, file_words, fixed, intern, text};
use super::types::{CellWord, FixedNames, FixedWord};
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
    /// The text of word zero, and the word the empty text interns to.
    Zero,
    /// The words of the first two fixed names, by their handles.
    FixedPlaces,
    /// A fixed name's word before and after the words of a file were cleared.
    FixedAcrossFiles,
    /// A fixed name's word by its handle, and the word its text interns to.
    FixedInterned,
    /// The first base text's word, which follows the fixed names.
    AfterFixed,
    /// Base words of fixed names that state one name at two places.
    NameTwice,
    /// Base words of fixed names that hold the empty text.
    NameEmpty,
    /// The word of the first handle past the fixed names.
    FixedPast,
}

/// Why a row was refused.
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    /// The base words were not made of the fixed names.
    Names(FixedNamesError),
    /// The read of a fixed word failed its assertion.
    Asserted,
}

type Input = Scenario;
/// Up to two numbers a row reads, the rest zero.
type Value = [u64; 2];
type Failure = Refusal;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_base_text_keeps_its_base_code"),
        word: Word::Succeeds,
        input: || Scenario::BaseText,
        expect: Expect::Value([4, 0]),
    },
    Case {
        name: CaseName("a_file_text_is_coded_after_the_base_words_once"),
        word: Word::Succeeds,
        input: || Scenario::OwnText,
        expect: Expect::Value([5, 5]),
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
        expect: Expect::Value([5, 0]),
    },
    Case {
        name: CaseName("word_zero_is_the_empty_text"),
        word: Word::Succeeds,
        input: || Scenario::Zero,
        expect: Expect::Value([1, 0]),
    },
    Case {
        name: CaseName("a_fixed_word_is_its_place_after_the_empty_text"),
        word: Word::Succeeds,
        input: || Scenario::FixedPlaces,
        expect: Expect::Value([1, 2]),
    },
    Case {
        name: CaseName("a_fixed_word_is_the_same_cell_in_every_file_of_the_run"),
        word: Word::Succeeds,
        input: || Scenario::FixedAcrossFiles,
        expect: Expect::Value([2, 2]),
    },
    Case {
        name: CaseName("a_fixed_word_is_the_word_its_text_interns_to"),
        word: Word::Succeeds,
        input: || Scenario::FixedInterned,
        expect: Expect::Value([2, 2]),
    },
    Case {
        name: CaseName("the_base_texts_follow_the_fixed_names"),
        word: Word::Succeeds,
        input: || Scenario::AfterFixed,
        expect: Expect::Value([3, 4]),
    },
    Case {
        name: CaseName("a_fixed_name_given_twice_is_refused"),
        word: Word::Refuses,
        input: || Scenario::NameTwice,
        expect: Expect::Error(Refusal::Names(FixedNamesError::Twice {
            first: FixedWord(0),
            again: FixedWord(2),
        })),
    },
    Case {
        name: CaseName("an_empty_fixed_name_is_refused"),
        word: Word::Refuses,
        input: || Scenario::NameEmpty,
        expect: Expect::Error(Refusal::Names(FixedNamesError::Empty {
            place: FixedWord(2),
        })),
    },
    Case {
        name: CaseName("a_handle_past_the_fixed_names_fails_the_assertion_of_a_test_build"),
        word: Word::Refuses,
        input: || Scenario::FixedPast,
        expect: Expect::Error(Refusal::Asserted),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut names = vec![NameText("next-token"), NameText("matching-delimiter")];
    match scenario {
        Scenario::NameTwice => names.push(NameText("next-token")),
        Scenario::NameEmpty => names.push(NameText("")),
        _ => {}
    }
    let base = base_words(
        &FixedNames(names),
        &[NameText("print"), NameText("eval"), NameText("print")],
    )
    .map_err(Refusal::Names)?;
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
        Scenario::Zero => [
            u64::from(text(&words, CellWord(0)).0.is_empty()),
            intern(&mut words, NameText("")).0,
        ],
        Scenario::FixedPlaces => [fixed(&words, FixedWord(0)).0, fixed(&words, FixedWord(1)).0],
        Scenario::FixedAcrossFiles => {
            let _own = intern(&mut words, NameText("first"));
            let before = fixed(&words, FixedWord(1));
            clear_words(&mut words);
            [before.0, fixed(&words, FixedWord(1)).0]
        }
        Scenario::FixedInterned => [
            fixed(&words, FixedWord(1)).0,
            intern(&mut words, NameText("matching-delimiter")).0,
        ],
        Scenario::AfterFixed => [
            intern(&mut words, NameText("print")).0,
            intern(&mut words, NameText("eval")).0,
        ],
        // Refused before any word is read: the base words were not made.
        Scenario::NameTwice | Scenario::NameEmpty => [0, 0],
        Scenario::FixedPast => {
            // The read fails its assertion in a test build: the failure is the answer this row expects.
            let read = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                fixed(&words, FixedWord(2))
            }));
            return read
                .map(|word| [word.0, 0])
                .map_err(|_panic| Refusal::Asserted);
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
