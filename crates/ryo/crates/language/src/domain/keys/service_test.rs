//! The `keys` service's cases, on values alone: equal keys share the first occurrence, a digest collision never merges two keys,
//! and clearing keeps the capacity for the next file.

use super::service::{clear_keys, intern_key, key_class_at, push_key_word};
use super::types::{KeyState, RetainedKeyIndex, ShapeWord};
use ryo_relations::domain::relations::types::Position;
use ryo_relations::domain::terms::types::Symbol;

use crate::cases::{Case, CaseName, Expect, Word, run};

/// A scenario: what a row asks of the service, observed as text.
type Input = fn() -> String;
/// What a scenario observed, each check in order, joined by `|`.
type Value = &'static str;
type Failure = ();

/// A word whose every value hashes alike, so that two different keys of one length share a digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Colliding(u32);

impl std::hash::Hash for Colliding {
    fn hash<H: std::hash::Hasher>(&self, _state: &mut H) {}
}

fn equal_keys_share_the_first_occurrence() -> String {
    let mut index = RetainedKeyIndex::<Position, ShapeWord>::default();
    let mut observed = Vec::new();
    for (at, word) in [(2, 1), (6, 2), (9, 1)] {
        index.scratch.clear();
        let _pushed = push_key_word(&mut index, ShapeWord::Identifier(Symbol(word)));
        observed.push(format!("{:?}", intern_key(&mut index, Position(at))));
    }
    observed.push(format!("{}", index.records.len()));
    observed.join("|")
}

fn colliding_digests_stay_distinct_classes() -> String {
    let mut index = RetainedKeyIndex::<Position, Colliding>::default();
    let mut observed = Vec::new();
    for (at, word) in [(2, 10), (5, 11), (8, 10)] {
        index.scratch.clear();
        index.scratch.push(Colliding(word));
        observed.push(format!("{:?}", intern_key(&mut index, Position(at))));
    }
    observed.push(format!("{}", index.records.len()));
    observed.push(format!("{}", index.buckets.len()));
    observed.join("|")
}

fn clearing_empties_every_field_and_keeps_capacity() -> String {
    let mut index = RetainedKeyIndex::<Position, ShapeWord>::default();
    index.classes = vec![None, Some(Position(1))];
    index.scratch.push(ShapeWord::Literal(Symbol(3)));
    let _class = intern_key(&mut index, Position(1));
    index.state = KeyState::Complete;
    let kept = index.words.capacity();
    let before = key_class_at(&index, Position(1));
    clear_keys(&mut index);
    format!(
        "{before:?}|{:?}|{}|{}|{}|{}|{}|{:?}|{}",
        key_class_at(&index, Position(1)),
        index.classes.len(),
        index.words.len(),
        index.records.len(),
        index.buckets.len(),
        index.scratch.len(),
        index.state,
        index.words.capacity() == kept,
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("equal_keys_share_the_first_occurrence"),
        word: Word::Succeeds,
        input: || equal_keys_share_the_first_occurrence,
        expect: Expect::Value("Position(2)|Position(6)|Position(2)|2"),
    },
    Case {
        name: CaseName("colliding_digests_stay_distinct_classes"),
        word: Word::Succeeds,
        input: || colliding_digests_stay_distinct_classes,
        expect: Expect::Value("Position(2)|Position(5)|Position(2)|2|1"),
    },
    Case {
        name: CaseName("clearing_empties_every_field_and_keeps_capacity"),
        word: Word::Succeeds,
        input: || clearing_empties_every_field_and_keeps_capacity,
        expect: Expect::Value("Some(Position(1))|None|0|0|0|0|0|Fresh|true"),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(Box::leak(scenario().into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
