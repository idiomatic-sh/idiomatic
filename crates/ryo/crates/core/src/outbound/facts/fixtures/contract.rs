//! What every `facts` adapter must pass.

use super::fake::one_function;
use ryo_language::domain::relations::types::FileId;
use ryo_language::domain::relations::types::Relation;

use super::super::errors::FactsError;
use super::super::interface::FactsPort;
use super::super::types::WordsToKeep;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::table_words::service::{pack, unpacked};
use crate::domain::table_words::types::WordSlice;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stored_facts::types::PackedWords;

/// What a row of the contract is given: which scenario to run against the adapter.
type Input = fn(&dyn FactsPort) -> Result<Value, FactsError>;
/// What a scenario answers: a count.
type Value = usize;
/// What a row of the contract expects when it is refused.
type Failure = FactsError;

/// How many rows a worker reads back after one is pushed: one.
fn appended_rows_are_read_back(facts: &dyn FactsPort) -> Result<Value, FactsError> {
    let mut worker = facts.worker()?;
    for (column, number) in worker
        .columns_mut()?
        .function
        .iter_mut()
        .zip(one_function(FileId(0)))
    {
        column.0.push(number);
    }
    Ok(worker
        .rows(Relation::Function)?
        .columns
        .first()
        .map_or(0, |column| column.0.len()))
}

/// The rows left after a release, times ten, plus those after the next file's one row: 1.
fn the_next_file_replaces_the_rows_of_the_one_before(
    facts: &dyn FactsPort,
) -> Result<Value, FactsError> {
    let mut worker = facts.worker()?;
    for (column, number) in worker
        .columns_mut()?
        .function
        .iter_mut()
        .zip(one_function(FileId(0)))
    {
        column.0.push(number);
    }
    worker.release()?;
    let empty = worker
        .rows(Relation::Function)?
        .columns
        .first()
        .map_or(0, |column| column.0.len());
    for (column, number) in worker
        .columns_mut()?
        .function
        .iter_mut()
        .zip(one_function(FileId(1)))
    {
        column.0.push(number);
    }
    Ok(empty * 10
        + worker
            .rows(Relation::Function)?
            .columns
            .first()
            .map_or(0, |column| column.0.len()))
}

/// How many words a bucket holds at the finish, times ten, plus those of a bucket nothing kept: 50.
fn words_wait_in_a_bucket_until_the_finish(facts: &dyn FactsPort) -> Result<Value, FactsError> {
    let (mut first, mut second) = (facts.worker()?, facts.worker()?);
    first.keep(Bucket(3), WordsToKeep(&PackedWords(vec![1, 2])))?;
    for (column, number) in first
        .columns_mut()?
        .function
        .iter_mut()
        .zip(one_function(FileId(0)))
    {
        column.0.push(number);
    }
    first.release()?;
    first.keep(Bucket(3), WordsToKeep(&PackedWords(vec![3])))?;
    second.keep(Bucket(3), WordsToKeep(&PackedWords(vec![4, 5])))?;
    let finished = facts.finish(&mut vec![first, second].into_iter())?;
    Ok(finished.kept(Bucket(3))?.0.0.len() * 10 + finished.kept(Bucket(9))?.0.0.len())
}

/// How many rows a worker holds when it is released before any is pushed: none.
fn releasing_before_appending_keeps_nothing(facts: &dyn FactsPort) -> Result<Value, FactsError> {
    let mut worker = facts.worker()?;
    worker.release()?;
    Ok(worker
        .rows(Relation::Function)?
        .columns
        .first()
        .map_or(0, |column| column.0.len()))
}

/// How many words a worker finished on its own holds of a bucket it kept to.
fn a_worker_finished_on_its_own_holds_what_it_kept(
    facts: &dyn FactsPort,
) -> Result<Value, FactsError> {
    let mut worker = facts.worker()?;
    worker.keep(Bucket(3), WordsToKeep(&PackedWords(vec![1, 2])))?;
    Ok(worker.finished()?.kept(Bucket(3))?.0.0.len())
}

/// Whether words kept packed by two workers come back, unpacked, as the words each kept, one worker after the other: 1.
fn packed_words_come_back_as_the_words_kept(facts: &dyn FactsPort) -> Result<Value, FactsError> {
    let (first_words, second_words) = ([300, u64::MAX], [0, 128]);
    let (mut first, mut second) = (facts.worker()?, facts.worker()?);
    first.keep(Bucket(3), WordsToKeep(&pack(WordSlice(&first_words))))?;
    second.keep(Bucket(3), WordsToKeep(&pack(WordSlice(&second_words))))?;
    let finished = facts.finish(&mut vec![first, second].into_iter())?;
    let kept = finished.kept(Bucket(3))?;
    Ok(usize::from(
        unpacked(&kept.0)
            .map(|each| each.0)
            .eq(first_words.into_iter().chain(second_words)),
    ))
}

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("appended_rows_are_read_back"),
        word: Word::Succeeds,
        input: || appended_rows_are_read_back,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_next_file_replaces_the_rows_of_the_one_before"),
        word: Word::Succeeds,
        input: || the_next_file_replaces_the_rows_of_the_one_before,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("words_wait_in_a_bucket_until_the_finish"),
        word: Word::Succeeds,
        input: || words_wait_in_a_bucket_until_the_finish,
        expect: Expect::Value(50),
    },
    Case {
        name: CaseName("releasing_before_appending_keeps_nothing"),
        word: Word::Succeeds,
        input: || releasing_before_appending_keeps_nothing,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_worker_finished_on_its_own_holds_what_it_kept"),
        word: Word::Succeeds,
        input: || a_worker_finished_on_its_own_holds_what_it_kept,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("packed_words_come_back_as_the_words_kept"),
        word: Word::Succeeds,
        input: || packed_words_come_back_as_the_words_kept,
        expect: Expect::Value(1),
    },
];

/// Runs the contract's rows against one implementation: the fake, or an adapter of another crate.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &dyn FactsPort) {
    run(CASES, Word::Succeeds, |scenario| scenario(adapter));
}
