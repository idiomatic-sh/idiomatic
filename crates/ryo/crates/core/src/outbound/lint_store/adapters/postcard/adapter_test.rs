//! The postcard adapter of the store port: the core's contract rows run through it, then the cases specific to it.

use std::sync::atomic::{AtomicUsize, Ordering};

use super::adapter::LintStorePostcardAdapter;
use super::schemas::{contents, contents_bytes};
use super::types::{SectionLength, TableOfContents};
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::outbound::lint_store::errors::LintStoreError;
use crate::outbound::lint_store::fixtures::contract::check;
use crate::outbound::lint_store::fixtures::fake::{
    again, over_a_b_d_and_e, over_b_and_d, stored, words_of,
};
use crate::outbound::lint_store::interface::LintStorePort as _;
use crate::outbound::lint_store::types::{BuildIdentity, CacheDirectory, Store};
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stores::types::{BucketsWanted, KeptRows, Stored};

/// A question specific to this adapter.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// What another build kept reads as nothing kept.
    AnotherBuildsStoreIsEmpty,
    /// Bytes that are not a store read as nothing kept.
    ADamagedStoreIsEmpty,
    /// A table of contents that reads back but names a section longer than the file reads as nothing kept, whichever section it
    /// is, the files the project's lookup read among the tables, and the store kept after it reads back whole, those files too.
    ADamagedSectionLengthIsEmpty,
    /// The files the project's lookup read beyond the manifests read back whole, by a store of the same build made anew.
    TheConsultedFilesReadBack,
    /// A temporary file left by a run that stopped is not read, and the store in place still is.
    ALeftoverIsNotRead,
    /// The store kept is one file of the cache directory.
    TheStoreIsOneFile,
    /// The cache directory is a file: the store cannot be written, and nothing is left behind.
    AnUnwritableDirectory,
    /// What `handed` answers after `stored` read the store back is what it answers alone, once.
    HandedAfterStoredIsHandedAlone,
    /// The place of the store is a directory: the store cannot be read.
    /// A keep that must copy words, over a store with none read, is refused.
    AKeepOfWordsNobodyReadIsRefused,
    AnUnreadablePlace,
    /// A bucket the run does not ask for is never decoded: the tables and the buckets asked for read back with its bytes damaged.
    OnlyTheBucketsAskedAreDecoded,
    /// A run of another process keeps between this run's read and keep: every word row stays under its own path.
    AnotherProcessKeptBetweenTheReadAndTheKeep,
}

type Input = Question;
type Value = bool;
type Failure = LintStoreError;

/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("another_builds_store_reads_as_nothing_kept"),
        word: Word::Succeeds,
        input: || Question::AnotherBuildsStoreIsEmpty,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_damaged_store_reads_as_nothing_kept"),
        word: Word::Succeeds,
        input: || Question::ADamagedStoreIsEmpty,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName(
            "a_section_longer_than_the_store_reads_as_nothing_kept_and_the_store_starts_over",
        ),
        word: Word::Succeeds,
        input: || Question::ADamagedSectionLengthIsEmpty,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_files_the_project_lookup_read_read_back_whole"),
        word: Word::Succeeds,
        input: || Question::TheConsultedFilesReadBack,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_leftover_temporary_file_is_not_read"),
        word: Word::Succeeds,
        input: || Question::ALeftoverIsNotRead,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_store_is_one_file_of_the_cache_directory"),
        word: Word::Succeeds,
        input: || Question::TheStoreIsOneFile,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_cache_directory_that_is_a_file_cannot_be_written"),
        word: Word::Refuses,
        input: || Question::AnUnwritableDirectory,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, LintStoreError::Unwritable { .. }))
        }),
    },
    Case {
        name: CaseName("handed_after_stored_is_what_handed_alone_answers"),
        word: Word::Succeeds,
        input: || Question::HandedAfterStoredIsHandedAlone,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_store_whose_place_is_a_directory_cannot_be_read"),
        word: Word::Refuses,
        input: || Question::AnUnreadablePlace,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, LintStoreError::Unreadable { .. }))
        }),
    },
    Case {
        name: CaseName("a_keep_that_must_copy_words_from_a_store_nobody_read_is_refused"),
        word: Word::Refuses,
        input: || Question::AKeepOfWordsNobodyReadIsRefused,
        expect: Expect::Refusal(|error| selects(matches!(error, LintStoreError::NothingRead))),
    },
    Case {
        name: CaseName("a_bucket_the_run_does_not_ask_for_is_never_decoded"),
        word: Word::Succeeds,
        input: || Question::OnlyTheBucketsAskedAreDecoded,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_keep_after_another_process_kept_copies_from_the_store_its_run_read"),
        word: Word::Succeeds,
        input: || Question::AnotherProcessKeptBetweenTheReadAndTheKeep,
        expect: Expect::Value(true),
    },
];

/// A store of `directory` for the build `build`.
fn store_in(directory: &std::path::Path, build: &str) -> LintStorePostcardAdapter {
    LintStorePostcardAdapter::new(Store {
        directory: CacheDirectory(directory.to_string_lossy().into()),
        build: BuildIdentity(build.into()),
    })
}

/// The names of the files of `directory`.
fn names(directory: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(directory)
        .expect("the cache directory")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Rewrites the store file of `directory`, kept by `build`, with the table of contents `damage` makes of the one it holds.
fn damage_contents(
    directory: &std::path::Path,
    build: &str,
    damage: &dyn Fn(&mut TableOfContents),
) {
    let file = directory.join("lint.store");
    let bytes = std::fs::read(&file).expect("the store");
    let (body, trailer) = bytes.split_at(bytes.len() - 8);
    let mut word = [0_u8; 8];
    word.copy_from_slice(trailer);
    let length = usize::try_from(u64::from_le_bytes(word)).expect("a table of contents length");
    let (sections, toc) = body.split_at(body.len() - length);
    let mut toc = contents(build, toc).expect("a table of contents that reads back");
    damage(&mut toc);
    let toc = contents_bytes(build, &toc).expect("a table of contents");
    let mut damaged = sections.to_vec();
    damaged.extend_from_slice(&toc);
    damaged.extend_from_slice(&u64::try_from(toc.len()).expect("a length").to_le_bytes());
    std::fs::write(&file, damaged).expect("a damaged store");
}

/// Overwrites the bytes of the section of `bucket` in the store file of `directory`, kept by `build`, with bytes that read back as
/// nothing.
fn damage_bucket(directory: &std::path::Path, build: &str, bucket: Bucket) {
    let file = directory.join("lint.store");
    let mut bytes = std::fs::read(&file).expect("the store");
    let (body, trailer) = bytes.split_at(bytes.len() - 8);
    let mut word = [0_u8; 8];
    word.copy_from_slice(trailer);
    let length = usize::try_from(u64::from_le_bytes(word)).expect("a table of contents length");
    let toc = contents(
        build,
        body.get(body.len() - length..)
            .expect("a table of contents"),
    )
    .expect("a table of contents that reads back");
    let span = toc
        .buckets
        .iter()
        .find(|each| each.bucket == bucket)
        .expect("the bucket's section")
        .span;
    let start = usize::try_from(span.start.0).expect("a start");
    let end = start + usize::try_from(span.length.0).expect("a length");
    bytes.get_mut(start..end).expect("the section").fill(0xff);
    std::fs::write(&file, bytes).expect("a damaged store");
}

/// `stored` without what its files handed: what the small tables hold.
fn tables(mut stored: Stored) -> Stored {
    stored.handed = ryo_values::domain::stores::types::StoredHanded::default();
    stored
}

/// Two processes over one cache directory read one store; the second keeps first, and the first still keeps against what it read.
fn another_process_kept_between_the_read_and_the_keep(
    directory: &std::path::Path,
    store: &LintStorePostcardAdapter,
) -> Result<Value, Failure> {
    store.stored()?.keep(over_b_and_d(), &KeptRows::default())?;
    // Two processes over one cache directory: each its own adapter.
    let first = store_in(directory, "build-1");
    let second = store_in(directory, "build-1");
    let read_by_first = first.stored()?;
    let read_by_second = second.stored()?;
    let (kept, rows) = over_a_b_d_and_e();
    read_by_second.keep(kept, &rows)?;
    let mut mine = stored(3);
    mine.files.truncate(2);
    if let Some(copy) = mine.files.first().cloned() {
        mine.files.push(copy);
    }
    mine.handed.frontier = vec![words_of(2, 13)];
    read_by_first.keep(
        mine,
        &KeptRows {
            unchanged: Vec::new(),
            file_again: vec![again(0, 0), again(1, 1)],
        },
    )?;
    let last = store_in(directory, "build-1");
    let all = last.stored()?.handed(&BucketsWanted::All)?;
    Ok(all.frontier == [words_of(0, 10), words_of(1, 11), words_of(2, 13)])
}

/// A bucket damaged beyond the ones a reader asks for is never decoded.
fn only_the_buckets_asked_are_decoded(
    directory: &std::path::Path,
    store: &LintStorePostcardAdapter,
) -> Result<Value, Failure> {
    let mut two = stored(1);
    let mut other = two.handed.frontier.clone();
    for each in &mut other {
        each.bucket = Bucket(5);
    }
    two.handed.frontier.extend(other);
    store.stored()?.keep(two.clone(), &KeptRows::default())?;
    damage_bucket(directory, "build-1", Bucket(5));
    let reader = store_in(directory, "build-1");
    let read = reader.stored()?;
    let tables_read = read.tables() == tables(two);
    let asked = read.handed(&BucketsWanted::These(vec![Bucket(2)]))?;
    Ok(tables_read && asked.frontier == stored(1).handed.frontier)
}

fn subject(question: Input) -> Result<Value, Failure> {
    let root = tempfile::tempdir().expect("a directory");
    let directory = root.path().join("cache");
    let store = store_in(&directory, "build-1");
    Ok(match question {
        Question::AnotherBuildsStoreIsEmpty => {
            store.stored()?.keep(stored(1), &KeptRows::default())?;
            store_in(&directory, "build-2").stored()?.tables() == Stored::default()
                && store.stored()?.tables() == tables(stored(1))
        }
        Question::ADamagedStoreIsEmpty => {
            store.stored()?.keep(stored(1), &KeptRows::default())?;
            let file = directory.join("lint.store");
            let mut bytes = std::fs::read(&file).expect("the store");
            bytes.truncate(bytes.len() / 2);
            std::fs::write(&file, bytes).expect("a damaged store");
            store.stored()?.tables() == Stored::default()
        }
        Question::ADamagedSectionLengthIsEmpty => {
            let past = SectionLength(u64::MAX);
            let damages: [&dyn Fn(&mut TableOfContents); 3] = [
                &|toc: &mut TableOfContents| toc.tables.length = past,
                &|toc: &mut TableOfContents| toc.rest.length = past,
                &|toc: &mut TableOfContents| {
                    for each in &mut toc.buckets {
                        each.span.length = past;
                    }
                },
            ];
            let mut empty = !stored(1).consulted.is_empty();
            for damage in damages {
                store.stored()?.keep(stored(1), &KeptRows::default())?;
                damage_contents(&directory, "build-1", damage);
                let read = store_in(&directory, "build-1").stored()?.tables();
                empty &= read == Stored::default() && read.consulted.is_empty();
            }
            store.stored()?.keep(stored(2), &KeptRows::default())?;
            let again = store.stored()?.tables();
            empty && again.consulted == stored(2).consulted && again == tables(stored(2))
        }
        Question::TheConsultedFilesReadBack => {
            store.stored()?.keep(stored(1), &KeptRows::default())?;
            let consulted = store_in(&directory, "build-1").stored()?.tables().consulted;
            let absent = store_in(&directory, "build-1")
                .stored()?
                .tables()
                .consulted_absent;
            !consulted.is_empty()
                && consulted == stored(1).consulted
                && !absent.is_empty()
                && absent == stored(1).consulted_absent
        }
        Question::ALeftoverIsNotRead => {
            store.stored()?.keep(stored(1), &KeptRows::default())?;
            std::fs::write(directory.join("lint.store.tmp.1.1"), b"half a store")
                .expect("a leftover");
            store.stored()?.tables() == tables(stored(1))
        }
        Question::TheStoreIsOneFile => {
            store.stored()?.keep(stored(1), &KeptRows::default())?;
            store.stored()?.keep(stored(2), &KeptRows::default())?;
            names(&directory) == ["lint.store"]
        }
        Question::AnUnwritableDirectory => {
            let read = store.stored()?;
            std::fs::write(&directory, b"a file").expect("a file");
            read.keep(stored(1), &KeptRows::default()).map(|()| false)?
        }
        Question::HandedAfterStoredIsHandedAlone => {
            store.stored()?.keep(stored(1), &KeptRows::default())?;
            let wanted = BucketsWanted::All;
            let reader = store_in(&directory, "build-1");
            let alone = reader.stored()?.handed(&wanted)?;
            let read = store.stored()?;
            let after = read.handed(&wanted)?;
            let again = read.handed(&wanted)?;
            after == alone && again == alone
        }
        Question::OnlyTheBucketsAskedAreDecoded => {
            only_the_buckets_asked_are_decoded(&directory, &store)?
        }
        Question::AnotherProcessKeptBetweenTheReadAndTheKeep => {
            another_process_kept_between_the_read_and_the_keep(&directory, &store)?
        }
        Question::AKeepOfWordsNobodyReadIsRefused => {
            let (kept, rows) = over_a_b_d_and_e();
            store.stored()?.keep(kept, &rows).map(|()| false)?
        }
        Question::AnUnreadablePlace => {
            std::fs::create_dir_all(directory.join("lint.store")).expect("a directory");
            store.stored().map(|_| false)?
        }
    })
}

#[test]
fn succeeds() {
    let root = tempfile::tempdir().expect("a directory");
    let next = AtomicUsize::new(0);
    check(&|| {
        Box::new(store_in(
            &root
                .path()
                .join(next.fetch_add(1, Ordering::Relaxed).to_string()),
            "build",
        ))
    });
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
