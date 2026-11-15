//! The operator fake preserves the port contract and a controlled refusal.
use super::{
    super::contract::{self, count},
    OperatorsFake,
};
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::borrow::Cow;
use std::sync::atomic::Ordering;

type Input = bool;
/// What one count through the fake left behind: the groups it answered, or its refusal, and the runs it recorded.
#[derive(Debug, PartialEq)]
struct Observed {
    answer: Result<Cow<'static, [(i64, i64)]>, PortableOperatorFault>,
    calls: usize,
}
type Value = Observed;
type Failure = Observed;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("available_fake_counts_and_records_once"),
        word: Word::Succeeds,
        input: || false,
        expect: Expect::Value(Observed {
            answer: Ok(Cow::Borrowed(&[(3, 2)])),
            calls: 1,
        }),
    },
    Case {
        name: CaseName("refusing_fake_answers_no_group_and_records_once"),
        word: Word::Refuses,
        input: || true,
        expect: Expect::Error(Observed {
            answer: Err(PortableOperatorFault::NoSuchColumn {
                index: ColumnIndex(8),
            }),
            calls: 1,
        }),
    },
];

fn subject(refuses: Input) -> Result<Value, Failure> {
    let fake = OperatorsFake {
        refuses,
        ..OperatorsFake::default()
    };
    let answer = count(&fake, &[3, 3], ColumnIndex(2)).map(Cow::Owned);
    let observed = Observed {
        answer,
        calls: fake.calls.load(Ordering::Relaxed),
    };
    if observed.answer.is_err() {
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
