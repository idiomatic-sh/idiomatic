//! The `terms` service's cases, on values alone.

use super::service::{is_term, symbols_with_text, term_symbol, term_text};
use super::types::{
    DeclarationKind, DependencyKind, MappingKind, ParameterKind, ValueKind, Wordness,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::terms::types::Symbol;

/// What a row asks: a word, written as the text it should have.
#[derive(Clone, Copy)]
enum Scenario {
    /// A declaration word.
    Declaration(DeclarationKind),
    /// A parameter word.
    Parameter(ParameterKind),
    /// A value word.
    Value(ValueKind),
    /// A mapping word.
    Mapping(MappingKind),
    /// A dependency word.
    Dependency(DependencyKind),
    /// A name some file wrote.
    Written,
    /// A number past the last word of its block.
    PastTheLast,
}

/// What a row of the cases asks.
type Input = Scenario;

/// The text the word reads as, or nothing when it is no word.
type Value = Option<&'static str>;
type Failure = ();

fn subject(input: Input) -> Result<Value, Failure> {
    let symbol = match input {
        Scenario::Declaration(word) => term_symbol(word),
        Scenario::Parameter(word) => term_symbol(word),
        Scenario::Value(word) => term_symbol(word),
        Scenario::Mapping(word) => term_symbol(word),
        Scenario::Dependency(word) => term_symbol(word),
        Scenario::Written => Symbol(7),
        Scenario::PastTheLast => Symbol(0xFFFF_00FF),
    };
    Ok((is_term(symbol) == Wordness::Term)
        .then(|| term_text(symbol))
        .flatten()
        .map(|text| text.0))
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_declaration_word_reads_as_its_text"),
        word: Word::Succeeds,
        input: || Scenario::Declaration(DeclarationKind::EnumMember),
        expect: Expect::Value(Some("enum-member")),
    },
    Case {
        name: CaseName("a_parameter_word_reads_as_its_text"),
        word: Word::Succeeds,
        input: || Scenario::Parameter(ParameterKind::PositionalOrKeyword),
        expect: Expect::Value(Some("positional-or-keyword")),
    },
    Case {
        name: CaseName("a_value_word_reads_as_its_text"),
        word: Word::Succeeds,
        input: || Scenario::Value(ValueKind::SequenceOfMappings),
        expect: Expect::Value(Some("sequence-of-mappings")),
    },
    Case {
        name: CaseName("the_null_word_reads_as_its_text"),
        word: Word::Succeeds,
        input: || Scenario::Value(ValueKind::Null),
        expect: Expect::Value(Some("null")),
    },
    Case {
        name: CaseName("a_mapping_word_reads_as_its_text"),
        word: Word::Succeeds,
        input: || Scenario::Mapping(MappingKind::SequenceMember),
        expect: Expect::Value(Some("sequence-member")),
    },
    Case {
        name: CaseName("a_dependency_word_reads_as_its_text"),
        word: Word::Succeeds,
        input: || Scenario::Dependency(DependencyKind::Dev),
        expect: Expect::Value(Some("dev")),
    },
    Case {
        name: CaseName("a_name_a_file_wrote_is_no_word"),
        word: Word::Succeeds,
        input: || Scenario::Written,
        expect: Expect::Value(None),
    },
    Case {
        name: CaseName("a_number_past_the_last_word_is_no_word"),
        word: Word::Succeeds,
        input: || Scenario::PastTheLast,
        expect: Expect::Value(None),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// The text lookup must cover every currently decodable closed term, including
/// future blocks and noncontiguous discriminants in the reserved term range.
#[test]
fn text_lookup_covers_every_closed_term() {
    for raw in super::constants::TERM_BASE..=u32::MAX {
        let symbol = Symbol(raw);
        let Some(text) = term_text(symbol) else {
            continue;
        };
        let mut found = Vec::new();
        symbols_with_text(text.0, |candidate| found.push(candidate));
        assert!(found.contains(&symbol), "missing {symbol:?} = {}", text.0);
    }
}
