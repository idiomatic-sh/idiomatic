//! The memory adapter of the facts port: the port's contract rows run through it, then the cases specific to it.

use super::adapter::FactsMemoryAdapter;
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::fixtures::contract::check;
use crate::outbound::facts::fixtures::words::test_words;
use crate::outbound::facts::interface::FactsPort as _;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::stored_facts::types::{Bucket, PackedWords};
use std::borrow::Cow;

/// What one worker of the adapter writes before it is read.
#[derive(Clone, Copy, Debug)]
enum Written {
    /// Nothing.
    Nothing,
    /// The function relation's first two columns grown to these lengths, then released, then
    /// one number pushed to every function column.
    Released {
        first: usize,
        second: usize,
        next: u64,
    },
    /// These words kept in bucket 3, then read twice from the finish.
    Kept(&'static [u8]),
}

/// What the worker or the finish answered.
#[derive(Debug, PartialEq)]
enum Read {
    /// The function relation's first column length.
    Rows(usize),
    /// After the release: whether every relation's columns were empty, whether their capacity
    /// stayed within 16 MiB, and the function columns after the next push.
    Released {
        emptied: bool,
        bounded: bool,
        next: Cow<'static, [Cow<'static, [u64]>]>,
    },
    /// The kept words, and whether both reads borrow the one buffer.
    Kept {
        words: Cow<'static, [u8]>,
        borrowed: bool,
    },
}

type Input = Written;
type Value = Read;
type Failure = FactsError;

/// The rows of what is specific to this adapter: a name, one case word, an input from the fixtures or a constructor, and the expected
/// value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_worker_that_wrote_nothing_has_no_rows_to_read"),
        word: Word::Succeeds,
        input: || Written::Nothing,
        expect: Expect::Value(Read::Rows(0)),
    },
    Case {
        name: CaseName("release_bounds_all_column_capacities_then_reads_next_file"),
        word: Word::Succeeds,
        input: || Written::Released {
            first: (1 << 21) + 1,
            second: 1024,
            next: 37,
        },
        expect: Expect::Value(Read::Released {
            emptied: true,
            bounded: true,
            next: Cow::Borrowed(&[
                Cow::Borrowed(&[37]),
                Cow::Borrowed(&[37]),
                Cow::Borrowed(&[37]),
                Cow::Borrowed(&[37]),
            ]),
        }),
    },
    Case {
        name: CaseName("the_finish_hands_out_a_stored_population_as_a_borrow_not_a_copy"),
        word: Word::Succeeds,
        input: || Written::Kept(&[1, 2, 3]),
        expect: Expect::Value(Read::Kept {
            words: Cow::Borrowed(&[1, 2, 3]),
            borrowed: true,
        }),
    },
];

/// Writes through one worker of a new adapter and reads back.
fn subject(written: Input) -> Result<Value, Failure> {
    let adapter = FactsMemoryAdapter::new();
    match written {
        Written::Nothing => {
            let worker = adapter.worker()?;
            let view = worker.rows(Relation::Function)?;
            Ok(Read::Rows(
                view.columns.first().map_or(0, |column| column.0.len()),
            ))
        }
        Written::Released {
            first,
            second,
            next,
        } => {
            let mut worker = adapter.worker()?;
            {
                let writable = worker.columns_mut()?;
                writable.function[0].0.resize(first, 7);
                writable.function[1].0.resize(second, 3);
            }
            worker.release()?;
            let mut emptied = true;
            let mut capacity = 0_usize;
            for relation in ryo_relations::domain::relations::service::all() {
                let rows = worker.rows(*relation)?;
                emptied &= rows.columns.iter().all(|column| column.0.is_empty());
                capacity += rows
                    .columns
                    .iter()
                    .map(|column| column.0.capacity() * size_of::<u64>())
                    .sum::<usize>();
            }
            {
                let writable = worker.columns_mut()?;
                for column in writable.function.iter_mut() {
                    column.0.push(next);
                }
            }
            let next = worker
                .rows(Relation::Function)?
                .columns
                .iter()
                .map(|column| Cow::Owned(column.0.to_vec()))
                .collect();
            Ok(Read::Released {
                emptied,
                bounded: capacity <= 16 * 1024 * 1024,
                next,
            })
        }
        Written::Kept(words) => {
            let worker = adapter.worker()?;
            worker.keep(Bucket(3), test_words(&PackedWords(words.to_vec())))?;
            let finished = adapter.finish(&mut vec![worker].into_iter())?;
            let (first, second) = (finished.kept(Bucket(3))?, finished.kept(Bucket(3))?);
            Ok(Read::Kept {
                words: first.words.0.0.to_vec().into(),
                borrowed: std::ptr::eq(first.words.0.0.as_ptr(), second.words.0.0.as_ptr()),
            })
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
