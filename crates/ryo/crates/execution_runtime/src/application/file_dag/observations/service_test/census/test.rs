//! The shared walk's census counts every synthetic observation union a compiled program adds, beside the original
//! instructions, each with its observation barrier.
use super::fixtures::{Census, Declared, Routes, census};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::Relation;
use std::borrow::Cow;

type Input = Declared;
type Value = Census;
type Failure = OperatingError;

/// Two observation inputs, `left` and `right`, over the column `file`.
const LEFT_AND_RIGHT: Declared = Declared {
    left: "left",
    right: "right",
    column: "file",
};

/// What the program of two named observation inputs answers: two consumers, each a lone head over
/// its slot, two synthetic unions, and a census of six instructions whose observation barrier
/// holds both of each and whose shared walk holds the other two.
const BOTH_UNIONS: Census = Census {
    relations_asked: Cow::Borrowed(&[
        Relation::LanguageTarget,
        Relation::CompilerFeature,
        Relation::LanguageTarget,
        Relation::CompilerFeature,
    ]),
    consumers_select_their_bound_observation: true,
    consumers_are_lone_heads_over_their_slot: true,
    consumers: 2,
    unions_are_lone_heads_over_observation_members: true,
    unions: 2,
    instructions: 6,
    logical_regions: 4,
    consumers_have_no_walk_class: true,
    routes: Routes {
        instructions: 6,
        shared: 2,
        representation: 0,
        dependent: 0,
        observation: 4,
        row: 0,
        derived: 0,
        request: 0,
        continuation: 0,
    },
};

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_census_includes_every_synthetic_observation_union"),
        word: Word::Succeeds,
        input: || LEFT_AND_RIGHT,
        expect: Expect::Value(BOTH_UNIONS),
    },
    Case {
        name: CaseName("shared_walk_census_includes_every_synthetic_observation_union"),
        word: Word::Succeeds,
        input: || LEFT_AND_RIGHT,
        expect: Expect::Value(BOTH_UNIONS),
    },
];

/// Compiles the declared program and reads its census.
fn subject(input: Input) -> Result<Value, Failure> {
    census(input)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
