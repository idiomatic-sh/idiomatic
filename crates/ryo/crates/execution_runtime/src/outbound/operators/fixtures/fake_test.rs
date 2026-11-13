//! The operator fake preserves the port contract and a controlled refusal.
use super::{
    super::{
        contract::{self, count},
        reduce_count::Groups,
    },
    OperatorsFake,
};
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::borrow::Cow;
use std::sync::atomic::Ordering;

type Input = bool;
/// What one count through the fake left behind: its answer, the calls it recorded and the groups it wrote.
#[derive(Debug, PartialEq)]
struct Observed {
    answer: Result<OperatorAnswer, PortableOperatorFault>,
    calls: usize,
    groups: Cow<'static, [(usize, usize)]>,
}
type Value = Observed;
type Failure = Observed;

const ROWS: [[i64; 2]; 2] = [[3, 1], [3, 2]];

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("available_fake_counts_and_records_once"),
        word: Word::Succeeds,
        input: || false,
        expect: Expect::Value(Observed {
            answer: Ok(OperatorAnswer::Written),
            calls: 1,
            groups: Cow::Borrowed(&[(0, 2)]),
        }),
    },
    Case {
        name: CaseName("refusing_fake_writes_no_group_and_records_once"),
        word: Word::Refuses,
        input: || true,
        expect: Expect::Error(Observed {
            answer: Err(PortableOperatorFault::NoSuchColumn {
                index: ColumnIndex(8),
            }),
            calls: 1,
            groups: Cow::Borrowed(&[]),
        }),
    },
];

fn subject(refuses: Input) -> Result<Value, Failure> {
    let fake = OperatorsFake {
        refuses,
        ..OperatorsFake::default()
    };
    let mut groups = Groups::default();
    let answer = count(&fake, &ROWS, &[ColumnIndex(0)], &mut groups);
    let observed = Observed {
        answer,
        calls: fake.calls.load(Ordering::Relaxed),
        groups: Cow::Owned(groups.0),
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
