//! Behavioral proof of `partition_views`: a partitioned closure is admitted only from an ordering and a closure plan of
//! their own kinds, and walks one partition per key of the ordering.
use super::super::admitted_views::fixtures::{context, owner, table, use_};
use crate::application::file_dag::closure_partitions::types::PartitionAdvance;
use crate::application::file_dag::relations::handle::service::Publication;
use crate::application::file_dag::workspace::service::InvocationWorkspace;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::derived_views::types::ViewRequest;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::borrow::Cow;
use std::rc::Rc;

/// The plans the closure of the edges `1 -> 2`, `2 -> 3` in key 5 and `9 -> 10` in key 7 is prepared with.
#[derive(Clone, Copy, Debug)]
enum Plans {
    /// The ordering by key and the closure adjacency of source to destination.
    Own,
    /// The ordering given as the closure plan too.
    OrderingAsTheClosure,
    /// An equality on the key given as the ordering.
    EqualityAsTheOrdering,
}

type Input = Plans;

/// The rows of each partition walked, in order.
type Value = Cow<'static, [usize]>;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_key_of_the_ordering_is_one_partition"),
        word: Word::Succeeds,
        input: || Plans::Own,
        expect: Expect::Value(Cow::Borrowed(&[2, 1])),
    },
    Case {
        name: CaseName("a_closure_plan_of_another_kind_is_the_wrong_operation"),
        word: Word::Refuses,
        input: || Plans::OrderingAsTheClosure,
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("an_ordering_of_another_kind_is_the_wrong_operation"),
        word: Word::Refuses,
        input: || Plans::EqualityAsTheOrdering,
        expect: Expect::Error(OperatingError::WrongOperation),
    },
];

/// Publishes the edges `(from, to, key)`, prepares their partitioned closure with `plans` and walks it to the end.
fn subject(plans: Plans) -> Result<Value, Failure> {
    let owner = owner();
    let mut workspace = InvocationWorkspace::new(Rc::clone(&owner));
    let (mut symbols, mut scratch, mut memo) = (Symbols::default(), Default::default(), vec![]);
    let mut context = context(&mut symbols, &mut scratch, &mut memo);
    let published = owner.computed(table(&[1, 2, 5, 2, 3, 5, 9, 10, 7], 3))?;
    let rows = owner.resolve(published.as_ref(), &|_| Err(OperatingError::NotRead))?;
    workspace.capture_admitted(vec![Publication::of(published.clone())]);
    let inputs = [RepresentationInput::from(&rows)];
    let ordering = use_(
        &ViewRequest::PartitionOrder {
            keys: vec![ColumnIndex(2)],
        },
        3,
        1,
    );
    let adjacency = use_(
        &ViewRequest::ClosureAdjacency {
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
        },
        3,
        1,
    );
    let equality = use_(
        &ViewRequest::Equality {
            columns: vec![ColumnIndex(2)],
        },
        3,
        1,
    );
    let (order, closure) = match plans {
        Plans::Own => (&ordering, &adjacency),
        Plans::OrderingAsTheClosure => (&ordering, &ordering),
        Plans::EqualityAsTheOrdering => (&equality, &adjacency),
    };
    let mut cursor =
        workspace
            .inputs(&inputs)
            .partitioned(order, closure, inputs[0], &mut context)?;
    let mut walked = Vec::new();
    while cursor.advance()? == PartitionAdvance::Advanced {
        walked.push(cursor.current()?.table.count.0);
    }
    Ok(Cow::Owned(walked))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
