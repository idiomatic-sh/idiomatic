//! The postcard adapter of the store port: the core's contract rows run through it, then the cases specific to it.

use std::sync::atomic::{AtomicUsize, Ordering};

use super::adapter::LintStorePostcardAdapter;
use super::schemas::{contents, contents_bytes};
use super::types::{SectionLength, TableOfContents};
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::outbound::lint_store::errors::LintStoreError;
use crate::outbound::lint_store::fixtures::contract::{check, materialized_handed};
use crate::outbound::lint_store::fixtures::fake::{
    again, over_a_b_d_and_e, over_b_and_d, register_proofs, stored, words_of,
};
use crate::outbound::lint_store::interface::LintStorePort as _;
use crate::outbound::lint_store::types::{BuildIdentity, CacheDirectory, Store};
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stores::types::{BucketsWanted, KeptRows, Stored};

/// A question specific to this adapter.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// Old relation-layout stores are misses even when the caller build string is unchanged.
    PriorSchemaStoreIsEmpty,
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
    /// Repeated unchanged and edited stores replace their one published file within the first-cycle byte bound.
    RepeatedKeepsStayBounded,
    /// An append-only writer fails the same measured file and byte bound.
    AppendingHistoryFailsBound,
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
        name: CaseName("a_prior_project_schema_store_is_a_safe_cache_miss"),
        word: Word::Succeeds,
        input: || Question::PriorSchemaStoreIsEmpty,
        expect: Expect::Value(true),
    },
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
        name: CaseName("repeated_unchanged_and_changed_keeps_bound_real_files_and_bytes"),
        word: Word::Succeeds,
        input: || Question::RepeatedKeepsStayBounded,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("an_append_only_writer_fails_the_same_retention_bound"),
        word: Word::Succeeds,
        input: || Question::AppendingHistoryFailsBound,
        expect: Expect::Value(false),
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

/// Run the real store twenty times, retaining the first unchanged/edited cycle as the byte bound.
fn repeated_keeps_within_first_cycle(
    directory: &std::path::Path,
    store: &LintStorePostcardAdapter,
    append_history: bool,
) -> Result<bool, Failure> {
    let mut first_cycle_bytes = 0_u64;
    let mut first_cycle_files = 0_usize;
    for round in 0..20_u8 {
        let expected = stored(1 + (round / 2) % 2);
        store
            .stored()?
            .keep(expected.clone(), &KeptRows::default())?;
        if append_history {
            let _copied = std::fs::copy(
                directory.join("lint.store"),
                directory.join(format!("history-{round}")),
            )
            .expect("an append-only writer retains each old snapshot");
        }
        if store.stored()?.tables() != tables(expected) {
            return Ok(false);
        }
        let files = names(directory);
        let bytes: u64 = std::fs::read_dir(directory)
            .expect("retained directory")
            .map(|entry| {
                entry
                    .expect("retained entry")
                    .metadata()
                    .expect("retained file")
                    .len()
            })
            .sum();
        if round < 4 {
            first_cycle_bytes = first_cycle_bytes.max(bytes);
            first_cycle_files = first_cycle_files.max(files.len());
        } else if files.len() > first_cycle_files || bytes > first_cycle_bytes {
            return Ok(false);
        }
    }
    Ok(first_cycle_files == if append_history { 5 } else { 1 })
}

/// Rewrites the store file of `directory`, kept by `build`, with the table of contents `damage` makes of the one it holds.
fn damage_contents(
    directory: &std::path::Path,
    build: &str,
    damage: &dyn Fn(&mut TableOfContents),
) {
    let file = directory.join("lint.store");
    let bytes = std::fs::read(&file).expect("the store");
    let tagged = bytes
        .strip_suffix(super::constants::FORMAT)
        .expect("current version trailer");
    let (body, trailer) = tagged.split_at(tagged.len() - 8);
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
    damaged.extend_from_slice(super::constants::FORMAT);
    std::fs::write(&file, damaged).expect("a damaged store");
}

/// Overwrites the bytes of the section of `bucket` in the store file of `directory`, kept by `build`, with bytes that read back as
/// nothing.
fn damage_bucket(directory: &std::path::Path, build: &str, bucket: Bucket) {
    let file = directory.join("lint.store");
    let mut bytes = std::fs::read(&file).expect("the store");
    let tagged = bytes
        .strip_suffix(super::constants::FORMAT)
        .expect("current version trailer");
    let (body, trailer) = tagged.split_at(tagged.len() - 8);
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
    register_proofs(&mut mine.handed);
    read_by_first.keep(
        mine,
        &KeptRows {
            staged: Vec::new(),
            unchanged: Vec::new(),
            file_again: vec![again(0, 0), again(1, 1)],
        },
    )?;
    let last = store_in(directory, "build-1");
    let all = materialized_handed(&*last.stored()?, &BucketsWanted::All)?;
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
    register_proofs(&mut two.handed);
    store.stored()?.keep(two.clone(), &KeptRows::default())?;
    damage_bucket(directory, "build-1", Bucket(5));
    let reader = store_in(directory, "build-1");
    let read = reader.stored()?;
    let tables_read = read.tables() == tables(two);
    let asked = materialized_handed(&*read, &BucketsWanted::These(vec![Bucket(2)]))?;
    Ok(tables_read
        && asked.frontier == stored(1).handed.frontier
        && read.handed(&BucketsWanted::These(vec![Bucket(5)])).is_err())
}

fn subject(question: Input) -> Result<Value, Failure> {
    let root = tempfile::tempdir().expect("a directory");
    let directory = root.path().join("cache");
    let store = store_in(&directory, "build-1");
    Ok(match question {
        Question::PriorSchemaStoreIsEmpty => {
            store.stored()?.keep(stored(1), &KeptRows::default())?;
            let current = store.stored()?.tables() == tables(stored(1));
            let file = directory.join(super::constants::FILE);
            let bytes = std::fs::read(&file).expect("current store");
            let body = bytes
                .strip_suffix(super::constants::FORMAT)
                .expect("checked current trailer");
            let mut prior = body.to_vec();
            prior.extend_from_slice(
                b"ryo-lint-store\0permissions-v1\0observation-readiness-v2\0project-entries-v2\0",
            );
            std::fs::write(&file, prior).expect("prior schema store");
            let missed = store.stored()?.tables() == Stored::default();
            store.stored()?.keep(stored(1), &KeptRows::default())?;
            current && missed && store.stored()?.tables() == tables(stored(1))
        }
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
        Question::RepeatedKeepsStayBounded => {
            repeated_keeps_within_first_cycle(&directory, &store, false)?
        }
        Question::AppendingHistoryFailsBound => {
            repeated_keeps_within_first_cycle(&directory, &store, true)?
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
            let alone = materialized_handed(&*reader.stored()?, &wanted)?;
            let read = store.stored()?;
            let after = materialized_handed(&*read, &wanted)?;
            let again = materialized_handed(&*read, &wanted)?;
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

#[test]
fn normalized_staging_preserves_store_bytes_and_snapshot_lifetime() {
    use ryo_values::domain::records::types::RecordWords;
    use ryo_values::domain::stores::types::StagedRow;
    let root = tempfile::tempdir().expect("directories");
    let inline_path = root.path().join("inline");
    let staged_path = root.path().join("staged");
    let inline_store = store_in(&inline_path, "staging");
    let staged_store = store_in(&staged_path, "staging");
    let original = stored(3);
    inline_store
        .stored()
        .expect("snapshot")
        .keep(original.clone(), &KeptRows::default())
        .expect("inline keep");
    let read = staged_store.stored().expect("snapshot");
    let mut staged = original.clone();
    let mut kept = KeptRows::default();
    for (ordinal, row) in staged.handed.frontier.drain(..).enumerate() {
        let RecordWords::Staged(words) = read.stage_words(row.words).expect("stage") else {
            panic!("postcard stages fresh payload")
        };
        kept.staged.push(StagedRow {
            file: row.file,
            bucket: row.bucket,
            ordinal: u64::try_from(ordinal).expect("ordinal"),
            words,
        });
    }
    // There is no worker-owned packed payload left in the retained records.
    assert!(staged.handed.frontier.is_empty());
    assert_eq!(kept.staged.len(), original.handed.frontier.len());
    let span = kept.staged.first().expect("staged row").words;
    let mut loaded = ryo_values::domain::stored_facts::types::PackedWords::default();
    read.read_staged(&span, &mut loaded)
        .expect("read after producing worker is gone");
    assert_eq!(loaded, original.handed.frontier[0].words);
    read.keep(staged, &kept).expect("stream staged keep");
    assert_eq!(
        std::fs::read(inline_path.join("lint.store")).expect("inline bytes"),
        std::fs::read(staged_path.join("lint.store")).expect("staged bytes")
    );
    let warm = staged_store.stored().expect("warm");
    assert_eq!(
        materialized_handed(&*warm, &BucketsWanted::All).expect("warm frontiers"),
        original.handed
    );
    // A later snapshot cannot resolve a descriptor, even while its owning snapshot remains alive.
    assert!(warm.read_staged(&span, &mut loaded).is_err());
    let before = std::fs::read(staged_path.join("lint.store")).expect("published bytes");
    assert!(warm.keep(original.clone(), &kept).is_err());
    assert_eq!(
        std::fs::read(staged_path.join("lint.store")).expect("old cache"),
        before
    );
    let mut malformed = kept.clone();
    malformed.staged[0].words.offset = u64::MAX;
    assert!(read.keep(original, &malformed).is_err());
    assert_eq!(
        std::fs::read(staged_path.join("lint.store")).expect("old cache"),
        before
    );
}

#[test]
fn warm_partial_frontier_keeps_exact_proof_through_facts_handover() {
    use ryo_execution::inbound::use_cases::execution::types::facts::{FactsPort as _, WordsToKeep};
    use ryo_relations::domain::relations::types::FileId;
    use ryo_values::domain::{
        records::types::{FrontierAffected, FrontierCoverage},
        stored_facts::types::PackedWords,
        stores::types::ReadWordPayload,
    };

    let root = tempfile::tempdir().expect("directory");
    let store = store_in(root.path(), "partial-frontier");
    let mut original = stored(3);
    let proof = &mut original.handed.frontier[0].proof;
    proof.coverage = FrontierCoverage::Partial(vec![FrontierAffected::CurrentFile]);
    original.handed.proofs[0].proof = proof.clone();
    store
        .stored()
        .expect("cold snapshot")
        .keep(original.clone(), &KeptRows::default())
        .expect("write partial proof");

    let warm = store.stored().expect("warm snapshot");
    let row = warm
        .handed(&BucketsWanted::All)
        .expect("warm frontier")
        .frontier
        .into_iter()
        .next()
        .expect("retained bucket");
    let ReadWordPayload::Old(descriptor) = row.words else {
        panic!("postcard frontier must retain snapshot-bound old descriptor")
    };
    let mut bytes = PackedWords::default();
    warm.read_old(&descriptor, &mut bytes)
        .expect("same snapshot resolves bytes");
    let facts = crate::outbound::facts::factory::memory();
    let mut worker = facts.worker().expect("facts worker");
    worker
        .keep(
            row.bucket,
            WordsToKeep {
                words: &bytes,
                file: FileId(row.file.0),
                proof: &row.proof,
            },
        )
        .expect("warm handover");
    let joined = facts
        .finish(&mut std::iter::once(worker))
        .expect("facts finish")
        .kept(row.bucket)
        .expect("same bucket");
    assert_eq!(joined.words.0, bytes);
    assert_eq!(joined.sources.len(), 1);
    assert_eq!(joined.sources[0].file, FileId(0));
    assert_eq!(joined.sources[0].proof, original.handed.frontier[0].proof);
    assert_eq!(
        joined.sources[0].proof.coverage,
        FrontierCoverage::Partial(vec![FrontierAffected::CurrentFile])
    );
}

#[test]
fn staged_fresh_and_selected_old_rows_match_inline_edit_add_remove_and_reorder() {
    use ryo_values::domain::records::types::RecordWords;
    use ryo_values::domain::stores::types::StagedRow;
    let root = tempfile::tempdir().expect("directories");
    let inline = store_in(&root.path().join("inline"), "edits");
    let staged = store_in(&root.path().join("staged"), "edits");
    for store in [&inline, &staged] {
        store
            .stored()
            .expect("cold")
            .keep(over_b_and_d(), &KeptRows::default())
            .expect("initial");
    }
    for _ in 0..2 {
        let (next, reused) = over_a_b_d_and_e();
        inline
            .stored()
            .expect("old inline")
            .keep(next.clone(), &reused)
            .expect("inline edit");
        let read = staged.stored().expect("old staged");
        let mut next = next;
        let mut reused = reused;
        // Interleave representations so ordinal reconstruction is exercised by the real adapter.
        let rows = std::mem::take(&mut next.handed.frontier);
        for (ordinal, row) in rows.into_iter().enumerate() {
            if ordinal % 2 == 0 {
                let RecordWords::Staged(words) = read.stage_words(row.words).expect("stage") else {
                    panic!("staged")
                };
                reused.staged.push(StagedRow {
                    file: row.file,
                    bucket: row.bucket,
                    ordinal: u64::try_from(ordinal).expect("ordinal"),
                    words,
                });
            } else {
                next.handed.frontier.push(row);
            }
        }
        read.keep(next, &reused).expect("stream edit");
        assert_eq!(
            std::fs::read(root.path().join("inline/lint.store")).expect("inline bytes"),
            std::fs::read(root.path().join("staged/lint.store")).expect("staged bytes")
        );
        assert_eq!(
            materialized_handed(&*inline.stored().expect("warm"), &BucketsWanted::All)
                .expect("inline"),
            materialized_handed(&*staged.stored().expect("warm"), &BucketsWanted::All)
                .expect("staged")
        );
    }
}

#[test]
fn failed_write_flush_or_rename_never_publishes_or_removes_previous_cache() {
    use super::adapter::published;
    let root = tempfile::tempdir().expect("directory");
    let target = root.path().join("lint.store");
    let temporary = root.path().join("new.tmp");
    let previous = b"the complete previous cache";
    std::fs::write(&target, previous).expect("old cache");
    for failure in ["write failure", "flush failure"] {
        std::fs::write(&temporary, b"partial cache").expect("partial new file");
        let renamed = std::cell::Cell::new(false);
        let result = published(&temporary, Err(std::io::Error::other(failure)), || {
            renamed.set(true);
            std::fs::rename(&temporary, &target)
        });
        assert!(result.is_err());
        assert!(!renamed.get());
        assert!(!temporary.exists());
        assert_eq!(std::fs::read(&target).expect("old cache"), previous);
    }
    std::fs::write(&temporary, b"fully flushed new file").expect("new file");
    assert!(
        published(&temporary, Ok(()), || Err(std::io::Error::other(
            "rename failure"
        )))
        .is_err()
    );
    assert!(!temporary.exists());
    assert_eq!(std::fs::read(&target).expect("old cache"), previous);
}

/// Descriptors remain tied to the open old file through atomic replacement and concurrent consumers.
#[test]
fn old_read_descriptors_survive_replacement_and_reject_other_snapshots() {
    use ryo_values::domain::stored_facts::types::PackedWords;
    use ryo_values::domain::stores::types::ReadWordPayload;
    let root = tempfile::tempdir().expect("directory");
    let store = store_in(root.path(), "old-descriptors");
    let mut original = stored(1);
    original
        .handed
        .frontier
        .push(original.handed.frontier[0].clone());
    store
        .stored()
        .unwrap()
        .keep(original.clone(), &KeptRows::default())
        .unwrap();
    let old = store.stored().unwrap();
    let handed = old.handed(&BucketsWanted::All).unwrap();
    assert_eq!(handed.frontier.len(), 2);
    assert!(
        handed
            .frontier
            .iter()
            .all(|row| matches!(row.words, ReadWordPayload::Old(_)))
    );
    let descriptors: Vec<_> = handed
        .frontier
        .iter()
        .map(|row| match row.words {
            ReadWordPayload::Old(words) => words,
            ReadWordPayload::Inline(_) => panic!("postcard retains no old payload"),
        })
        .collect();
    old.keep(stored(2), &KeptRows::default())
        .expect("replace current pathname");
    let current = store.stored().unwrap();
    let mut loaded = PackedWords::default();
    for descriptor in &descriptors {
        old.read_old(descriptor, &mut loaded)
            .expect("original open file");
        assert_eq!(loaded, original.handed.frontier[0].words);
        assert!(current.read_old(descriptor, &mut loaded).is_err());
    }
    let mut bad_bucket = descriptors[0];
    bad_bucket.bucket = Bucket(u32::MAX);
    assert!(old.read_old(&bad_bucket, &mut loaded).is_err());
    std::thread::scope(|scope| {
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let old = &old;
                let descriptor = descriptors[0];
                let expected = &original.handed.frontier[0].words;
                scope.spawn(move || {
                    let mut loaded = PackedWords::default();
                    old.read_old(&descriptor, &mut loaded)
                        .expect("same pinned snapshot");
                    assert_eq!(&loaded, expected);
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
    });
    assert_eq!(
        materialized_handed(&*current, &BucketsWanted::All).unwrap(),
        stored(2).handed
    );
}

#[test]
fn an_open_snapshot_refuses_external_payload_truncation() {
    use ryo_values::domain::stored_facts::types::PackedWords;
    use ryo_values::domain::stores::types::ReadWordPayload;

    let root = tempfile::tempdir().expect("directory");
    let store = store_in(root.path(), "truncated-open-snapshot");
    store
        .stored()
        .unwrap()
        .keep(stored(1), &KeptRows::default())
        .unwrap();
    let snapshot = store.stored().expect("open immutable snapshot");
    let handed = snapshot.handed(&BucketsWanted::All).expect("row framing");
    let descriptor = match handed.frontier.first().expect("a row").words {
        ReadWordPayload::Old(words) => words,
        ReadWordPayload::Inline(_) => panic!("postcard retains descriptors"),
    };
    assert!(descriptor.length > 0);
    std::fs::OpenOptions::new()
        .write(true)
        .open(root.path().join("lint.store"))
        .expect("external writer")
        .set_len(descriptor.offset + descriptor.length - 1)
        .expect("truncate payload after snapshot creation");
    let mut loaded = PackedWords::default();
    assert!(
        snapshot.read_old(&descriptor, &mut loaded).is_err(),
        "the cached original length must never turn truncated bytes into a successful read"
    );
}

#[cfg(unix)]
#[test]
fn bucket_reader_observes_each_row_at_its_buffer_acquisition() {
    use ryo_values::domain::stored_facts::types::PackedWords;
    use ryo_values::domain::stores::types::ReadWordPayload;
    use std::io::{Seek as _, SeekFrom, Write as _};

    for change_header in [true, false] {
        let root = tempfile::tempdir().expect("directory");
        let store = store_in(root.path(), "bucket-window");
        let mut source = stored(1);
        source
            .handed
            .frontier
            .push(source.handed.frontier[0].clone());
        store
            .stored()
            .unwrap()
            .keep(source.clone(), &KeptRows::default())
            .unwrap();
        let snapshot = store.stored().expect("snapshot");
        let rows = snapshot.handed(&BucketsWanted::All).expect("rows").frontier;
        let first = match &rows[0].words {
            ReadWordPayload::Old(words) => *words,
            ReadWordPayload::Inline(_) => panic!("old row"),
        };
        let second = match &rows[1].words {
            ReadWordPayload::Old(words) => *words,
            ReadWordPayload::Inline(_) => panic!("old row"),
        };
        let mut batch = snapshot
            .begin_old_rows(&BucketsWanted::All)
            .expect("reader")
            .expect("postcard reader");
        batch
            .read(&first, &mut PackedWords::default())
            .expect("first row");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(root.path().join("lint.store"))
            .expect("same inode");
        let _position = file
            .seek(SeekFrom::Start(if change_header {
                second.header
            } else {
                second.offset
            }))
            .expect("second row");
        file.write_all(&[0x7f]).expect("equal-length edit");
        drop(file);
        let mut direct = PackedWords::default();
        if change_header {
            assert!(snapshot.read_old(&second, &mut direct).is_err());
        } else {
            snapshot
                .read_old(&second, &mut direct)
                .expect("direct reread");
            assert_ne!(direct, source.handed.frontier[0].words);
        }
        let mut acquired = PackedWords::default();
        batch
            .read(&second, &mut acquired)
            .expect("already buffered second row");
        assert_eq!(acquired, source.handed.frontier[0].words);
    }
}

#[cfg(unix)]
#[test]
fn bucket_reader_acquires_other_bucket_after_mutation_or_truncation() {
    use ryo_values::domain::stored_facts::types::PackedWords;
    use ryo_values::domain::stores::types::ReadWordPayload;
    use std::io::{Seek as _, SeekFrom, Write as _};

    for truncate in [false, true] {
        let root = tempfile::tempdir().expect("directory");
        let store = store_in(root.path(), "bucket-boundary");
        let mut source = stored(1);
        let mut other = source.handed.frontier[0].clone();
        other.bucket = Bucket(17);
        source.handed.frontier.push(other);
        register_proofs(&mut source.handed);
        store
            .stored()
            .unwrap()
            .keep(source, &KeptRows::default())
            .unwrap();
        let snapshot = store.stored().expect("snapshot");
        let rows = snapshot.handed(&BucketsWanted::All).expect("rows").frontier;
        let first = match &rows[0].words {
            ReadWordPayload::Old(words) => *words,
            ReadWordPayload::Inline(_) => panic!("old row"),
        };
        let second = match &rows[1].words {
            ReadWordPayload::Old(words) => *words,
            ReadWordPayload::Inline(_) => panic!("old row"),
        };
        let mut batch = snapshot
            .begin_old_rows(&BucketsWanted::All)
            .expect("reader")
            .expect("postcard reader");
        batch
            .read(&first, &mut PackedWords::default())
            .expect("first bucket");
        let path = root.path().join("lint.store");
        if truncate {
            std::fs::OpenOptions::new()
                .write(true)
                .open(path)
                .unwrap()
                .set_len(second.offset + second.length - 1)
                .expect("truncate second bucket");
        } else {
            let mut file = std::fs::OpenOptions::new().write(true).open(path).unwrap();
            let _position = file.seek(SeekFrom::Start(second.header)).unwrap();
            file.write_all(&[0x7f]).expect("corrupt second header");
        }
        assert!(batch.read(&second, &mut PackedWords::default()).is_err());
    }
}

#[cfg(unix)]
#[test]
fn bucket_readers_keep_snapshot_identity_across_concurrent_consumers() {
    use ryo_values::domain::stored_facts::types::PackedWords;
    use ryo_values::domain::stores::types::ReadWordPayload;

    let root = tempfile::tempdir().expect("directory");
    let store = store_in(root.path(), "parallel-bucket-reader");
    let original = stored(1);
    store
        .stored()
        .unwrap()
        .keep(original.clone(), &KeptRows::default())
        .unwrap();
    let snapshot = store.stored().expect("snapshot");
    let rows = snapshot.handed(&BucketsWanted::All).expect("rows").frontier;
    let descriptor = match &rows[0].words {
        ReadWordPayload::Old(words) => *words,
        ReadWordPayload::Inline(_) => panic!("old row"),
    };
    let foreign = store.stored().expect("other snapshot");
    let foreign_rows = foreign
        .handed(&BucketsWanted::All)
        .expect("foreign rows")
        .frontier;
    let foreign_descriptor = match &foreign_rows[0].words {
        ReadWordPayload::Old(words) => *words,
        ReadWordPayload::Inline(_) => panic!("old row"),
    };
    let mut batch = snapshot
        .begin_old_rows(&BucketsWanted::All)
        .unwrap()
        .unwrap();
    assert!(
        batch
            .read(&foreign_descriptor, &mut PackedWords::default())
            .is_err()
    );
    snapshot
        .keep(stored(2), &KeptRows::default())
        .expect("replace published pathname");
    let mut retained = PackedWords::default();
    batch
        .read(&descriptor, &mut retained)
        .expect("batch retains original open inode");
    assert_eq!(retained, original.handed.frontier[0].words);
    std::thread::scope(|scope| {
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let snapshot = &snapshot;
                let expected = &original.handed.frontier[0].words;
                scope.spawn(move || {
                    let mut reader = snapshot
                        .begin_old_rows(&BucketsWanted::All)
                        .unwrap()
                        .unwrap();
                    let mut loaded = PackedWords::default();
                    reader
                        .read(&descriptor, &mut loaded)
                        .expect("same snapshot");
                    assert_eq!(&loaded, expected);
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
    });
}

mod executable;
