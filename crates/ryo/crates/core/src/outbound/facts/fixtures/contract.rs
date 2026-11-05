//! What every `facts` adapter must pass.

use super::fake::one_function;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Relation;

use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution::inbound::use_cases::execution::types::facts::FactsError;
use ryo_execution::inbound::use_cases::execution::types::facts::FactsPort;
use ryo_execution::inbound::use_cases::execution::types::facts::WordsToKeep;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stored_facts::types::PackedWords;
use ryo_values::domain::table_words::service::{pack, unpacked};
use ryo_values::domain::table_words::types::WordSlice;

static FRONTIER_PROOF: std::sync::LazyLock<ryo_values::domain::records::types::FrontierProof> =
    std::sync::LazyLock::new(|| ryo_values::domain::records::types::FrontierProof {
        version: 1,
        rule: 0,
        input: 0,
        rule_digest: ryo_values::domain::stored_facts::types::Digest([0; 16]),
        coverage: ryo_values::domain::records::types::FrontierCoverage::Complete,
    });
pub(crate) fn test_words(words: &PackedWords) -> WordsToKeep<'_> {
    WordsToKeep {
        words,
        file: FileId(0),
        proof: &FRONTIER_PROOF,
    }
}

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
    first.keep(Bucket(3), test_words(&PackedWords(vec![1, 2])))?;
    for (column, number) in first
        .columns_mut()?
        .function
        .iter_mut()
        .zip(one_function(FileId(0)))
    {
        column.0.push(number);
    }
    first.release()?;
    first.keep(Bucket(3), test_words(&PackedWords(vec![3])))?;
    second.keep(Bucket(3), test_words(&PackedWords(vec![4, 5])))?;
    let finished = facts.finish(&mut vec![first, second].into_iter())?;
    Ok(finished.kept(Bucket(3))?.words.0.0.len() * 10 + finished.kept(Bucket(9))?.words.0.0.len())
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
    worker.keep(Bucket(3), test_words(&PackedWords(vec![1, 2])))?;
    Ok(worker.finished()?.kept(Bucket(3))?.words.0.0.len())
}

/// Whether words kept packed by two workers come back, unpacked, as the words each kept, one worker after the other: 1.
fn packed_words_come_back_as_the_words_kept(facts: &dyn FactsPort) -> Result<Value, FactsError> {
    let (first_words, second_words) = ([300, u64::MAX], [0, 128]);
    let (mut first, mut second) = (facts.worker()?, facts.worker()?);
    first.keep(Bucket(3), test_words(&pack(WordSlice(&first_words))))?;
    second.keep(Bucket(3), test_words(&pack(WordSlice(&second_words))))?;
    let finished = facts.finish(&mut vec![first, second].into_iter())?;
    let kept = finished.kept(Bucket(3))?;
    Ok(usize::from(
        unpacked(&kept.words.0)
            .map(|each| each.0)
            .eq(first_words.into_iter().chain(second_words)),
    ))
}

/// Byte ranges retain exact per-file coverage through worker joining.
fn frontier_proofs_follow_their_bytes(facts: &dyn FactsPort) -> Result<Value, FactsError> {
    use ryo_values::domain::records::types::{FrontierAffected, FrontierCoverage};
    let mut first = facts.worker()?;
    let mut second = facts.worker()?;
    let mut partial = FRONTIER_PROOF.clone();
    partial.coverage = FrontierCoverage::Partial(vec![FrontierAffected::CurrentFile]);
    first.keep(
        Bucket(3),
        WordsToKeep {
            words: &PackedWords(vec![1, 2]),
            file: FileId(1),
            proof: &partial,
        },
    )?;
    second.keep(
        Bucket(3),
        WordsToKeep {
            words: &PackedWords(vec![3]),
            file: FileId(0),
            proof: &FRONTIER_PROOF,
        },
    )?;
    let bucket = facts
        .finish(&mut vec![first, second].into_iter())?
        .kept(Bucket(3))?;
    Ok(usize::from(
        bucket.words.0.0 == [1, 2, 3]
            && bucket.sources.len() == 2
            && bucket.sources[0].file == FileId(1)
            && bucket.sources[0].start == 0
            && bucket.sources[0].length == 2
            && bucket.sources[0].proof.coverage == partial.coverage
            && bucket.sources[1].file == FileId(0)
            && bucket.sources[1].start == 2
            && bucket.sources[1].length == 1
            && bucket.sources[1].proof.coverage == FrontierCoverage::Complete,
    ))
}

/// A stable resident snapshot survives reads of other relations and spill-sized word retention.
fn resident_rows_survive_other_reads_and_word_flushes(
    facts: &dyn FactsPort,
) -> Result<Value, FactsError> {
    use ryo_execution::inbound::use_cases::execution::types::facts::ResidentStability;
    let mut worker = facts.worker()?;
    if worker.resident_stability() == ResidentStability::Unstable {
        return Ok(1);
    }
    let first = [7, 3, 2, (7_u64 << 32_u32) | 0x0b];
    let second = [1, 1, 2, (1_u64 << 32_u32) | 0x03];
    {
        let columns = worker.columns_mut()?;
        for row in [first, second, first] {
            for (column, value) in columns.function.iter_mut().zip(row) {
                column.0.push(value);
            }
        }
    }
    let snapshot = |resident: &dyn ryo_execution::inbound::use_cases::execution::types::facts::FactsWorker| -> Result<_, FactsError> {
        let rows = resident.rows(Relation::Function)?;
        Ok((
            rows.kinds.to_vec(),
            rows.columns
                .iter()
                .map(|column| column.0.clone())
                .collect::<Vec<_>>(),
        ))
    };
    let expected = snapshot(worker.as_ref())?;
    for amount in [2, (1_usize << 20_u32) + 17, 3] {
        let _scope_rows = worker.rows(Relation::Scope)?;
        let pinned = worker.rows(Relation::Function)?;
        worker.keep(Bucket(7), test_words(&PackedWords(vec![1; amount])))?;
        if pinned.kinds != expected.0.as_slice()
            || pinned
                .columns
                .iter()
                .map(|column| column.0.as_slice())
                .ne(expected.1.iter().map(Vec::as_slice))
        {
            return Ok(0);
        }
        if snapshot(worker.as_ref())? != expected {
            return Ok(0);
        }
        let _identifier_rows = worker.rows(Relation::Identifier)?;
        if snapshot(worker.as_ref())? != expected {
            return Ok(0);
        }
    }
    worker.release()?;
    Ok(usize::from(
        worker
            .rows(Relation::Function)?
            .columns
            .iter()
            .all(|column| column.0.is_empty()),
    ))
}

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("frontier_proofs_follow_their_bytes_across_workers"),
        word: Word::Succeeds,
        input: || frontier_proofs_follow_their_bytes,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("stable_resident_rows_survive_other_reads_and_spill_sized_keeps"),
        word: Word::Succeeds,
        input: || resident_rows_survive_other_reads_and_word_flushes,
        expect: Expect::Value(1),
    },
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
