//! The operator fake preserves the port contract and a controlled refusal.
use super::{super::contract, OperatorsFake};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase as _;
use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_operators::domain::contract::types::PortableOperatorFault;
use ryo_relations::domain::relations::types::{Integer, Value as CellValue};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::atomic::Ordering;

type Input = bool;
type Value = usize;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("available_fake_projects_and_records_once"),
        word: Word::Succeeds,
        input: || false,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("refusing_fake_keeps_scratch_and_records_once"),
        word: Word::Refuses,
        input: || true,
        expect: Expect::Error(PortableOperatorFault::NoSuchColumn { index: ryo_values::domain::plan_parts::types::ColumnIndex(8) }),
    },
];

#[test]
fn fake_obeys_contract_and_refusal_boundary() {
    contract::check(&OperatorsFake::default());
    let subject = |refuses: Input| {
        let fake = OperatorsFake {
            refuses,
            ..OperatorsFake::default()
        };
        let row = [CellValue::Number(Integer(3))];
        let initial = CellValue::Number(Integer(9));
        let mut target = vec![initial];
        let result = fake.operators(OperatorCall::Project {
            row: &row,
            columns: &[ColumnIndex(0)],
            target: &mut target,
        });
        assert_eq!(fake.calls.load(Ordering::Relaxed), 1);
        if refuses {
            assert_eq!(target, vec![initial]);
        } else {
            assert_eq!(result, Ok(OperatorAnswer::Written));
            assert_eq!(target, vec![initial, row[0]]);
        }
        result.map(|_| 1)
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
