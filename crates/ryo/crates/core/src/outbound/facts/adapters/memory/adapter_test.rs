//! The memory adapter of the facts port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::FactsMemoryAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::fixtures::contract::check;
use crate::outbound::facts::interface::FactsPort as _;
use ryo_language::domain::relations::types::Relation;

/// What a row of the cases asks of the adapter.
#[derive(Clone, Copy)]
enum Scenario {
    /// Reads the rows of a worker that wrote none.
    NothingWritten,
}

/// What a row of the cases asks of the adapter.
type Input = Scenario;
/// How many rows the worker has, or how many columns it gives.
type Value = usize;
type Failure = FactsError;

/// The rows of what is specific to this adapter: a name, one case word, an input from the fixtures or a constructor, and the expected
/// value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_worker_that_wrote_nothing_has_no_rows_to_read"),
    word: Word::Succeeds,
    input: || Scenario::NothingWritten,
    expect: Expect::Value(0),
}];

/// Calls the adapter on one input, for its own rows.
fn subject(scenario: Input) -> Result<Value, Failure> {
    let adapter = FactsMemoryAdapter::new();
    match scenario {
        Scenario::NothingWritten => {
            let worker = adapter.worker()?;
            let view = worker.rows(Relation::Function)?;
            Ok(view.columns.first().map_or(0, |column| column.0.len()))
        }
    }
}

#[test]
fn succeeds() {
    check(&adapter_over_all_relations());
    run(CASES, Word::Succeeds, subject);
}

/// The adapter the contract is run through.
fn adapter_over_all_relations() -> FactsMemoryAdapter {
    FactsMemoryAdapter::new()
}
