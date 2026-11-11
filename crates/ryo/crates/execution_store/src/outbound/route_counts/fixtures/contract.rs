//! What every `route_counts` adapter must pass.

use super::super::interface::RouteCountsPort;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::route_counts::types::{RouteField, RouteValue};

/// What the contract asks of the port.
#[derive(Clone, Copy, Debug)]
pub enum Scenario {
    /// One record written twice.
    SameRecordTwice,
    /// Two records that differ in one text.
    RecordsThatDiffer,
}

/// What a row of the contract is given.
pub type Input = Scenario;
/// What a row of the contract expects when it succeeds: whether the two documents are equal.
pub type Value = bool;
/// What a row of the contract expects when it is refused.
pub type Failure = ();

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("one_record_writes_one_document"),
        word: Word::Succeeds,
        input: || Scenario::SameRecordTwice,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("records_that_differ_write_documents_that_differ"),
        word: Word::Succeeds,
        input: || Scenario::RecordsThatDiffer,
        expect: Expect::Value(false),
    },
];

/// One row of a file's route counts, named by `route`.
fn row(route: &str) -> RouteValue {
    RouteValue::Record(vec![
        RouteField {
            key: "route",
            value: RouteValue::Text(route.to_owned()),
        },
        RouteField {
            key: "calls",
            value: RouteValue::Tally(2),
        },
    ])
}

/// Whether the adapter writes the scenario's two records as one document.
pub fn subject(adapter: &impl RouteCountsPort, scenario: Input) -> Result<Value, Failure> {
    let (first, second) = match scenario {
        Scenario::SameRecordTwice => (row("owner_build"), row("owner_build")),
        Scenario::RecordsThatDiffer => (row("owner_build"), row("graph_domain_build")),
    };
    Ok(adapter.document(&first) == adapter.document(&second))
}

/// Runs the contract's rows against one implementation: the fake, or an adapter of another crate.
///
/// # Panics
///
/// When the implementation writes one record two ways, or two records one way.
pub fn check(adapter: &impl RouteCountsPort) {
    run(CASES, Word::Succeeds, |scenario| subject(adapter, scenario));
}
