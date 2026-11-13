//! Comparison-family classification and project text identity are pure operating contracts.
use super::{
    errors::OperatingError,
    service::{family_of, minted},
    types::{AfterTheFiles, Family},
};
use crate::domain::operating::service::derived_fault;
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_language::domain::symbols::{
    service::{hash_in, intern_derived, place_of},
    types::{ByteSlice, Symbols},
};
use ryo_relations::domain::{
    relations::types::{CROSS_FILE, Origin, Value as Cell},
    terms::types::NameText,
};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::plan_parts::types::Test,
};
use std::borrow::Cow;
/// What a row asks of the operating contracts.
#[derive(Clone, Debug)]
enum Ask {
    /// The family a test belongs to.
    Family(Test),
    /// The cell minted for the project text, interned as derived text, before or after the files were read.
    Mint(&'static str, AfterTheFiles),
    /// The refusal a derived fault becomes.
    Fault(DerivedFault),
}

/// What a minted cell is.
#[derive(Debug, PartialEq)]
enum Minted {
    /// The symbol, and whether it is the symbol interned.
    Symbol(bool),
    /// A name: whether its hash is the symbol's, where its text is, and the text it reads as.
    Name {
        hashed: bool,
        origin: Option<Origin>,
        text: Option<Cow<'static, str>>,
    },
    /// Some other cell.
    Other,
}

/// What the operating contracts answer.
#[derive(Debug, PartialEq)]
enum Answer {
    Family(Family),
    Minted(Minted),
}

type Input = Ask;
type Value = Answer;
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_test_belongs_to_its_family"),
        word: Word::Succeeds,
        input: || Ask::Family(Test::NotGlobAny),
        expect: Expect::Value(Answer::Family(Family::Glob)),
    },
    Case {
        name: CaseName("a_name_made_after_the_files_were_read_compares_with_theirs"),
        word: Word::Succeeds,
        input: || Ask::Mint("here", AfterTheFiles::After),
        expect: Expect::Value(Answer::Minted(Minted::Name {
            hashed: true,
            origin: Some(Origin::Derived(CROSS_FILE)),
            text: Some(Cow::Borrowed("here")),
        })),
    },
    Case {
        name: CaseName("a_name_made_before_the_files_were_read_stays_its_symbol"),
        word: Word::Succeeds,
        input: || Ask::Mint("here", AfterTheFiles::Before),
        expect: Expect::Value(Answer::Minted(Minted::Symbol(true))),
    },
    Case {
        name: CaseName("a_duplicate_order_fault_is_refused_as_a_duplicate_order"),
        word: Word::Refuses,
        input: || Ask::Fault(DerivedFault::DuplicateOrder),
        expect: Expect::Error(OperatingError::DuplicateOrder),
    },
    Case {
        name: CaseName("a_duplicate_dfs_order_fault_is_refused_as_a_duplicate_dfs_order"),
        word: Word::Refuses,
        input: || Ask::Fault(DerivedFault::DuplicateDfsOrder),
        expect: Expect::Error(OperatingError::DuplicateDfsOrder),
    },
];
fn minted_of(text: &'static str, after: AfterTheFiles) -> Minted {
    use crate::domain::table_texts::types::{Datum, TextView, Texts};
    let mut symbols = Symbols::default();
    let symbol = intern_derived(&mut symbols, ByteSlice(b""), NameText(text));
    match minted(&symbols, symbol, after) {
        Cell::Symbol(kept) => Minted::Symbol(kept == symbol),
        Cell::Name(name) => {
            let texts = Texts {
                symbols: &symbols,
                source: ryo_values::domain::files::types::SourceView(b""),
                paths: &[],
                arenas: &[],
            };
            Minted::Name {
                hashed: Some(name.hash) == hash_in(&symbols, symbol),
                origin: place_of(name, &[], &symbols).map(|place| place.origin),
                text: match crate::domain::table_texts::service::datum(Cell::Name(name), &texts) {
                    Datum::Text(TextView(read)) => Some(Cow::Owned(read.into())),
                    Datum::Unlocated
                    | Datum::Number(_)
                    | Datum::Flag(_)
                    | Datum::Numbers(_)
                    | Datum::Texts(_) => None,
                },
            }
        }
        Cell::Node(_) | Cell::File(_) | Cell::Span(_) | Cell::Number(_) | Cell::Flag(_) => {
            Minted::Other
        }
    }
}
fn subject(ask: Input) -> Result<Value, Failure> {
    match ask {
        Ask::Fault(fault) => Err(derived_fault(fault)),
        Ask::Family(test) => Ok(Answer::Family(family_of(test))),
        Ask::Mint(text, after) => Ok(Answer::Minted(minted_of(text, after))),
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
