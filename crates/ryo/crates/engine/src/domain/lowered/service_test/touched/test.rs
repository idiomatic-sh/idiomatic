//! The walk of what a pack's steps and stages touch: every program and every pipeline of every fill is met, each step and
//! stage once. Every step and stage of the pack a row walks writes a register of its own, so the registers written, in the
//! order met, say what was met and when.

use super::super::fixtures::{nested, whole};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowered::service::each_touched;
use crate::domain::lowered::types::LoweredPack;
use std::borrow::Cow;

/// The pack a row walks.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// No definition, rule or table.
    Empty,
    /// A definition of a call form (registers 1 and 2) and a table form; a rule of two branches (3; then 4 with an inline body
    /// that sets 5, then 6); and a table of each fill: rows (two pipelines, 10 and 11, then 12), a group (13), a sorted table
    /// (14), an ordered match (15 and 16), a graph of two inputs (17 and 18) and written rows.
    Whole,
    /// One rule whose branch holds every step that runs bodies, each step and body writing registers of its own: see
    /// `fixtures::nested`.
    Nested,
}

type Input = Scenario;
/// The registers written by what the walk met, in the order met.
type Value = Cow<'static, [u32]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_empty_pack_has_nothing_to_meet"),
        word: Word::Succeeds,
        input: || Scenario::Empty,
        expect: Expect::Value(Cow::Borrowed(&[])),
    },
    Case {
        name: CaseName("a_packs_walk_meets_every_program_and_every_pipeline_of_every_fill_once"),
        word: Word::Succeeds,
        input: || Scenario::Whole,
        expect: Expect::Value(Cow::Borrowed(&[
            1, 2, 3, 4, 5, 6, 10, 11, 12, 13, 14, 15, 16, 17, 18,
        ])),
    },
    Case {
        name: CaseName("a_step_is_met_before_its_inline_bodies_and_they_in_its_operands_order"),
        word: Word::Succeeds,
        input: || Scenario::Nested,
        expect: Expect::Value(Cow::Borrowed(&[
            20, 22, 25, 26, 21, 23, 24, 28, 29, 27, 32, 30, 31, 33, 34, 35, 36,
        ])),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let pack = match scenario {
        Scenario::Empty => LoweredPack::default(),
        Scenario::Whole => whole(),
        Scenario::Nested => nested(),
    };
    let mut writes = Vec::new();
    each_touched(&pack, &mut |touched| {
        writes.extend(touched.writes.iter().map(|register| register.0));
    });
    Ok(Cow::Owned(writes))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
