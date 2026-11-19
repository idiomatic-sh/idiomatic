//! Real consumers keep duplicate rows in input order through file rebinding and index retirement.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::row_predicates::{
    service::{bind, matches_row, prepare, release, select},
    types::{
        Access, Equality, Instruction, LiteralId, MatchDemand, PredicateSelection, SelectionInput,
    },
};
use ryo_relations::domain::relations::types::Bits;
use ryo_values::domain::plan_parts::types::{ColumnIndex, StepId};
use ryo_values::domain::table_words::types::{ColumnCount, WordSlice};

type Input = MatchDemand;
type Value = [Option<Bits>; 3];
type Failure = std::convert::Infallible;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("duplicate_rows_in_source_order_after_release"),
        word: Word::Succeeds,
        input: || MatchDemand::Rows,
        expect: Expect::Value([Some(Bits(10)), Some(Bits(12)), None]),
    },
    Case {
        name: CaseName("first_witness_after_rebinding"),
        word: Word::Succeeds,
        input: || MatchDemand::First,
        expect: Expect::Value([Some(Bits(10)), None, None]),
    },
];

fn subject(demand: Input) -> Result<Value, Failure> {
    let selector = || PredicateSelection {
        source: SelectionInput::Derived(StepId(0)),
        instructions: vec![Instruction::Word(
            ColumnIndex(0),
            LiteralId(0),
            Equality::Equal,
        )],
    };
    let program = prepare(vec![PredicateSelection::default(), selector(), selector()]);
    let source = WordSlice(&[7, 10, 9, 11, 7, 12]);
    let mut previous = bind(&program, vec![Some(Bits(9))], Access::Shared);
    assert_eq!(
        matches_row(&previous, StepId(1), WordSlice(&source.0[..2]), |_| {
            PredicateOutcome::Holds
        }),
        PredicateOutcome::Fails
    );
    for node in [StepId(1), StepId(2)] {
        select(
            &mut previous,
            node,
            source,
            ColumnCount(2),
            MatchDemand::Rows,
            |_, _| PredicateOutcome::Holds,
            |_| {},
        );
    }
    release(&mut previous, StepId(0));
    assert_eq!(previous.counts.builds.0, 1);

    let mut current = bind(&program, vec![Some(Bits(7))], Access::Shared);
    select(
        &mut current,
        StepId(1),
        source,
        ColumnCount(2),
        demand,
        |_, _| PredicateOutcome::Holds,
        |_| {},
    );
    let mut rows = Vec::new();
    select(
        &mut current,
        StepId(2),
        source,
        ColumnCount(2),
        demand,
        |_, _| PredicateOutcome::Holds,
        |row| rows.push(Bits(row.0[1])),
    );
    assert_eq!(current.counts.indexed_reads.0, 1);
    Ok([
        rows.first().copied(),
        rows.get(1).copied(),
        rows.get(2).copied(),
    ])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
