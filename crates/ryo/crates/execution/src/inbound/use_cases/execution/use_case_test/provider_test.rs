//! Binary operators must demand the refusing RHS after a real nonempty successful LHS.
use super::super::{
    fixtures,
    types::{OperatingError, SourceRows},
};
use ryo_relations::domain::relations::{
    service::{kinds, node_id},
    types::{FileId, Offset, Position, Relation, Span, Value as Cell},
};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{ColumnIndex, Input as PlanInput, StepId},
        plans::types::Operation,
        rows::types::{RowCount, Table},
    },
};
use std::cell::RefCell;
#[derive(Clone, Copy)]
enum Binary {
    Union,
    Intersection,
    Difference,
    Antijoin,
}
#[derive(Clone, Copy)]
enum Missing {
    First,
    Second,
}
type Input = (Binary, Missing);
type Value = ();
type Failure = OperatingError;
fn subject((operator, missing): Input) -> Result<Value, Failure> {
    let (elements, mut symbols) = fixtures::tree_with_names();
    // Actual Function declaration and valid typed cells; no numeric catalog stand-in.
    let function = Table {
        kinds: kinds(Relation::Function).to_vec(),
        count: RowCount(1),
        cells: vec![
            Cell::Node(node_id(FileId(0), Position(1))),
            Cell::Symbol(ryo_relations::domain::terms::types::Symbol(0)),
            Cell::File(FileId(0)),
            Cell::Span(Span {
                start: Offset(0),
                end: Offset(1),
            }),
        ],
    };
    let reads = RefCell::new(Vec::new());
    let read = |relation| {
        reads.borrow_mut().push(relation);
        match relation {
            Relation::Function if matches!(missing, Missing::Second) => {
                Ok(SourceRows::Computed(&function))
            }
            _ => Err(OperatingError::NotRead),
        }
    };
    let left = PlanInput::Step(StepId(0));
    let right = PlanInput::Step(StepId(1));
    let operation = match operator {
        Binary::Union => Operation::Union { left, right },
        Binary::Intersection => Operation::Intersection { left, right },
        Binary::Difference => Operation::Difference { left, right },
        Binary::Antijoin => Operation::Antijoin {
            left,
            right,
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
        },
    };
    let plan = fixtures::plan(vec![
        Operation::Select {
            from: PlanInput::Relation(Relation::Function),
            comparisons: vec![],
            kept: vec![ColumnIndex(2)],
        },
        Operation::Select {
            from: PlanInput::Relation(Relation::Comment),
            comparisons: vec![],
            kept: vec![ColumnIndex(2)],
        },
        operation,
    ]);
    let result = fixtures::published_from(
        plan,
        vec![],
        vec![],
        b"abcd",
        &elements,
        &mut symbols,
        &read,
    );
    let history = reads.into_inner();
    // Pinned successful LHS borrows may repeat, but refusal is terminal.
    match missing {
        Missing::First => assert_eq!(history, [Relation::Function]),
        Missing::Second => {
            let (last, prefix) = history.split_last().expect("RHS must actually be demanded");
            assert_eq!(*last, Relation::Comment);
            assert!(
                !prefix.is_empty(),
                "nonempty successful LHS is demanded first"
            );
            assert!(
                prefix
                    .iter()
                    .all(|relation| *relation == Relation::Function),
                "RHS refusal occurs exactly once, with no subsequent producer read"
            );
        }
    }
    result.map(|_| ())
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("union_requires_rhs_after_successful_lhs"),
        word: Word::Refuses,
        input: || (Binary::Union, Missing::Second),
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("intersection_requires_rhs_after_successful_lhs"),
        word: Word::Refuses,
        input: || (Binary::Intersection, Missing::Second),
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("difference_requires_rhs_after_successful_lhs"),
        word: Word::Refuses,
        input: || (Binary::Difference, Missing::Second),
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("antijoin_requires_rhs_after_successful_lhs"),
        word: Word::Refuses,
        input: || (Binary::Antijoin, Missing::Second),
        expect: Expect::Error(OperatingError::NotRead),
    },
    Case {
        name: CaseName("antijoin_first_provider_failure_prevents_rhs_demand"),
        word: Word::Refuses,
        input: || (Binary::Antijoin, Missing::First),
        expect: Expect::Error(OperatingError::NotRead),
    },
];
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
