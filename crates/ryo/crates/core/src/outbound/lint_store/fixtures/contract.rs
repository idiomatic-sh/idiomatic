//! What every `lint_store` adapter must pass.

use super::super::errors::LintStoreError;
use super::super::interface::{LintStorePort, ReadStore};
use super::super::types::StoreKeeping;
use super::fake::{
    again, failed, over_a_b_d_and_e, over_b_and_d, register_proofs, stored, words_of,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stored_facts::types::PackedWords;
use ryo_values::domain::stored_findings::types::StoredFileNumber;
use ryo_values::domain::stores::types::{
    BucketsWanted, FileAgain, KeptRows, Stored, StoredHanded, StoredWords,
};

/// Explicitly resolve the read representation only in contract result oracles.
/// Production handover resolves a filtered row at a time instead of materializing this table.
pub fn materialized_handed(
    read: &dyn ReadStore,
    wanted: &BucketsWanted,
) -> Result<StoredHanded, LintStoreError> {
    let handed = read.handed(wanted)?.0;
    let mut rows = Vec::with_capacity(handed.frontier.len());
    for row in handed.frontier {
        let words = match row.payload {
            ryo_values::domain::stores::types::ReadWordPayload::Inline(words) => words,
            ryo_values::domain::stores::types::ReadWordPayload::Old(span) => {
                let mut loaded = PackedWords::default();
                read.read_old(&span, &mut loaded)?;
                loaded
            }
        };
        rows.push(StoredWords {
            file: row.file,
            bucket: row.bucket,
            proof: row.proof,
            words,
        });
    }
    Ok(StoredHanded(
        ryo_values::domain::stores::types::HandedRows {
            observations: handed.observations,
            proofs: handed.proofs,
            frontier: rows,
            texts: handed.texts,
            notes: handed.notes,
            covered: handed.covered,
        },
    ))
}

/// What a row of the contract asks of a store.
#[derive(Clone, Copy, Debug)]
pub enum Question {
    /// What a store nothing was kept in reads.
    NothingKept,
    /// Whether what was kept is what is read.
    KeptIsRead,
    /// Whether the last keep stands.
    TheLastKeepStands,
    /// Whether a file no language could parse reads back with what the language said.
    AFailureIsKept,
    /// Whether the words of a bucket kept as they were stay, and only the buckets asked for are read.
    UnchangedBucketsStay,
    /// Whether the words a file used again handed a bucket made again stay under its number now, and those of a file not used
    /// again go.
    FilesUsedAgainMove,
    /// Whether a run that read the store keeps against that store, not against the one another run kept after it was read, and
    /// whether what it is handed is of the one it read: every row stays under its own path.
    ARunKeepsAgainstTheStoreItRead,
    /// Whether a keep that must copy words from a store that holds none is refused.
    AKeepOfWordsNobodyReadIsRefused,
}

/// What a row of the contract asks.
type Input = Question;
/// Whether the store did what the row asks.
type Value = bool;
/// What a row of the contract expects when it is refused.
type Failure = LintStoreError;

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_store_nothing_was_kept_in_is_empty"),
        word: Word::Succeeds,
        input: || Question::NothingKept,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("what_was_kept_is_what_is_read"),
        word: Word::Succeeds,
        input: || Question::KeptIsRead,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_last_keep_stands"),
        word: Word::Succeeds,
        input: || Question::TheLastKeepStands,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_file_no_language_could_parse_reads_back_with_what_was_said"),
        word: Word::Succeeds,
        input: || Question::AFailureIsKept,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName(
            "the_words_of_a_bucket_kept_as_they_were_stay_and_only_the_buckets_asked_are_read",
        ),
        word: Word::Succeeds,
        input: || Question::UnchangedBucketsStay,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_words_of_a_file_used_again_move_to_its_number_now"),
        word: Word::Succeeds,
        input: || Question::FilesUsedAgainMove,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_keep_that_must_copy_words_from_a_store_that_holds_none_is_refused"),
        word: Word::Succeeds,
        input: || Question::AKeepOfWordsNobodyReadIsRefused,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_run_keeps_against_the_store_it_read_whatever_another_run_kept_after"),
        word: Word::Succeeds,
        input: || Question::ARunKeepsAgainstTheStoreItRead,
        expect: Expect::Value(true),
    },
];

/// Puts `question` to a store `fresh` makes.
fn answer(
    fresh: &dyn Fn() -> Box<dyn LintStorePort>,
    question: Question,
) -> Result<Value, Failure> {
    let store = fresh();
    // What a run that reads the store keeps in it.
    let kept = |stored: Stored, rows: &KeptRows| store.stored()?.keep(stored, rows);
    // A store that keeps nothing reads back as empty whatever it is given.
    if store.keeps() == StoreKeeping::Discards {
        kept(stored(1), &KeptRows::default())?;
        let read = store.stored()?;
        return Ok(read.tables() == Stored::default()
            && materialized_handed(&*read, &BucketsWanted::All)? == StoredHanded::default());
    }
    // The whole store: the small tables, and what the files handed, every bucket of it.
    let whole = |port: &dyn LintStorePort| -> Result<Stored, LintStoreError> {
        let read = port.stored()?;
        let mut tables = read.tables();
        tables.handed = materialized_handed(&*read, &BucketsWanted::All)?;
        Ok(tables)
    };
    Ok(match question {
        Question::NothingKept => whole(&*store)? == Stored::default(),
        Question::KeptIsRead => {
            kept(stored(1), &KeptRows::default())?;
            whole(&*store)? == stored(1)
                && store.stored()?.tables().handed == StoredHanded::default()
        }
        Question::TheLastKeepStands => {
            kept(stored(1), &KeptRows::default())?;
            kept(stored(2), &KeptRows::default())?;
            whole(&*store)? == stored(2)
        }
        Question::AFailureIsKept => {
            kept(failed(3), &KeptRows::default())?;
            whole(&*store)? == failed(3)
        }
        Question::UnchangedBucketsStay => {
            kept(stored(1), &KeptRows::default())?;
            let mut without = stored(2);
            without.handed.frontier.clear();
            kept(
                without,
                &KeptRows {
                    unchanged: vec![Bucket(2)],
                    ..KeptRows::default()
                },
            )?;
            let read = store.stored()?;
            let handed = materialized_handed(&*read, &BucketsWanted::These(vec![Bucket(2)]))?;
            handed.frontier == stored(1).handed.frontier
                && read
                    .handed(&BucketsWanted::These(Vec::new()))?
                    .frontier
                    .is_empty()
        }
        Question::FilesUsedAgainMove => {
            // The file read now is the first; the one used again was the first and is the second now.
            let read_now = StoredWords {
                file: StoredFileNumber(0),
                bucket: Bucket(3),
                proof: super::proof(),
                words: PackedWords(vec![5]),
            };
            let was = stored(1).handed.0.frontier;
            kept(stored(1), &KeptRows::default())?;
            let mut again = stored(2);
            again.handed.frontier = vec![read_now.clone()];
            register_proofs(&mut again.handed);
            kept(
                again.clone(),
                &KeptRows {
                    staged: Vec::new(),
                    unchanged: Vec::new(),
                    file_again: vec![FileAgain {
                        was: StoredFileNumber(0),
                        now: StoredFileNumber(1),
                    }],
                },
            )?;
            let moved = materialized_handed(&*store.stored()?, &BucketsWanted::All)?
                .0
                .frontier;
            kept(again, &KeptRows::default())?;
            let gone = materialized_handed(&*store.stored()?, &BucketsWanted::All)?
                .0
                .frontier;
            moved
                == [read_now.clone()]
                    .into_iter()
                    .chain(was.into_iter().map(|each| StoredWords {
                        file: StoredFileNumber(1),
                        ..each
                    }))
                    .collect::<Vec<_>>()
                && gone == [read_now]
        }
        Question::ARunKeepsAgainstTheStoreItRead => keeps_against_the_store_it_read(&*store)?,
        Question::AKeepOfWordsNobodyReadIsRefused => {
            // A run that read a store with nothing in it cannot keep the words of one: it would copy from whatever is there.
            let (over, rows) = over_a_b_d_and_e();
            matches!(kept(over, &rows), Err(LintStoreError::NothingRead))
        }
    })
}

/// Two runs read one store; the second keeps first, and the first still keeps against what it read.
fn keeps_against_the_store_it_read(store: &dyn LintStorePort) -> Result<Value, Failure> {
    store.stored()?.keep(over_b_and_d(), &KeptRows::default())?;
    // Two runs read the store as it is; the second keeps first, over the files `a, b, d, e`.
    let first = store.stored()?;
    let second = store.stored()?;
    let (over, rows) = over_a_b_d_and_e();
    second.keep(over, &rows)?;
    // What the first run is handed is of the store it read, not of the one just kept.
    let handed = materialized_handed(&*first, &BucketsWanted::All)?;
    // The first run lists `b, d, e` and used `b` and `d` again, under the same numbers.
    let mut mine = stored(3);
    mine.files.truncate(2);
    if let Some(copy) = mine.files.first().cloned() {
        mine.files.push(copy);
    }
    mine.handed.frontier = vec![words_of(2, 13)];
    register_proofs(&mut mine.handed);
    first.keep(
        mine,
        &KeptRows {
            staged: Vec::new(),
            unchanged: Vec::new(),
            file_again: vec![again(0, 0), again(1, 1)],
        },
    )?;
    let all = materialized_handed(&*store.stored()?, &BucketsWanted::All)?;
    Ok(handed.frontier == over_b_and_d().handed.frontier
        && all.frontier == [words_of(0, 10), words_of(1, 11), words_of(2, 13)])
}

/// Runs the contract's rows against one implementation, each over a store `fresh` makes, which holds nothing.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(fresh: &dyn Fn() -> Box<dyn LintStorePort>) {
    run(CASES, Word::Succeeds, |question| answer(fresh, question));
}
