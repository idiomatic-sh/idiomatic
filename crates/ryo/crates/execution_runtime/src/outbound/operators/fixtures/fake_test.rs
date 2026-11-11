//! The operator fake preserves the port contract and a controlled refusal.
use super::{super::contract, OperatorsFake};
use crate::outbound::operators::interface::OperatorsPort as _;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_relations::domain::relations::types::{Integer, Value as CellValue};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::borrow::Cow;
use std::sync::atomic::Ordering;

type Input = bool;
/// What one projection through the fake left behind: its answer, the calls it recorded and the target row.
#[derive(Debug, PartialEq)]
struct Observed {
    answer: Result<OperatorAnswer, PortableOperatorFault>,
    calls: usize,
    target: Cow<'static, [CellValue]>,
}
type Value = Observed;
type Failure = Observed;

const INITIAL: CellValue = CellValue::Number(Integer(9));
const ROW: CellValue = CellValue::Number(Integer(3));

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("available_fake_projects_and_records_once"),
        word: Word::Succeeds,
        input: || false,
        expect: Expect::Value(Observed {
            answer: Ok(OperatorAnswer::Written),
            calls: 1,
            target: Cow::Borrowed(&[INITIAL, ROW]),
        }),
    },
    Case {
        name: CaseName("refusing_fake_keeps_scratch_and_records_once"),
        word: Word::Refuses,
        input: || true,
        expect: Expect::Error(Observed {
            answer: Err(PortableOperatorFault::NoSuchColumn {
                index: ColumnIndex(8),
            }),
            calls: 1,
            target: Cow::Borrowed(&[INITIAL]),
        }),
    },
];

fn subject(refuses: Input) -> Result<Value, Failure> {
    let fake = OperatorsFake {
        refuses,
        ..OperatorsFake::default()
    };
    let row = [ROW];
    let mut target = vec![INITIAL];
    let answer = fake.operators(OperatorCall::Project {
        row: &row,
        columns: &[ColumnIndex(0)],
        target: &mut target,
    });
    let observed = Observed {
        answer,
        calls: fake.calls.load(Ordering::Relaxed),
        target: Cow::Owned(target),
    };
    if observed.answer.as_ref().err().is_some() {
        Err(observed)
    } else {
        Ok(observed)
    }
}
#[test]
fn succeeds() {
    contract::check(&OperatorsFake::default());
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
