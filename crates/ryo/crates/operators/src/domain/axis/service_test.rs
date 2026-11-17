//! The `axis` service's cases, on values alone.

use super::service::lower;
use super::types::AxisShape;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction, JoinColumn, Side};

/// A scenario: whether what the part answers is what the case states.
type Input = fn() -> bool;
type Value = bool;
type Failure = ();

/// An axis walks in its direction over its columns, and its domain is a select over the domain's comparisons.
fn walks_its_columns() -> bool {
    let output = [JoinColumn {
        side: Side::Right,
        column: ColumnIndex(0),
    }];
    let parts = lower(
        AxisShape {
            direction: Direction::Ancestor,
            id: ColumnIndex(0),
            parent: ColumnIndex(1),
            child: ColumnIndex(2),
            domain_id: ColumnIndex(3),
            output: &output,
        },
        &[],
        &[ColumnIndex(3)],
        &[],
    );
    parts.direction == Direction::Ancestor
        && (parts.id, parts.parent, parts.child, parts.domain_id)
            == (
                ColumnIndex(0),
                ColumnIndex(1),
                ColumnIndex(2),
                ColumnIndex(3),
            )
        && parts.output == output
        && parts.domain == crate::domain::select::service::lower(&[], &[ColumnIndex(3)], &[])
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("an_axis_walks_its_columns_in_its_direction"),
    word: Word::Succeeds,
    input: || walks_its_columns,
    expect: Expect::Value(true),
}];

fn subject(input: Input) -> Result<Value, Failure> {
    Ok(input())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
