//! What every `route_counts` adapter must pass.

use super::super::interface::RouteCountsPort;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::route_counts::types::{
    RecordKey, RecordTally, RecordText, RouteField, RouteValue,
};

/// The two route count records the port writes, each named by its route.
#[derive(Clone, Copy, Debug)]
pub struct Records {
    /// The route the first record counts.
    pub first: &'static str,
    /// The route the second record counts.
    pub second: &'static str,
}

/// What a row of the contract is given.
type Input = Records;
/// What a row of the contract expects when it succeeds: whether the two documents are equal.
type Value = bool;
/// What a row of the contract expects when it is refused.
type Failure = ();

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("one_record_writes_one_document"),
        word: Word::Succeeds,
        input: || Records {
            first: "owner_build",
            second: "owner_build",
        },
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("records_that_differ_write_documents_that_differ"),
        word: Word::Succeeds,
        input: || Records {
            first: "owner_build",
            second: "graph_domain_build",
        },
        expect: Expect::Value(false),
    },
];

/// Runs the contract's rows against one implementation: the fake, or an adapter of another crate.
///
/// # Panics
///
/// When the implementation writes one record two ways, or two records one way.
pub fn check(adapter: &impl RouteCountsPort) {
    // One row of a file's route counts, named by `route`.
    let row = |route: &str| {
        RouteValue::Record(vec![
            RouteField {
                key: RecordKey("route"),
                value: RouteValue::Text(RecordText(route.to_owned())),
            },
            RouteField {
                key: RecordKey("calls"),
                value: RouteValue::Tally(RecordTally(2)),
            },
        ])
    };
    // Whether the adapter writes the two records as one document.
    let subject = |records: Input| -> Result<Value, Failure> {
        Ok(adapter.document(&row(records.first)) == adapter.document(&row(records.second)))
    };
    run(CASES, Word::Succeeds, subject);
}
