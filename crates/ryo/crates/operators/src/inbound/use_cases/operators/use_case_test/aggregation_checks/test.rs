//! Public aggregation metadata checks keep the kinds of their keys in an engine-owned target and check columns in the order
//! the aggregation reads them.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::*;
use crate::domain::reduction::types::*;
use crate::inbound::use_cases::operators::types::*;
use crate::inbound::use_cases::operators::{fixtures, interface::OperatorsUseCase};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::borrow::Cow;

#[derive(Clone, Copy)]
enum Scenario {
    CheckCount,
    CheckDigest,
    CheckStreamCountOrder,
    CheckCollectedCountOrder,
    CheckStreamDigestOrder,
}
#[derive(Debug, PartialEq)]
enum Observation {
    Kinds(Cow<'static, [CellKind]>),
}
type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("count_metadata_keeps_repeated_key_kinds"),
        word: Word::Succeeds,
        input: || Scenario::CheckCount,
        expect: Expect::Value(Observation::Kinds(Cow::Borrowed(&[
            CellKind::Number,
            CellKind::Number,
        ]))),
    },
    Case {
        name: CaseName("digest_metadata_keeps_key_kinds"),
        word: Word::Succeeds,
        input: || Scenario::CheckDigest,
        expect: Expect::Value(Observation::Kinds(Cow::Borrowed(&[CellKind::Number]))),
    },
    Case {
        name: CaseName("streamed_count_metadata_checks_keys_first"),
        word: Word::Refuses,
        input: || Scenario::CheckStreamCountOrder,
        expect: Expect::Error(Failure::NoSuchColumn {
            index: ryo_values::domain::plan_parts::types::ColumnIndex(99),
        }),
    },
    Case {
        name: CaseName("collected_count_metadata_checks_operand_first"),
        word: Word::Refuses,
        input: || Scenario::CheckCollectedCountOrder,
        expect: Expect::Error(Failure::NoSuchColumn {
            index: ryo_values::domain::plan_parts::types::ColumnIndex(98),
        }),
    },
    Case {
        name: CaseName("streamed_digest_metadata_checks_operand_first"),
        word: Word::Refuses,
        input: || Scenario::CheckStreamDigestOrder,
        expect: Expect::Error(Failure::NoSuchColumn {
            index: ryo_values::domain::plan_parts::types::ColumnIndex(98),
        }),
    },
];
fn written(call: OperatorCall<'_>) -> Result<(), Failure> {
    assert_eq!(
        fixtures::operators().operators(call)?,
        OperatorAnswer::Written
    );
    Ok(())
}
fn check(input: Input) -> Result<Value, Failure> {
    let mut kinds = Vec::with_capacity(4);
    let original = kinds.as_ptr();
    let (program, order) = match input {
        Scenario::CheckCount => (
            AggregationPlan::Count {
                keys: &[ColumnIndex(1), ColumnIndex(1)],
                distinct: Some(ColumnIndex(0)),
            },
            AggregationCheckOrder::Streaming,
        ),
        Scenario::CheckDigest => (
            AggregationPlan::Digest {
                keys: &[ColumnIndex(1)],
                over: ColumnIndex(0),
            },
            AggregationCheckOrder::Streaming,
        ),
        Scenario::CheckStreamDigestOrder => (
            AggregationPlan::Digest {
                keys: &[ColumnIndex(99)],
                over: ColumnIndex(98),
            },
            AggregationCheckOrder::Streaming,
        ),
        Scenario::CheckCollectedCountOrder => (
            AggregationPlan::Count {
                keys: &[ColumnIndex(99)],
                distinct: Some(ColumnIndex(98)),
            },
            AggregationCheckOrder::Collected,
        ),
        Scenario::CheckStreamCountOrder => (
            AggregationPlan::Count {
                keys: &[ColumnIndex(99)],
                distinct: Some(ColumnIndex(98)),
            },
            AggregationCheckOrder::Streaming,
        ),
    };
    written(OperatorCall::CheckAggregation {
        program,
        kinds: &[CellKind::Number; 2],
        order,
        target: &mut kinds,
    })?;
    assert_eq!(kinds.as_ptr(), original);
    Ok(Observation::Kinds(Cow::Owned(kinds)))
}
fn subject(input: Input) -> Result<Value, Failure> {
    check(input)
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
