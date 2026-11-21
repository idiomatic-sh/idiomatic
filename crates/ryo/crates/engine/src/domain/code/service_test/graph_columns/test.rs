//! The records of the columns a graph fill's kernel is told: each written through its constructor and read back through
//! its reader at the place the constructor answered, after other slots of the pool; a place the pool does not reach, a
//! record the pool holds part of, and a slot that names no column read as none.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::records::service::{
    edge_columns_at, push_edge_columns, push_visit_columns, push_worklist_columns,
    visit_columns_at, worklist_columns_at,
};
use crate::domain::code::records::types::{EdgeColumns, VisitColumns, WorklistColumns};
use crate::domain::code::types::{PoolAt, WordSlot};
use crate::domain::steps::types::MadeColumn;
use std::borrow::Cow;

/// What a row writes and reads back.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The edge columns of a `follow` or a `cycles` fill.
    Edges,
    /// The four columns of an `ordered-dfs` fill.
    Visits,
    /// The eight columns of a `worklist` fill.
    Worklists,
}

type Input = Scenario;
type Value = Cow<'static, str>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "the_edge_columns_read_back_and_a_record_the_pool_does_not_hold_whole_is_none",
        ),
        word: Word::Succeeds,
        input: || Scenario::Edges,
        expect: Expect::Value(Cow::Borrowed(
            "at PoolAt(3) Some(EdgeColumns { source: MadeColumn(4), destination: MadeColumn(65535) }), as written: true; past the pool: None; one slot short: None; a slot no column has: None",
        )),
    },
    Case {
        name: CaseName("the_four_columns_of_an_ordered_walk_read_back_each_from_its_own_slot"),
        word: Word::Succeeds,
        input: || Scenario::Visits,
        expect: Expect::Value(Cow::Borrowed(
            "at PoolAt(3) Some(VisitColumns { root: MadeColumn(1), source: MadeColumn(2), destination: MadeColumn(3), edge: MadeColumn(4) }), as written: true; the slots [WordSlot(1), WordSlot(2), WordSlot(3), WordSlot(4)]; one slot short: None",
        )),
    },
    Case {
        name: CaseName("the_eight_columns_of_a_worklist_read_back_each_from_its_own_slot"),
        word: Word::Succeeds,
        input: || Scenario::Worklists,
        expect: Expect::Value(Cow::Borrowed(
            "at PoolAt(3) Some(WorklistColumns { root: MadeColumn(1), state: MadeColumn(2), source: MadeColumn(3), destination: MadeColumn(4), before: MadeColumn(5), after: MadeColumn(6), lower: MadeColumn(7), upper: MadeColumn(8) }), as written: true; the slots [WordSlot(1), WordSlot(2), WordSlot(3), WordSlot(4), WordSlot(5), WordSlot(6), WordSlot(7), WordSlot(8)]; one slot short: None",
        )),
    },
];

/// The edge columns written after three slots of the pool, read back; then read a place too far, one slot short of the
/// record, and with a slot no column has in the destination's place.
fn edges() -> String {
    let mut pool = vec![WordSlot(9); 3];
    let wrote = EdgeColumns {
        source: MadeColumn(4),
        destination: MadeColumn(65_535),
    };
    let at = push_edge_columns(&mut pool, wrote);
    let read = edge_columns_at(&pool, at);
    let short = edge_columns_at(pool.get(..4).unwrap_or(&[]), at);
    if let Some(last) = pool.last_mut() {
        *last = WordSlot(65_536);
    }
    format!(
        "at {at:?} {read:?}, as written: {}; past the pool: {:?}; one slot short: {short:?}; a slot no column has: {:?}",
        read == Some(wrote),
        edge_columns_at(&pool, PoolAt(99)),
        edge_columns_at(&pool, at),
    )
}

/// The four columns of an ordered walk, each another number, written after three slots of the pool and read back; then
/// read one slot short of the record.
fn visits() -> String {
    let mut pool = vec![WordSlot(9); 3];
    let wrote = VisitColumns {
        root: MadeColumn(1),
        source: MadeColumn(2),
        destination: MadeColumn(3),
        edge: MadeColumn(4),
    };
    let at = push_visit_columns(&mut pool, wrote);
    let read = visit_columns_at(&pool, at);
    let short = visit_columns_at(pool.get(..6).unwrap_or(&[]), at);
    format!(
        "at {at:?} {read:?}, as written: {}; the slots {:?}; one slot short: {short:?}",
        read == Some(wrote),
        pool.get(3..).unwrap_or(&[]),
    )
}

/// The eight columns of a worklist, each another number, written after three slots of the pool and read back; then read
/// one slot short of the record.
fn worklists() -> String {
    let mut pool = vec![WordSlot(9); 3];
    let wrote = WorklistColumns {
        root: MadeColumn(1),
        state: MadeColumn(2),
        source: MadeColumn(3),
        destination: MadeColumn(4),
        before: MadeColumn(5),
        after: MadeColumn(6),
        lower: MadeColumn(7),
        upper: MadeColumn(8),
    };
    let at = push_worklist_columns(&mut pool, wrote);
    let read = worklist_columns_at(&pool, at);
    let short = worklist_columns_at(pool.get(..10).unwrap_or(&[]), at);
    format!(
        "at {at:?} {read:?}, as written: {}; the slots {:?}; one slot short: {short:?}",
        read == Some(wrote),
        pool.get(3..).unwrap_or(&[]),
    )
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(Cow::Owned(match scenario {
        Scenario::Edges => edges(),
        Scenario::Visits => visits(),
        Scenario::Worklists => worklists(),
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
