//! The `ordering` item's cases.

use super::super::interface::ComparisonsItem as _;
use super::item::{OrderingItem, natural_compare};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Verdict;
use crate::domain::table_texts::types::{Datum, TextView};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{Test, WholeNumber};
use std::cmp::Ordering::{Equal, Less};

/// What a row asks: a comparison of two sides, or a check of the natural order of texts.
enum Ask {
    /// A comparison: the test, the left side and the right side.
    Holds(Test, Datum<'static>, Datum<'static>),
    /// Pairs of texts, each less than the other.
    NaturalLess,
    /// Pairs of texts equal in natural order.
    NaturalEqual,
    /// Digit runs of four thousand digits, and a letter against its combining form.
    NaturalEdges,
    /// A corpus of texts that obeys antisymmetry and transitivity.
    NaturalLaws,
}
/// What a row answers: a verdict, or how many checks held.
#[derive(Debug, PartialEq, Eq)]
enum Answer {
    /// The verdict of a comparison.
    Verdict(Verdict),
    /// How many orders held.
    Checked(usize),
}
type Input = Ask;
type Value = Answer;
type Failure = OperatingError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("greater_than_holds"),
        word: Word::Succeeds,
        input: || {
            Ask::Holds(
                Test::GreaterThan,
                Datum::Number(WholeNumber(5)),
                Datum::Number(WholeNumber(4)),
            )
        },
        expect: Expect::Value(Answer::Verdict(Verdict::Holds)),
    },
    Case {
        name: CaseName("a_number_and_a_text_are_never_ordered"),
        word: Word::Succeeds,
        input: || {
            Ask::Holds(
                Test::LessThan,
                Datum::Number(WholeNumber(5)),
                Datum::Text(TextView("9")),
            )
        },
        expect: Expect::Value(Answer::Verdict(Verdict::Fails)),
    },
    Case {
        name: CaseName("texts_are_ordered_byte_by_byte"),
        word: Word::Succeeds,
        input: || {
            Ask::Holds(
                Test::LessThan,
                Datum::Text(TextView("crates/a")),
                Datum::Text(TextView("crates/b")),
            )
        },
        expect: Expect::Value(Answer::Verdict(Verdict::Holds)),
    },
    Case {
        name: CaseName("a_text_is_not_less_than_itself"),
        word: Word::Succeeds,
        input: || {
            Ask::Holds(
                Test::LessThan,
                Datum::Text(TextView("same")),
                Datum::Text(TextView("same")),
            )
        },
        expect: Expect::Value(Answer::Verdict(Verdict::Fails)),
    },
    Case {
        name: CaseName("a_text_is_at_least_itself"),
        word: Word::Succeeds,
        input: || {
            Ask::Holds(
                Test::GreaterThanOrEqual,
                Datum::Text(TextView("same")),
                Datum::Text(TextView("same")),
            )
        },
        expect: Expect::Value(Answer::Verdict(Verdict::Holds)),
    },
    Case {
        name: CaseName("a_longer_text_is_greater_than_its_prefix"),
        word: Word::Succeeds,
        input: || {
            Ask::Holds(
                Test::GreaterThan,
                Datum::Text(TextView("ab")),
                Datum::Text(TextView("a")),
            )
        },
        expect: Expect::Value(Answer::Verdict(Verdict::Holds)),
    },
    Case {
        name: CaseName("natural_order_reads_digit_runs_as_numbers"),
        word: Word::Succeeds,
        input: || {
            Ask::Holds(
                Test::NaturalLessThan,
                Datum::Text(TextView("item2")),
                Datum::Text(TextView("item10")),
            )
        },
        expect: Expect::Value(Answer::Verdict(Verdict::Holds)),
    },
    Case {
        name: CaseName("ordinary_order_reads_the_same_texts_byte_by_byte"),
        word: Word::Succeeds,
        input: || {
            Ask::Holds(
                Test::LessThan,
                Datum::Text(TextView("item2")),
                Datum::Text(TextView("item10")),
            )
        },
        expect: Expect::Value(Answer::Verdict(Verdict::Fails)),
    },
    Case {
        name: CaseName("natural_order_never_orders_a_number_and_a_text"),
        word: Word::Succeeds,
        input: || {
            Ask::Holds(
                Test::NaturalLessThan,
                Datum::Number(WholeNumber(2)),
                Datum::Text(TextView("item10")),
            )
        },
        expect: Expect::Value(Answer::Verdict(Verdict::Fails)),
    },
    Case {
        name: CaseName("numeric_runs_padding_boundaries_and_unicode_are_ordered"),
        word: Word::Succeeds,
        input: || Ask::NaturalLess,
        expect: Expect::Value(Answer::Checked(11)),
    },
    Case {
        name: CaseName("equivalent_whitespace_is_equal"),
        word: Word::Succeeds,
        input: || Ask::NaturalEqual,
        expect: Expect::Value(Answer::Checked(4)),
    },
    Case {
        name: CaseName("very_long_runs_and_combining_marks_are_ordered"),
        word: Word::Succeeds,
        input: || Ask::NaturalEdges,
        expect: Expect::Value(Answer::Checked(2)),
    },
    Case {
        name: CaseName("antisymmetry_and_transitivity_hold_over_a_corpus"),
        word: Word::Succeeds,
        input: || Ask::NaturalLaws,
        expect: Expect::Value(Answer::Checked(20)),
    },
    Case {
        name: CaseName("a_test_of_another_family_is_refused"),
        word: Word::Refuses,
        input: || {
            Ask::Holds(
                Test::Equal,
                Datum::Number(WholeNumber(1)),
                Datum::Number(WholeNumber(1)),
            )
        },
        expect: Expect::Error(OperatingError::WrongTest),
    },
];

fn compare(left: &str, right: &str) -> std::cmp::Ordering {
    natural_compare(TextView(left), TextView(right))
}

const LESS: &[(&str, &str)] = &[
    ("item2", "item10"),
    ("item2a", "item10"),
    ("item1", "item1a"),
    ("x01", "x010"),
    ("x02", "x1"),
    ("x001", "x01"),
    ("A2", "a1"),
    ("a9", "a٩"),
    ("aé", "aΩ"),
    ("1 2", "12"),
    ("1a", "1b"),
];

const EQUAL: &[(&str, &str)] = &[
    ("a1", "a 1"),
    ("a\u{2003}1", "a1"),
    ("", " \t"),
    ("1 2", "1  2"),
];

const CORPUS: &[&str] = &[
    "", " ", "0", "00", "01", "1", "1 2", "12", "2", "10", "a", "a1", "a 1", "a01", "a1a", "a2",
    "A1", "é", "e\u{301}", "٩",
];

fn natural_less() -> Value {
    for (left, right) in LESS {
        assert_eq!(compare(left, right), Less, "{left:?} < {right:?}");
        assert_eq!(compare(right, left), Less.reverse());
    }
    Answer::Checked(LESS.len())
}

fn natural_equal() -> Value {
    for (left, right) in EQUAL {
        assert_eq!(compare(left, right), Equal);
    }
    Answer::Checked(EQUAL.len())
}

fn natural_edges() -> Value {
    let long = "9".repeat(4096);
    let longer = format!("1{}", "0".repeat(4096));
    assert_eq!(compare(&long, &longer), Less);
    assert_eq!(compare("é", "e\u{301}"), Less.reverse());
    Answer::Checked(2)
}

fn natural_laws() -> Value {
    for a in CORPUS {
        for b in CORPUS {
            assert_eq!(compare(a, b), compare(b, a).reverse(), "{a:?}, {b:?}");
            for c in CORPUS {
                if compare(a, b).is_le() && compare(b, c).is_le() {
                    assert!(compare(a, c).is_le(), "{a:?} <= {b:?} <= {c:?}");
                }
                if compare(a, b) == Equal {
                    assert_eq!(compare(a, c), compare(b, c), "equivalent {a:?}, {b:?}");
                }
            }
        }
    }
    Answer::Checked(CORPUS.len())
}

fn subject(ask: Input) -> Result<Value, Failure> {
    match ask {
        Ask::Holds(test, left, right) => {
            OrderingItem.holds(test, &left, &right).map(Answer::Verdict)
        }
        Ask::NaturalLess => Ok(natural_less()),
        Ask::NaturalEqual => Ok(natural_equal()),
        Ask::NaturalEdges => Ok(natural_edges()),
        Ask::NaturalLaws => Ok(natural_laws()),
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
