//! The memory adapter of the facts port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::FactsMemoryAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::facts::fixtures::contract::check;
use ryo_execution::inbound::use_cases::execution::types::facts::FactsError;
use ryo_execution::inbound::use_cases::execution::types::facts::FactsPort as _;
use ryo_relations::domain::relations::types::Relation;

/// What a row of the cases asks of the adapter.
#[derive(Clone, Copy)]
enum Scenario {
    /// Reads the rows of a worker that wrote none.
    NothingWritten,
    /// Large columns release excess capacity before the worker's next population.
    ReleasedCapacity,
}

/// What a row of the cases asks of the adapter.
type Input = Scenario;
/// How many rows the worker has, or how many columns it gives.
type Value = usize;
type Failure = FactsError;

/// The rows of what is specific to this adapter: a name, one case word, an input from the fixtures or a constructor, and the expected
/// value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_worker_that_wrote_nothing_has_no_rows_to_read"),
        word: Word::Succeeds,
        input: || Scenario::NothingWritten,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("release_bounds_all_column_capacities_then_reads_next_file"),
        word: Word::Succeeds,
        input: || Scenario::ReleasedCapacity,
        expect: Expect::Value(1),
    },
];

/// Calls the adapter on one input, for its own rows.
fn subject(scenario: Input) -> Result<Value, Failure> {
    let adapter = FactsMemoryAdapter::new();
    match scenario {
        Scenario::NothingWritten => {
            let worker = adapter.worker()?;
            let view = worker.rows(Relation::Function)?;
            Ok(view.columns.first().map_or(0, |column| column.0.len()))
        }
        Scenario::ReleasedCapacity => {
            let mut worker = adapter.worker()?;
            {
                let writable = worker.columns_mut()?;
                writable.function[0].0.resize((1 << 21) + 1, 7);
                writable.function[1].0.resize(1024, 3);
            }
            worker.release()?;
            let capacity = ryo_relations::domain::relations::service::all()
                .iter()
                .try_fold(0_usize, |bytes, relation| {
                    let rows = worker.rows(*relation)?;
                    assert!(rows.columns.iter().all(|column| column.0.is_empty()));
                    Ok::<_, FactsError>(
                        bytes
                            + rows
                                .columns
                                .iter()
                                .map(|column| column.0.capacity() * size_of::<u64>())
                                .sum::<usize>(),
                    )
                })?;
            assert!(capacity <= 16 * 1024 * 1024);
            {
                let writable = worker.columns_mut()?;
                for column in writable.function.iter_mut() {
                    column.0.push(37);
                }
            }
            assert!(
                worker
                    .rows(Relation::Function)?
                    .columns
                    .iter()
                    .all(|column| column.0 == [37])
            );
            Ok(1)
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
