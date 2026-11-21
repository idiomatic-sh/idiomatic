//! The records of the columns a graph fill's kernel is told: each written through its constructor and read back through
//! its reader at the place the constructor answered, after other slots of the pool; a place the pool does not reach, a
//! record the pool holds part of, and a slot that names no column read as none.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::records::service::{edge_columns_at, push_edge_columns};
use crate::domain::code::records::types::EdgeColumns;
use crate::domain::code::types::{PoolAt, WordSlot};
use crate::domain::steps::types::MadeColumn;
use std::borrow::Cow;

/// What a row writes and reads back.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The edge columns of a `follow` or a `cycles` fill.
    Edges,
}

type Input = Scenario;
type Value = Cow<'static, str>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_edge_columns_read_back_and_a_record_the_pool_does_not_hold_whole_is_none"),
    word: Word::Succeeds,
    input: || Scenario::Edges,
    expect: Expect::Value(Cow::Borrowed(
        "at PoolAt(3) Some(EdgeColumns { source: MadeColumn(4), destination: MadeColumn(65535) }), as written: true; past the pool: None; one slot short: None; a slot no column has: None",
    )),
}];

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

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(Cow::Owned(match scenario {
        Scenario::Edges => edges(),
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
