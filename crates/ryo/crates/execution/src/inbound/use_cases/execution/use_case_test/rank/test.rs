//! Rank binding and row-retention contracts through the compiled public owner.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{
            ColumnIndex, HostedNumber, Input as PlanInput, LiteralText, ParameterId, StepId,
        },
        plans::types::Operation,
        query_bindings::types::ParameterBinding,
    },
};
use std::borrow::Cow;
type Input = (&'static [&'static str], Option<ParameterBinding>);
/// What the rank published: its row count, and each row's name and numeric layer in order.
#[derive(Debug, PartialEq, Eq)]
struct Ranked {
    rows: usize,
    ranked: Cow<'static, [(&'static str, i64)]>,
}
type Value = Ranked;
type Failure = OperatingError;
fn sets(layers: &[&[&str]]) -> ParameterBinding {
    ParameterBinding::TextSets(
        layers
            .iter()
            .map(|set| set.iter().map(|each| LiteralText((*each).into())).collect())
            .collect(),
    )
}

fn subject((names, binding): Input) -> Result<Value, Failure> {
    let mut plan = inputs::plan(vec![
        Operation::Hosted {
            relation: HostedNumber(0),
        },
        Operation::Rank {
            from: PlanInput::Step(StepId(0)),
            column: ColumnIndex(0),
            sets: ParameterId(0),
        },
    ]);
    plan.parameters
        .push(ryo_values::domain::query_bindings::types::PlanParameter {
            name: ryo_values::domain::plan_parts::types::ParameterName("input".into()),
            kind: ryo_values::domain::query_bindings::types::ParameterType::TextSets,
        });
    let mut symbols = Symbols::default();
    let output = fixtures::requests::published(
        plan,
        binding.into_iter().collect(),
        vec![fixtures::requests::texts("names", names)],
        &mut symbols,
    )?;
    // A row whose layer is not a number, or whose text is none of the names, is left out: `rows` still counts it.
    let ranked: Vec<_> = output
        .cells
        .chunks_exact(2)
        .filter_map(|row| {
            let Cell::Number(place) = row[1] else {
                return None;
            };
            let text = inputs::text(row[0], &symbols);
            names
                .iter()
                .copied()
                .find(|name| *name == text)
                .map(|name| (name, place.0))
        })
        .collect();
    Ok(Ranked {
        rows: output.count.0,
        ranked: Cow::Owned(ranked),
    })
}
const fn ranked(ranked: &'static [(&'static str, i64)]) -> Expect<Value, Failure> {
    Expect::Value(Ranked {
        rows: ranked.len(),
        ranked: Cow::Borrowed(ranked),
    })
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_text_takes_the_place_of_the_first_set_that_holds_it"),
        word: Word::Succeeds,
        input: || {
            (
                &["cli", "core", "sdk"],
                Some(sets(&[&["cli"], &["sdk", "core"], &["core"]])),
            )
        },
        expect: ranked(&[("cli", 0), ("core", 1), ("sdk", 1)]),
    },
    Case {
        name: CaseName("a_text_no_set_holds_is_dropped"),
        word: Word::Succeeds,
        input: || (&["cli", "leaf"], Some(sets(&[&["cli"], &["core"]]))),
        expect: ranked(&[("cli", 0)]),
    },
    Case {
        name: CaseName("no_sets_drop_every_row"),
        word: Word::Succeeds,
        input: || (&["cli"], Some(sets(&[]))),
        expect: ranked(&[]),
    },
    Case {
        name: CaseName("a_rank_parameter_nothing_bound_is_refused"),
        word: Word::Refuses,
        input: || (&["cli"], None),
        expect: Expect::Error(OperatingError::Unbound),
    },
    Case {
        name: CaseName("a_rank_parameter_bound_to_a_text_is_refused"),
        word: Word::Refuses,
        input: || {
            (
                &["cli"],
                Some(ParameterBinding::Text(LiteralText("cli".into()))),
            )
        },
        expect: Expect::Error(OperatingError::WrongBinding),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
