//! The first layer wins, including duplicate membership across layers.
use super::service::rank;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::row_texts::types::CellText;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::LiteralText;
type Input = &'static str;
type Value = Option<Integer>;
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("first_duplicate_layer_wins"),
        word: Word::Succeeds,
        input: || "same",
        expect: Expect::Value(Some(Integer(0))),
    },
    Case {
        name: CaseName("later_layer_is_its_index"),
        word: Word::Succeeds,
        input: || "later",
        expect: Expect::Value(Some(Integer(1))),
    },
    Case {
        name: CaseName("unmatched_text_has_no_rank"),
        word: Word::Succeeds,
        input: || "absent",
        expect: Expect::Value(None),
    },
];
fn subject(text: Input) -> Result<Value, Failure> {
    let layers = [
        vec![LiteralText("same".into())],
        vec![LiteralText("same".into()), LiteralText("later".into())],
    ];
    Ok(rank(CellText(text), &layers))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
