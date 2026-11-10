//! The `relation_lookup` service's cases, on values alone.

use ryo_relations::domain::relations::types::Relation;

use super::errors::RelationLookupError;
use super::service::resolve;
use crate::cases::{Case, CaseName, Expect, Matched, Word, run, selects};
use ryo_values::domain::descriptions::types::RelationName;

/// What a row asks: the relations ryo lists and the text asked for.
#[derive(Clone, Copy)]
struct Question {
    /// The relations ryo lists.
    known: &'static [Relation],
    /// The text asked for.
    text: &'static str,
}

/// What a row asks.
type Input = Question;
/// How many columns the relation found has.
type Value = usize;
type Failure = RelationLookupError;

/// The relations ryo lists when it lists none.
const NO_RELATIONS: &[Relation] = &[];

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_relation_is_found_by_its_text"),
        word: Word::Succeeds,
        input: || Question {
            known: &[Relation::Function],
            text: "function",
        },
        expect: Expect::Value(4),
    },
    Case {
        name: CaseName("a_name_no_relation_has_is_refused"),
        word: Word::Refuses,
        input: || Question {
            known: &[Relation::Function],
            text: "nothing",
        },
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(
                error,
                RelationLookupError::UnknownRelation { .. }
            )))
        }),
    },
    Case {
        name: CaseName("a_relation_ryo_does_not_list_is_unknown_even_when_it_exists"),
        word: Word::Refuses,
        input: || Question {
            known: NO_RELATIONS,
            text: "function",
        },
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(
                error,
                RelationLookupError::UnknownRelation { .. }
            )))
        }),
    },
];

fn subject(question: Input) -> Result<Value, Failure> {
    let relation = resolve(question.known, &RelationName(question.text.into()))?;
    Ok(ryo_relations::domain::relations::service::columns(relation).len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
