//! Proves the `GraphWalks` value's behaviour over an invocation that acquired no partition and no
//! depth-first graph.

use super::super::row_delivery::value::RowDelivery;
use super::value::GraphWalks;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::primitive::types::PreparedInputs;
use crate::domain::operator::contract::types::PrimitiveWorkspace;
use crate::domain::operator::fixtures::{Kept, Resources, extend, with_lease};
use crate::domain::table_model::types::Verdict;
use ryo_operators::domain::closure::types::PartitionPlace;
use ryo_relations::domain::relations::types::{Count, Integer, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// What a case asks of the walks of an evaluation.
type Input = for<'f> fn(&mut GraphWalks<'f>, &mut RowDelivery<'_, 'f>) -> Result<Count, ()>;
/// How many partitions, sources or reached vertices the walks answered.
type Value = Count;
/// The engine error the delivery kept.
type Failure = Option<OperatingError>;

fn subject(case: Input) -> Result<Value, Failure> {
    let resources = Resources::default();
    let mut workspace = PrimitiveWorkspace::default();
    let operation = extend();
    let mut sink = Kept::default();
    with_lease(&resources, Verdict::Holds, &mut workspace, |lease| {
        let mut walks = GraphWalks::default();
        let mut rows = RowDelivery::new(lease, &mut sink, &operation, PreparedInputs::Views(&[]));
        case(&mut walks, &mut rows).map_err(|()| rows.take_failure())
    })
}

/// The rows of this value's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_empty_closure_has_no_partition"),
        word: Word::Succeeds,
        input: || |walks, rows| walks.partitions(rows).map_err(|_| ()),
        expect: Expect::Value(Count(0)),
    },
    Case {
        name: CaseName("a_partition_outside_the_closure_has_no_source"),
        word: Word::Succeeds,
        input: || {
            |walks, rows| {
                let _partitions = walks.partitions(rows).map_err(|_| ())?;
                Ok(walks.sources(PartitionPlace(0)))
            }
        },
        expect: Expect::Value(Count(0)),
    },
    Case {
        name: CaseName("a_reach_outside_the_closure_is_refused"),
        word: Word::Refuses,
        input: || {
            |walks, rows| {
                walks
                    .reach(rows, PartitionPlace(0), Cell::Number(Integer(1)))
                    .map_err(|_| ())
            }
        },
        expect: Expect::Error(Some(OperatingError::ShapeMismatch)),
    },
    Case {
        name: CaseName("roots_need_a_depth_first_graph"),
        word: Word::Refuses,
        input: || {
            |walks, rows| {
                let _roots = walks.roots(rows).map_err(|_| ())?;
                Ok(Count(0))
            }
        },
        expect: Expect::Error(Some(OperatingError::WrongBinding)),
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
