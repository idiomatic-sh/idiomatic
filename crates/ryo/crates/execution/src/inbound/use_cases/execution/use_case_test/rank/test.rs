//! Rank binding and row-retention contracts through the compiled public owner.
use super::super::super::fixtures;
use crate::application::file_dag::acceptance::operator_contract_test::fixtures::inputs;
use crate::inbound::use_cases::execution::interface::OperatingError;
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
type Value = Cow<'static, [(&'static str, i64)]>;
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
    let output = fixtures::published(
        plan,
        binding.into_iter().collect(),
        vec![fixtures::texts("names", names)],
        &mut symbols,
    )?;
    let found: Vec<_> = output
        .cells
        .chunks_exact(2)
        .map(|row| {
            let Cell::Number(place) = row[1] else {
                panic!("rank must append a numeric layer")
            };
            let text = inputs::text(row[0], &symbols);
            let name = names
                .iter()
                .copied()
                .find(|name| *name == text)
                .expect("rank preserves original text");
            (name, place.0)
        })
        .collect();
    assert_eq!(found.len(), output.count.0);
    Ok(Cow::Owned(found))
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
        expect: Expect::Value(Cow::Borrowed(&[("cli", 0), ("core", 1), ("sdk", 1)])),
    },
    Case {
        name: CaseName("a_text_no_set_holds_is_dropped"),
        word: Word::Succeeds,
        input: || (&["cli", "leaf"], Some(sets(&[&["cli"], &["core"]]))),
        expect: Expect::Value(Cow::Borrowed(&[("cli", 0)])),
    },
    Case {
        name: CaseName("no_sets_drop_every_row"),
        word: Word::Succeeds,
        input: || (&["cli"], Some(sets(&[]))),
        expect: Expect::Value(Cow::Borrowed(&[])),
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
