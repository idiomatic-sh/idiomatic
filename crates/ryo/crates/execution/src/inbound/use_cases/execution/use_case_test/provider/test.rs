//! Binary operators must demand the refusing RHS after a real nonempty successful LHS.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_relations::domain::relations::types::ColumnsView;
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
use std::sync::Mutex;
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
    Neither,
}
use std::borrow::Cow;
/// The relations the providers were asked for, in order.
type History = Cow<'static, [Relation]>;
/// The provider reads.
#[derive(Debug, PartialEq, Eq)]
struct Delivered {
    history: History,
}
/// A refused rule: the error and what was delivered before and after it.
#[derive(Debug, PartialEq, Eq)]
struct Refused {
    error: OperatingError,
    delivered: Delivered,
}
type Input = (Binary, Missing);
type Value = Delivered;
type Failure = Refused;
fn subject((operator, missing): Input) -> Result<Value, Failure> {
    let (elements, mut symbols) = fixtures::requests::tree_with_names();
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
    let comment = Table {
        kinds: kinds(Relation::Comment).to_vec(),
        count: RowCount(0),
        cells: vec![],
    };
    let (function, comment) = (
        fixtures::facts::columns_of(&function),
        fixtures::facts::columns_of(&comment),
    );
    let reads = Mutex::new(Vec::new());
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "the fake serves the two relations the rule reads; every other relation is not read"
    )]
    let read = |relation| {
        reads.lock().expect("the reads").push(relation);
        match relation {
            Relation::Function if matches!(missing, Missing::Second | Missing::Neither) => {
                Ok(ColumnsView {
                    kinds: kinds(relation),
                    columns: function.as_slice(),
                })
            }
            Relation::Comment if matches!(missing, Missing::Neither) => Ok(ColumnsView {
                kinds: kinds(relation),
                columns: comment.as_slice(),
            }),
            _ => Err(fixtures::facts::not_held()),
        }
    };
    let facts = fixtures::facts::ReadFacts::new(read);
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
    let plan = inputs::plan(vec![
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
    let result = fixtures::requests::published_from(
        plan,
        vec![],
        vec![],
        b"abcd",
        &elements,
        &mut symbols,
        &facts,
    )
    .map(|_| ());
    let delivered = Delivered {
        history: reads.into_inner().expect("the reads").into(),
    };
    match result.err() {
        Some(error) => Err(Refused { error, delivered }),
        None => Ok(delivered),
    }
}
/// The reads of a rule whose left provider answers: the left, the right, then the left's pinned borrows again.
const BOTH_READ: &[Relation] = &[
    Relation::Function,
    Relation::Comment,
    Relation::Function,
    Relation::Function,
    Relation::Function,
];
const fn refused(history: &'static [Relation]) -> Expect<Value, Failure> {
    Expect::Error(Refused {
        error: OperatingError::NotRead,
        delivered: Delivered {
            history: Cow::Borrowed(history),
        },
    })
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("union_over_two_available_providers_publishes"),
        word: Word::Succeeds,
        input: || (Binary::Union, Missing::Neither),
        expect: Expect::Value(Delivered {
            history: Cow::Borrowed(BOTH_READ),
        }),
    },
    Case {
        name: CaseName("union_requires_rhs_after_successful_lhs"),
        word: Word::Refuses,
        input: || (Binary::Union, Missing::Second),
        expect: refused(BOTH_READ),
    },
    Case {
        name: CaseName("intersection_requires_rhs_after_successful_lhs"),
        word: Word::Refuses,
        input: || (Binary::Intersection, Missing::Second),
        expect: refused(BOTH_READ),
    },
    Case {
        name: CaseName("difference_requires_rhs_after_successful_lhs"),
        word: Word::Refuses,
        input: || (Binary::Difference, Missing::Second),
        expect: refused(BOTH_READ),
    },
    Case {
        name: CaseName("antijoin_requires_rhs_after_successful_lhs"),
        word: Word::Refuses,
        input: || (Binary::Antijoin, Missing::Second),
        expect: refused(BOTH_READ),
    },
    Case {
        name: CaseName("antijoin_first_provider_failure_prevents_rhs_demand"),
        word: Word::Refuses,
        input: || (Binary::Antijoin, Missing::First),
        expect: refused(&[Relation::Function]),
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
