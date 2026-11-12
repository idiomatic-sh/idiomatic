//! The `derived` service's cases: the access it lends prepares a contract as the access it holds would.

use super::service::Derived;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// What a row of the cases asks of the service: the key columns of an equality index.
type Input = &'static [u32];
/// How many schemas the prepared contract keeps, and whether its metadata is valid.
type Value = (usize, bool);
/// What a row of the cases expects when it is refused: lending is never refused.
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_lent_access_prepares_an_equality_index_over_one_schema"),
    word: Word::Succeeds,
    input: || &[0],
    expect: Expect::Value((1, true)),
}];

/// Prepares an equality index over the input's columns with the access a held `Derived` lends.
fn subject(columns: Input) -> Result<Value, Failure> {
    let derived = Derived::from(ryo_execution::derived());
    let prepared = derived.access().prepare(
        DerivedRequest::Equality {
            columns: columns.iter().copied().map(ColumnIndex).collect(),
        },
        vec![vec![CellKind::Number]],
    );
    Ok((
        prepared.schemas().len(),
        matches!(prepared.validate(), Ok(())),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
