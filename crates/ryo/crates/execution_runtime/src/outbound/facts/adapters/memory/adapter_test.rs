//! The memory adapter of the facts port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::FactsMemoryAdapter;
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::fixtures::contract::check;
use crate::outbound::facts::fixtures::words::test_words;
use crate::outbound::facts::interface::FactsPort as _;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::stored_facts::types::{Bucket, PackedWords};

/// What a row of the cases asks of the adapter.
#[derive(Clone, Copy)]
enum Scenario {
    /// Reads the rows of a worker that wrote none.
    NothingWritten,
    /// Large columns release excess capacity before the worker's next population.
    ReleasedCapacity,
    /// One worker's stored population, read twice from the finish: both reads borrow the one buffer the worker filled.
    StoredPopulationBorrowed,
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
    Case {
        name: CaseName("the_finish_hands_out_a_stored_population_as_a_borrow_not_a_copy"),
        word: Word::Succeeds,
        input: || Scenario::StoredPopulationBorrowed,
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
        Scenario::StoredPopulationBorrowed => {
            let worker = adapter.worker()?;
            worker.keep(Bucket(3), test_words(&PackedWords(vec![1, 2, 3])))?;
            let finished = adapter.finish(&mut vec![worker].into_iter())?;
            let (first, second) = (finished.kept(Bucket(3))?, finished.kept(Bucket(3))?);
            Ok(usize::from(
                first.words.0.0 == [1, 2, 3]
                    && std::ptr::eq(first.words.0.0.as_ptr(), second.words.0.0.as_ptr()),
            ))
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
