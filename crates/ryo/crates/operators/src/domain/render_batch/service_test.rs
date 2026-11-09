//! Unsupported refuses the whole batch before a row; not-applicable skips only its own.
use super::service::deliver;
use super::types::{RenderAnswer, RenderedOutput};
use crate::domain::contract::errors::ContractError;
use crate::domain::contract::types::PortableOperatorFault;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::types::RowIndex;
struct Collect(Vec<String>);
impl RenderedOutput for Collect {
    fn text(&mut self, position: RowIndex, text: CellText<'_>) -> Result<(), ContractError> {
        self.0.push(format!("{}:{}", position.0, text.0));
        Ok(())
    }
}
type Input = &'static [RenderAnswer<'static>];
type Value = std::borrow::Cow<'static, str>;
type Failure = PortableOperatorFault;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("known_texts_keep_their_positions"),
        word: Word::Succeeds,
        input: || {
            &[
                RenderAnswer::Known(CellText("a")),
                RenderAnswer::NotApplicable,
                RenderAnswer::Known(CellText("c")),
            ]
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed("0:a,2:c")),
    },
    Case {
        name: CaseName("an_unsupported_answer_refuses_before_any_delivery"),
        word: Word::Refuses,
        input: || {
            &[
                RenderAnswer::Known(CellText("a")),
                RenderAnswer::Unsupported,
            ]
        },
        expect: Expect::Error(PortableOperatorFault::UnsupportedRender),
    },
];
fn subject(answers: Input) -> Result<Value, Failure> {
    let mut out = Collect(Vec::new());
    deliver(answers, &mut out)?;
    Ok(std::borrow::Cow::Owned(out.0.join(",")))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
