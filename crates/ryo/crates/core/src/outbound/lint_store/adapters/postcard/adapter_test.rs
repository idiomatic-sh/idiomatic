// ignore: idiomatic.arch/file-length - the one adapter_test.rs the adapter spec places for this adapter: the port contract rows and the cases of its writer and its connection run through it
//! The postcard adapter of the store port: the core's contract rows run through it, then the cases specific to it.

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::io::Write as _;
use super::adapter::LintStorePostcardAdapter;
use super::schemas::contents;
use super::schemas::contents_bytes;
use super::types::SectionLength;
use super::types::TableOfContents;
use crate::cases::Case;
use crate::cases::CaseName;
use crate::cases::Expect;
use crate::cases::Word;
use crate::cases::run;
use crate::cases::Matched;
use crate::cases::selects;
use crate::outbound::lint_store::errors::LintStoreError;
use crate::outbound::lint_store::fixtures::contract::check;
use crate::outbound::lint_store::fixtures::handed::materialized_handed;
use crate::outbound::lint_store::fixtures::tables::again;
use crate::outbound::lint_store::fixtures::tables::over_a_b_d_and_e;
use crate::outbound::lint_store::fixtures::tables::over_b_and_d;
use crate::outbound::lint_store::fixtures::tables::register_proofs;
use crate::outbound::lint_store::fixtures::tables::stored;
use crate::outbound::lint_store::fixtures::tables::words_of;
use crate::outbound::lint_store::interface::LintStorePort as _;
use crate::outbound::lint_store::types::BuildIdentity;
use crate::outbound::lint_store::types::CacheDirectory;
use crate::outbound::lint_store::types::Store;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stores::types::BucketsWanted;
use ryo_values::domain::stores::types::KeptRows;
use ryo_values::domain::stores::types::Stored;
use crate::outbound::lint_store::adapters::postcard::types::Payload;
use crate::outbound::lint_store::adapters::postcard::adapter::fresh;
use crate::outbound::lint_store::adapters::postcard::writer::old_rows;
use crate::outbound::lint_store::adapters::postcard::writer::write_bucket;
use crate::outbound::lint_store::adapters::postcard::schemas::bucket_bytes;
use crate::outbound::lint_store::adapters::postcard::schemas::rebuilt_bucket_bytes;
use crate::outbound::lint_store::adapters::postcard::connection::StagingConnection;
use crate::outbound::lint_store::adapters::postcard::types::FileOffset;
use crate::outbound::lint_store::adapters::postcard::types::SectionSpan;
use ryo_values::domain::records::types::PayloadLength;
use ryo_values::domain::records::types::StagingSession;
use ryo_values::domain::stored_facts::types::PackedWords;
use ryo_values::domain::stored_findings::types::StoredFileNumber;
use ryo_values::domain::stores::types::FileAgain;
use ryo_values::domain::stores::types::StagedRow;
use ryo_values::domain::stores::types::StoredWords;
use crate::outbound::lint_store::adapters::postcard::writer::old_rows_in;
use crate::outbound::lint_store::adapters::postcard::connection::BUFFER_BYTES;
use std::io::Cursor;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use crate::outbound::lint_store::adapters::postcard::writer::old_word_rows_in;
use crate::outbound::lint_store::adapters::postcard::writer::read_old_in;
use ryo_values::domain::stores::types::OldWords;
use ryo_values::domain::stores::types::StoreSnapshot;
use crate::outbound::lint_store::adapters::postcard::connection::append;
use crate::outbound::lint_store::adapters::postcard::connection::read;
use crate::outbound::lint_store::adapters::postcard::connection::validate;
use ryo_values::domain::records::types::StagedWords;
use std::io::Write;
use crate::outbound::lint_store::adapters::postcard::connection::copy_buffered;
use std::cell::Cell;
use std::io::BufReader;
use std::io::BufWriter;
use std::rc::Rc;
use super::constants::FILE;
use super::constants::TRAILER;
use ryo_values::domain::files::types::ExecutablePermission;
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
    /// normalized staging preserves store bytes and snapshot lifetime.
    NormalizedStagingPreservesStoreBytesAndSnapshotLifetime,
    /// warm partial frontier keeps exact proof through facts handover.
    WarmPartialFrontierKeepsExactProofThroughFactsHandover,
    /// staged fresh and selected old rows match inline edit add remove and reorder.
    StagedFreshAndSelectedOldRowsMatchInlineEditAddRemoveAndReorder,
    /// failed write flush or rename never publishes or removes previous cache.
    FailedWriteFlushOrRenameNeverPublishesOrRemovesPreviousCache,
    /// old read descriptors survive replacement and reject other snapshots.
    OldReadDescriptorsSurviveReplacementAndRejectOtherSnapshots,
    /// an open snapshot refuses external payload truncation.
    AnOpenSnapshotRefusesExternalPayloadTruncation,
    #[cfg(unix)]
    /// bucket reader observes each row at its buffer acquisition.
    BucketReaderObservesEachRowAtItsBufferAcquisition,
    #[cfg(unix)]
    /// bucket reader acquires other bucket after mutation or truncation.
    BucketReaderAcquiresOtherBucketAfterMutationOrTruncation,
    #[cfg(unix)]
    /// bucket readers keep snapshot identity across concurrent consumers.
    BucketReadersKeepSnapshotIdentityAcrossConcurrentConsumers,
    /// inline and staged mixed rows match legacy bytes at varint boundaries.
    InlineAndStagedMixedRowsMatchLegacyBytesAtVarintBoundaries,
    /// old spans and fresh rows preserve selected file remapping and tie order.
    OldSpansAndFreshRowsPreserveSelectedFileRemappingAndTieOrder,
    /// corrupt headers order and truncated payloads are refused.
    CorruptHeadersOrderAndTruncatedPayloadsAreRefused,
    /// streamed payload copies have bounded write chunks and propagate write failure.
    StreamedPayloadCopiesHaveBoundedWriteChunksAndPropagateWriteFailure,
    /// scanning tiny old rows reads and seeks by chunks.
    ScanningTinyOldRowsReadsAndSeeksByChunks,
    /// old descriptor enumeration skips huge payloads and resolution reads one.
    OldDescriptorEnumerationSkipsHugePayloadsAndResolutionReadsOne,
    /// old payloads and headers share one exact read without neighbor prefetch.
    OldPayloadsAndHeadersShareOneExactReadWithoutNeighborPrefetch,
    #[cfg(unix)]
    /// positional old payload read preserves cursor and refuses late truncation.
    PositionalOldPayloadReadPreservesCursorAndRefusesLateTruncation,
    /// spool lifetime and foreign overflow truncation checks are not empty fallbacks.
    SpoolLifetimeAndForeignOverflowTruncationChecksAreNotEmptyFallbacks,
    /// failed creation write flush and read do not commit a descriptor.
    FailedCreationWriteFlushAndReadDoNotCommitADescriptor,
    /// contiguous tiny payloads use chunk io instead of row io.
    ContiguousTinyPayloadsUseChunkIoInsteadOfRowIo,
    /// known true false and unavailable observations roundtrip through the store.
    KnownTrueFalseAndUnavailableObservationsRoundtripThroughTheStore,
    /// missing or changed format tag is a cache miss without mutating old bytes.
    MissingOrChangedFormatTagIsACacheMissWithoutMutatingOldBytes,
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
            selects(Matched(matches!(error, LintStoreError::Unwritable { .. })))
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
            selects(Matched(matches!(error, LintStoreError::Unreadable { .. })))
        }),
    },
    Case {
        name: CaseName("a_keep_that_must_copy_words_from_a_store_nobody_read_is_refused"),
        word: Word::Refuses,
        input: || Question::AKeepOfWordsNobodyReadIsRefused,
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, LintStoreError::NothingRead)))),
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
    Case {
        name: CaseName("normalized_staging_preserves_store_bytes_and_snapshot_lifetime"),
        word: Word::Succeeds,
        input: || Question::NormalizedStagingPreservesStoreBytesAndSnapshotLifetime,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("warm_partial_frontier_keeps_exact_proof_through_facts_handover"),
        word: Word::Succeeds,
        input: || Question::WarmPartialFrontierKeepsExactProofThroughFactsHandover,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("staged_fresh_and_selected_old_rows_match_inline_edit_add_remove_and_reorder"),
        word: Word::Succeeds,
        input: || Question::StagedFreshAndSelectedOldRowsMatchInlineEditAddRemoveAndReorder,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("failed_write_flush_or_rename_never_publishes_or_removes_previous_cache"),
        word: Word::Succeeds,
        input: || Question::FailedWriteFlushOrRenameNeverPublishesOrRemovesPreviousCache,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("old_read_descriptors_survive_replacement_and_reject_other_snapshots"),
        word: Word::Succeeds,
        input: || Question::OldReadDescriptorsSurviveReplacementAndRejectOtherSnapshots,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("an_open_snapshot_refuses_external_payload_truncation"),
        word: Word::Succeeds,
        input: || Question::AnOpenSnapshotRefusesExternalPayloadTruncation,
        expect: Expect::Value(true),
    },
    #[cfg(unix)]
    Case {
        name: CaseName("bucket_reader_observes_each_row_at_its_buffer_acquisition"),
        word: Word::Succeeds,
        input: || Question::BucketReaderObservesEachRowAtItsBufferAcquisition,
        expect: Expect::Value(true),
    },
    #[cfg(unix)]
    Case {
        name: CaseName("bucket_reader_acquires_other_bucket_after_mutation_or_truncation"),
        word: Word::Succeeds,
        input: || Question::BucketReaderAcquiresOtherBucketAfterMutationOrTruncation,
        expect: Expect::Value(true),
    },
    #[cfg(unix)]
    Case {
        name: CaseName("bucket_readers_keep_snapshot_identity_across_concurrent_consumers"),
        word: Word::Succeeds,
        input: || Question::BucketReadersKeepSnapshotIdentityAcrossConcurrentConsumers,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("inline_and_staged_mixed_rows_match_legacy_bytes_at_varint_boundaries"),
        word: Word::Succeeds,
        input: || Question::InlineAndStagedMixedRowsMatchLegacyBytesAtVarintBoundaries,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("old_spans_and_fresh_rows_preserve_selected_file_remapping_and_tie_order"),
        word: Word::Succeeds,
        input: || Question::OldSpansAndFreshRowsPreserveSelectedFileRemappingAndTieOrder,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("corrupt_headers_order_and_truncated_payloads_are_refused"),
        word: Word::Succeeds,
        input: || Question::CorruptHeadersOrderAndTruncatedPayloadsAreRefused,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("streamed_payload_copies_have_bounded_write_chunks_and_propagate_write_failure"),
        word: Word::Succeeds,
        input: || Question::StreamedPayloadCopiesHaveBoundedWriteChunksAndPropagateWriteFailure,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("scanning_tiny_old_rows_reads_and_seeks_by_chunks"),
        word: Word::Succeeds,
        input: || Question::ScanningTinyOldRowsReadsAndSeeksByChunks,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("old_descriptor_enumeration_skips_huge_payloads_and_resolution_reads_one"),
        word: Word::Succeeds,
        input: || Question::OldDescriptorEnumerationSkipsHugePayloadsAndResolutionReadsOne,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("old_payloads_and_headers_share_one_exact_read_without_neighbor_prefetch"),
        word: Word::Succeeds,
        input: || Question::OldPayloadsAndHeadersShareOneExactReadWithoutNeighborPrefetch,
        expect: Expect::Value(true),
    },
    #[cfg(unix)]
    Case {
        name: CaseName("positional_old_payload_read_preserves_cursor_and_refuses_late_truncation"),
        word: Word::Succeeds,
        input: || Question::PositionalOldPayloadReadPreservesCursorAndRefusesLateTruncation,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("spool_lifetime_and_foreign_overflow_truncation_checks_are_not_empty_fallbacks"),
        word: Word::Succeeds,
        input: || Question::SpoolLifetimeAndForeignOverflowTruncationChecksAreNotEmptyFallbacks,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("failed_creation_write_flush_and_read_do_not_commit_a_descriptor"),
        word: Word::Succeeds,
        input: || Question::FailedCreationWriteFlushAndReadDoNotCommitADescriptor,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("contiguous_tiny_payloads_use_chunk_io_instead_of_row_io"),
        word: Word::Succeeds,
        input: || Question::ContiguousTinyPayloadsUseChunkIoInsteadOfRowIo,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("known_true_false_and_unavailable_observations_roundtrip_through_the_store"),
        word: Word::Succeeds,
        input: || Question::KnownTrueFalseAndUnavailableObservationsRoundtripThroughTheStore,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("missing_or_changed_format_tag_is_a_cache_miss_without_mutating_old_bytes"),
        word: Word::Succeeds,
        input: || Question::MissingOrChangedFormatTagIsACacheMissWithoutMutatingOldBytes,
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
        .strip_suffix(super::constants::TRAILER)
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
    damaged.extend_from_slice(super::constants::TRAILER);
    std::fs::write(&file, damaged).expect("a damaged store");
}

/// Overwrites the bytes of the section of `bucket` in the store file of `directory`, kept by `build`, with bytes that read back as
/// nothing.
fn damage_bucket(directory: &std::path::Path, build: &str, bucket: Bucket) {
    let file = directory.join("lint.store");
    let mut bytes = std::fs::read(&file).expect("the store");
    let tagged = bytes
        .strip_suffix(super::constants::TRAILER)
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
        && matches!(
            read.handed(&BucketsWanted::These(vec![Bucket(5)])),
            Err(LintStoreError::Unreadable { .. })
        ))
}

/// Distinguishable rows include duplicate file identities and arbitrary packed bytes.
fn row(file: u32, length: usize) -> StoredWords {
    StoredWords {
        file: StoredFileNumber(file),
        bucket: Bucket(3),
        proof: crate::outbound::lint_store::fixtures::proof(),
        words: PackedWords(
            (0..length)
                .map(|at| u8::try_from(at % 256).expect("byte"))
                .collect(),
        ),
    }
}

fn inline_and_staged_mixed_rows_match_legacy_bytes_at_varint_boundaries() -> Result<Value, Failure> {
    for length in [0, 1, 127, 128, 16383, 16384, 100_000] {
        let rows = [row(128, length), row(0, 0), row(128, 9)];
        let expected = bucket_bytes(&rows.iter().collect::<Vec<_>>()).expect("legacy bytes");
        let mut spool = StagingConnection::new(StagingSession(1)).expect("spool");
        let span = spool.append(rows[1].words.clone()).expect("stage");
        let staged = [StagedRow {
            file: rows[1].file,
            bucket: rows[1].bucket,
            ordinal: ryo_values::domain::records::types::StagedOrdinal(1),
            words: span,
        }];
        let inline = [rows[0].clone(), rows[2].clone()];
        let groups = fresh(&inline, &staged).expect("mixed order");
        let mut bytes = Vec::new();
        let mut at = FileOffset(0);
        let _section = write_bucket(
            &mut bytes,
            &mut at,
            &groups[&Bucket(3)],
            None,
            Some(&mut spool),
        )
        .expect("stream");
        assert_eq!(bytes, expected);
        assert_eq!(at.0, u64::try_from(expected.len()).expect("size"));
    }
    let mut bytes = Vec::new();
    let mut at = FileOffset(0);
    let _empty_section = write_bucket(&mut bytes, &mut at, &[], None, None).expect("empty bucket");
    assert_eq!(bytes, bucket_bytes(&[]).expect("empty legacy"));
    Ok(true)
}

fn old_spans_and_fresh_rows_preserve_selected_file_remapping_and_tie_order() -> Result<Value, Failure> {
    let old = [row(0, 128), row(1, 16384), row(1, 7), row(2, 99)];
    let old_bytes = bucket_bytes(&old.iter().collect::<Vec<_>>()).expect("legacy");
    let mut file = tempfile::tempfile().expect("old snapshot");
    file.write_all(&old_bytes).expect("write");
    let span = SectionSpan {
        start: FileOffset(0),
        length: SectionLength(u64::try_from(old_bytes.len()).expect("size")),
    };
    let parsed = old_rows(&mut file, span).expect("headers only");
    assert!(
        parsed
            .iter()
            .all(|row| matches!(row.payload, Payload::Old(_)))
    );
    let again = [FileAgain {
        was: StoredFileNumber(1),
        now: StoredFileNumber(0),
    }];
    let fresh_rows = [row(0, 3), row(2, 4)];
    let mut groups = fresh(&fresh_rows, &[]).expect("fresh");
    let rows = groups.get_mut(&Bucket(3)).expect("bucket");
    rows.extend(parsed.into_iter().filter_map(|mut row| {
        let mapping = again.iter().find(|entry| entry.was == row.file)?;
        row.file = mapping.now;
        Some(row)
    }));
    rows.sort_by_key(|row| row.file);
    let mut actual = Vec::new();
    let mut at = FileOffset(0);
    let _copied_section =
        write_bucket(&mut actual, &mut at, rows, Some(&mut file), None).expect("bounded copy");
    assert_eq!(
        Some(actual),
        rebuilt_bucket_bytes(
            &fresh_rows.iter().collect::<Vec<_>>(),
            Some(&old_bytes),
            &again
        )
        .expect("legacy rebuilt")
    );
    Ok(true)
}

fn corrupt_headers_order_and_truncated_payloads_are_refused() -> Result<Value, Failure> {
    for bytes in [vec![], vec![1], vec![1, 0, 9, 1], vec![0, 0], vec![255; 12]] {
        let mut file = tempfile::tempfile().expect("file");
        file.write_all(&bytes).expect("write");
        let _refusal = old_rows(
                &mut file,
                SectionSpan {
                    start: FileOffset(0),
                    length: SectionLength(u64::try_from(bytes.len()).expect("size"))
                }
            ).expect_err("a refusal");
    }
    let mut spool = StagingConnection::new(StagingSession(2)).expect("spool");
    let words = spool.append(PackedWords(vec![1])).expect("stage");
    let row = StagedRow {
        file: StoredFileNumber(0),
        bucket: Bucket(3),
        ordinal: ryo_values::domain::records::types::StagedOrdinal(1),
        words,
    };
    let _refusal = fresh(&[], &[row]).expect_err("a refusal");
    let _refusal = fresh(
            &[],
            &[
                StagedRow {
                    ordinal: ryo_values::domain::records::types::StagedOrdinal(0),
                    ..row
                },
                StagedRow {
                    ordinal: ryo_values::domain::records::types::StagedOrdinal(0),
                    ..row
                }
            ]
        ).expect_err("a refusal");
    let frames = [crate::outbound::lint_store::adapters::postcard::types::Frame {
        file: StoredFileNumber(0),
        payload: Payload::Staged(words),
    }];
    let mut at = FileOffset(u64::MAX);
    let _refusal = write_bucket(&mut Vec::new(), &mut at, &frames, None, Some(&mut spool)).expect_err("a refusal");
    let _refusal = write_bucket(
            &mut std::io::sink(),
            &mut FileOffset(0),
            &frames,
            None,
            None
        ).expect_err("a refusal");
    Ok(true)
}

fn streamed_payload_copies_have_bounded_write_chunks_and_propagate_write_failure() -> Result<Value, Failure> {
    let mut spool = StagingConnection::new(StagingSession(4)).expect("spool");
    let span = spool
        .append(PackedWords(vec![5; 1024 * 1024]))
        .expect("large frontier");
    let rows = [crate::outbound::lint_store::adapters::postcard::types::Frame {
        file: StoredFileNumber(0),
        payload: Payload::Staged(span),
    }];
    let mut output = ObservedWriter {
        largest: 0,
        bytes: 0,
        fail_after: usize::MAX,
    };
    let _streamed_section = write_bucket(
        &mut output,
        &mut FileOffset(0),
        &rows,
        None,
        Some(&mut spool),
    )
    .expect("bounded copy");
    assert!(output.largest < usize::try_from(span.length.0).expect("size"));
    assert!(output.bytes >= usize::try_from(span.length.0).expect("size"));
    let mut failing = ObservedWriter {
        largest: 0,
        bytes: 0,
        fail_after: 16,
    };
    let mut position = FileOffset(9);
    let _refusal = write_bucket(&mut failing, &mut position, &rows, None, Some(&mut spool)).expect_err("a refusal");
    assert_eq!(
        position,
        FileOffset(9),
        "failed bucket never publishes a completed span"
    );
    Ok(true)
}

fn scanning_tiny_old_rows_reads_and_seeks_by_chunks() -> Result<Value, Failure> {
    let rows = vec![row(0, 13); 10_000];
    let bytes = bucket_bytes(&rows.iter().collect::<Vec<_>>()).expect("existing wire");
    let length = u64::try_from(bytes.len()).expect("size");
    let mut file = Counted {
        inner: Cursor::new(bytes),
        reads: 0,
        seeks: 0,
    };
    let spans = old_rows_in(
        &mut file,
        SectionSpan {
            start: FileOffset(0),
            length: SectionLength(length),
        },
        SectionLength(length),
    )
    .expect("scan exact production path");
    assert_eq!(spans.len(), rows.len());
    let chunks = usize::try_from(length)
        .expect("size")
        .div_ceil(BUFFER_BYTES);
    assert!(
        file.reads <= chunks + 1,
        "reads are buffered, not one per header byte"
    );
    assert!(
        file.seeks <= chunks + 2,
        "small payload skips stay inside the buffer"
    );
    Ok(true)
}

/// A virtual huge bucket exposes actual read volume without allocating its payload set.
fn old_descriptor_enumeration_skips_huge_payloads_and_resolution_reads_one() -> Result<Value, Failure> {
    let count = postcard::to_allocvec(&3_usize).unwrap();
    let mut at = count.len() as u64;
    let mut headers = vec![(0, count)];
    let payload = 2 * 1024 * 1024_usize;
    for file in [2_u32, 0, 2] {
        let mut header = postcard::to_allocvec(&file).unwrap();
        header.extend(postcard::to_allocvec(&payload).unwrap());
        let header_length = header.len() as u64;
        headers.push((at, header));
        at += header_length + payload as u64;
    }
    let mut file = Sparse {
        headers,
        length: at,
        at: 0,
        bytes: 0,
    };
    let bucket = SectionSpan {
        start: FileOffset(0),
        length: SectionLength(at),
    };
    let rows = old_word_rows_in(&mut file, bucket, SectionLength(at)).expect("header scan");
    assert_eq!(
        rows.iter().map(|row| row.file.0).collect::<Vec<_>>(),
        [2, 0, 2]
    );
    assert!(
        file.bytes <= 3 * BUFFER_BYTES,
        "only bounded header read-ahead, never six MiB payload ownership"
    );
    let descriptor = OldWords {
        snapshot: StoreSnapshot(91),
        bucket: Bucket(3),
        file: rows[1].file,
        header: ryo_values::domain::records::types::HeaderOffset(rows[1].header),
        offset: ryo_values::domain::records::types::PayloadOffset(rows[1].span.start.0),
        length: ryo_values::domain::records::types::PayloadLength(rows[1].span.length.0),
    };
    file.bytes = 0;
    let mut loaded = PackedWords::default();
    read_old_in(
        &mut file,
        SectionLength(at),
        StoreSnapshot(91),
        bucket,
        &descriptor,
        &mut loaded,
    )
    .expect("one payload");
    assert_eq!(loaded.0.len(), payload);
    assert!(loaded.0.iter().all(|byte| *byte == 0x5a));
    assert_eq!(
        file.bytes as u64,
        descriptor.length.0 + descriptor.offset.0 - descriptor.header.0
    );
    // A tiny requested bucket must not read ahead into an adjacent unrequested section.
    let mut tiny = Sparse {
        headers: vec![(0, vec![1, 0, 0])],
        length: 1024 * 1024,
        at: 0,
        bytes: 0,
    };
    let tiny_rows = old_word_rows_in(
        &mut tiny,
        SectionSpan {
            start: FileOffset(0),
            length: SectionLength(3),
        },
        SectionLength(1024 * 1024),
    )
    .expect("one empty row");
    assert_eq!(tiny_rows.len(), 1);
    assert_eq!(
        tiny.bytes, 3,
        "no bytes from the unrequested neighboring bucket"
    );
    let _refusal = read_old_in(
            &mut file,
            SectionLength(at - 1),
            StoreSnapshot(91),
            bucket,
            &descriptor,
            &mut loaded
        ).expect_err("a refusal");
    for bad in [
        OldWords {
            snapshot: StoreSnapshot(92),
            ..descriptor
        },
        OldWords {
            length: ryo_values::domain::records::types::PayloadLength(u64::MAX),
            ..descriptor
        },
        OldWords {
            header: ryo_values::domain::records::types::HeaderOffset(u64::MAX),
            ..descriptor
        },
        OldWords {
            offset: ryo_values::domain::records::types::PayloadOffset(descriptor.offset.0 + 1),
            ..descriptor
        },
        OldWords {
            file: StoredFileNumber(99),
            ..descriptor
        },
    ] {
        let _refusal = read_old_in(&mut file, SectionLength(at), StoreSnapshot(91), bucket, &bad, &mut loaded).expect_err("a refusal");
    }
    Ok(true)
}

fn old_payloads_and_headers_share_one_exact_read_without_neighbor_prefetch() -> Result<Value, Failure> {
    for file_number in [0, 127, 128, u32::MAX] {
        for length in [0, 1, 127, 128, 16_384] {
            let selected = row(file_number, length);
            let neighbor = row(7, 23);
            let bytes = bucket_bytes(&[&selected, &neighbor]).expect("two rows");
            let held = u64::try_from(bytes.len()).expect("length");
            let bucket = SectionSpan {
                start: FileOffset(0),
                length: SectionLength(held),
            };
            let mut file = ExactReadCounted {
                bytes: std::io::Cursor::new(bytes),
                reads: 0,
                transferred: 0,
            };
            let headers = old_word_rows_in(&mut file, bucket, SectionLength(held)).expect("framing");
            let first = headers.first().expect("selected row");
            let descriptor = OldWords {
                snapshot: StoreSnapshot(81),
                bucket: Bucket(3),
                file: first.file,
                header: ryo_values::domain::records::types::HeaderOffset(first.header),
                offset: ryo_values::domain::records::types::PayloadOffset(first.span.start.0),
                length: ryo_values::domain::records::types::PayloadLength(first.span.length.0),
            };
            file.reads = 0;
            file.transferred = 0;
            let mut loaded = PackedWords::default();
            read_old_in(
                &mut file,
                SectionLength(held),
                StoreSnapshot(81),
                bucket,
                &descriptor,
                &mut loaded,
            )
            .expect("selected payload");
            assert_eq!(loaded, selected.words);
            assert_eq!(file.reads, 1);
            assert_eq!(
                u64::try_from(file.transferred).expect("count"),
                descriptor.offset.0 - descriptor.header.0 + descriptor.length.0,
                "only this exact header and payload are read"
            );
            let malformed = OldWords {
                header: ryo_values::domain::records::types::HeaderOffset(bucket.start.0 + 1),
                offset: ryo_values::domain::records::types::PayloadOffset(bucket.start.0 + 17),
                length: ryo_values::domain::records::types::PayloadLength(0),
                ..descriptor
            };
            file.reads = 0;
            let _refusal = read_old_in(
                    &mut file,
                    SectionLength(held),
                    StoreSnapshot(81),
                    bucket,
                    &malformed,
                    &mut loaded
                ).expect_err("a refusal");
            assert_eq!(file.reads, 0, "oversized headers are refused before IO");
            let header = usize::try_from(descriptor.header.0).expect("header offset");
            file.bytes.get_mut()[header] ^= 1;
            let _refusal = read_old_in(
                &mut file,
                SectionLength(held),
                StoreSnapshot(81),
                bucket,
                &descriptor,
                &mut loaded,
            )
            .expect_err("changed framing is refused after snapshot enumeration");
            assert!(loaded.0.is_empty(), "rejected payload is not returned");
        }
    }
    Ok(true)
}

#[cfg(unix)]
fn positional_old_payload_read_preserves_cursor_and_refuses_late_truncation() -> Result<Value, Failure> {
    use crate::outbound::lint_store::adapters::postcard::writer::{old_word_rows_in, read_old_file_in};
    use ryo_values::domain::stores::types::{OldWords, StoreSnapshot};
    use std::io::{Seek as _, SeekFrom};

    let selected = row(128, 16_384);
    let neighbor = row(7, 23);
    let bytes = bucket_bytes(&[&selected, &neighbor]).expect("two rows");
    let held = u64::try_from(bytes.len()).expect("length");
    let bucket = SectionSpan {
        start: FileOffset(0),
        length: SectionLength(held),
    };
    let mut file = tempfile::tempfile().expect("snapshot");
    file.write_all(&bytes).expect("write rows");
    let headers = old_word_rows_in(&mut file, bucket, SectionLength(held)).expect("framing");
    let first = headers.first().expect("selected row");
    let descriptor = OldWords {
        snapshot: StoreSnapshot(81),
        bucket: Bucket(3),
        file: first.file,
        header: ryo_values::domain::records::types::HeaderOffset(first.header),
        offset: ryo_values::domain::records::types::PayloadOffset(first.span.start.0),
        length: ryo_values::domain::records::types::PayloadLength(first.span.length.0),
    };
    let cursor = file.seek(SeekFrom::End(0)).expect("cursor");
    let mut loaded = PackedWords::default();
    read_old_file_in(
        &file,
        SectionLength(held),
        StoreSnapshot(81),
        bucket,
        &descriptor,
        &mut loaded,
    )
    .expect("positional payload");
    assert_eq!(loaded, selected.words);
    assert_eq!(file.seek(SeekFrom::Current(0)).expect("cursor"), cursor);

    file.set_len(descriptor.offset.0 + descriptor.length.0 - 1)
        .expect("truncate selected payload");
    let _refusal = read_old_file_in(
            &file,
            SectionLength(held),
            StoreSnapshot(81),
            bucket,
            &descriptor,
            &mut loaded
        ).expect_err("a refusal");
    assert!(loaded.0.is_empty());
    Ok(true)
}





struct ObservedWriter {
    largest: usize,
    bytes: usize,
    fail_after: usize,
}

impl std::io::Write for ObservedWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.largest = self.largest.max(bytes.len());
        if self.bytes >= self.fail_after {
            return Err(std::io::Error::other("injected final write failure"));
        }
        self.bytes += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}


struct Counted {
    inner: Cursor<Vec<u8>>,
    reads: usize,
    seeks: usize,
}

impl Read for Counted {
    fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
        self.reads += 1;
        self.inner.read(into)
    }
}

impl Seek for Counted {
    fn seek(&mut self, at: SeekFrom) -> std::io::Result<u64> {
        self.seeks += 1;
        self.inner.seek(at)
    }
}


struct Sparse {
    headers: Vec<(u64, Vec<u8>)>,
    length: u64,
    at: u64,
    bytes: usize,
}

impl Read for Sparse {
    fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
        let length = into
            .len()
            .min(usize::try_from(self.length - self.at).unwrap());
        into[..length].fill(0x5a);
        for (start, header) in &self.headers {
            let begin = self.at.max(*start);
            let end = (self.at + length as u64).min(*start + header.len() as u64);
            if begin < end {
                into[(begin - self.at) as usize..(end - self.at) as usize]
                    .copy_from_slice(&header[(begin - start) as usize..(end - start) as usize]);
            }
        }
        self.at += length as u64;
        self.bytes += length;
        Ok(length)
    }
}

impl Seek for Sparse {
    fn seek(&mut self, seek: SeekFrom) -> std::io::Result<u64> {
        let at = match seek {
            SeekFrom::Start(at) => i128::from(at),
            SeekFrom::Current(by) => i128::from(self.at) + i128::from(by),
            SeekFrom::End(by) => i128::from(self.length) + i128::from(by),
        };
        self.at = u64::try_from(at).map_err(std::io::Error::other)?;
        Ok(self.at)
    }
}


struct ExactReadCounted {
    bytes: std::io::Cursor<Vec<u8>>,
    reads: usize,
    transferred: usize,
}

impl Read for ExactReadCounted {
    fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
        self.reads += 1;
        let count = self.bytes.read(into)?;
        self.transferred += count;
        Ok(count)
    }
}

impl Seek for ExactReadCounted {
    fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
        self.bytes.seek(from)
    }
}


#[derive(Default)]
struct FaultFile {
    bytes: Cursor<Vec<u8>>,
    write_fails: bool,
    read_fails: bool,
    flush_fails: bool,
}
impl Write for FaultFile {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.write_fails {
            return Err(std::io::Error::other("write failure"));
        }
        self.bytes.write(bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        if self.flush_fails {
            return Err(std::io::Error::other("flush failure"));
        }
        Ok(())
    }
}
impl Read for FaultFile {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        if self.read_fails {
            return Err(std::io::Error::other("read failure"));
        }
        self.bytes.read(bytes)
    }
}
impl Seek for FaultFile {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        self.bytes.seek(position)
    }
}

fn spool_lifetime_and_foreign_overflow_truncation_checks_are_not_empty_fallbacks() -> Result<Value, Failure> {
    let mut spool = StagingConnection::new(StagingSession(1)).expect("temporary spool");
    let path = spool.file.path().to_owned();
    assert!(path.exists());
    let first = spool.append(PackedWords(vec![1, 2, 3])).expect("append");
    let second = spool
        .append(PackedWords(vec![9; 10000]))
        .expect("second append");
    let mut loaded = PackedWords::default();
    spool
        .read(&first, &mut loaded)
        .expect("selected first only");
    assert_eq!(loaded.0, [1, 2, 3]);
    assert!(
        loaded.0.capacity() < usize::try_from(second.length.0).expect("size"),
        "never read every staged bucket at once"
    );
    let _refusal = spool
            .read(
                &StagedWords {
                    session: StagingSession(2),
                    ..first
                },
                &mut loaded
            ).expect_err("a refusal");
    let _refusal = validate(
            spool.session,
            spool.length,
            &StagedWords {
                offset: ryo_values::domain::records::types::PayloadOffset(u64::MAX),
                length: ryo_values::domain::records::types::PayloadLength(1),
                ..first
            }
        ).expect_err("a refusal");
    let _refusal = validate(
            spool.session,
            spool.length,
            &StagedWords {
                offset: ryo_values::domain::records::types::PayloadOffset(0),
                length: ryo_values::domain::records::types::PayloadLength(u64::MAX),
                ..first
            }
        ).expect_err("a refusal");
    spool
        .file
        .as_file_mut()
        .set_len(1)
        .expect("truncate injected input");
    let _empty = spool
        .append(PackedWords::default())
        .expect("append mode discards prior read-ahead without extending the truncated file");
    let _refusal = spool.read(&second, &mut loaded).expect_err("a refusal");
    drop(spool);
    assert!(
        !path.exists(),
        "RAII closes and deletes on success or failure"
    );
    Ok(true)
}

fn failed_creation_write_flush_and_read_do_not_commit_a_descriptor() -> Result<Value, Failure> {
    let _refusal = StagingConnection::create(StagingSession(1), || Err(std::io::Error::other(
            "create failure"
        ))).expect_err("a refusal");
    for (write_fails, flush_fails) in [(true, false), (false, true)] {
        let mut file = FaultFile {
            write_fails,
            flush_fails,
            ..FaultFile::default()
        };
        let mut held = PayloadLength(0);
        let _refusal = append(
                &mut file,
                StagingSession(1),
                &mut held,
                &PackedWords(vec![1, 2])
            ).expect_err("a refusal");
        assert_eq!(held, PayloadLength(0));
    }
    let mut file = FaultFile::default();
    let mut held = PayloadLength(0);
    let span = append(
        &mut file,
        StagingSession(1),
        &mut held,
        &PackedWords(vec![1, 2]),
    )
    .expect("valid control");
    let mut out = PackedWords::default();
    file.read_fails = true;
    let _refusal = read(&mut file, StagingSession(1), held, &span, &mut out).expect_err("a refusal");
    file.read_fails = false;
    file.flush_fails = true;
    let _refusal = read(&mut file, StagingSession(1), held, &span, &mut out).expect_err("a refusal");
    let mut impossible = u64::MAX;
    let _refusal = append(
            &mut file,
            StagingSession(1),
            &mut impossible,
            &PackedWords(vec![0])
        ).expect_err("a refusal");
    assert_eq!(impossible, u64::MAX);
    Ok(true)
}

fn contiguous_tiny_payloads_use_chunk_io_instead_of_row_io() -> Result<Value, Failure> {
    let rows = 10_000;
    let width = 13;
    let payload = vec![7; rows * width];
    let counts = Rc::new(Counts::default());
    let mut read = BufReader::with_capacity(
        BUFFER_BYTES,
        SpoolCounted {
            inner: Cursor::new(payload.clone()),
            counts: Rc::clone(&counts),
        },
    );
    let mut write = BufWriter::with_capacity(
        BUFFER_BYTES,
        SpoolCounted {
            inner: Cursor::new(Vec::new()),
            counts: Rc::clone(&counts),
        },
    );
    for _ in 0..rows {
        copy_buffered(
            &mut read,
            &mut write,
            PayloadLength(u64::try_from(width).expect("width")),
        )
            .expect("one span");
    }
    write.flush().expect("publication barrier");
    assert_eq!(write.get_ref().inner.get_ref(), &payload);
    assert!(counts.reads.get() <= payload.len().div_ceil(BUFFER_BYTES) + 1);
    assert!(counts.writes.get() <= payload.len().div_ceil(BUFFER_BYTES) + 1);
    assert_eq!(counts.seeks.get(), 0);
    // Buffer acceptance is not publication: a later flush refusal remains an error.
    let mut failing = BufWriter::with_capacity(
        BUFFER_BYTES,
        FaultFile {
            write_fails: true,
            ..FaultFile::default()
        },
    );
    failing
        .write_all(b"accepted only into memory")
        .expect("buffered");
    let _refusal = failing.flush().expect_err("a refusal");
    Ok(true)
}


#[derive(Default)]
struct Counts {
    reads: Cell<usize>,
    writes: Cell<usize>,
    seeks: Cell<usize>,
}

struct SpoolCounted {
    inner: Cursor<Vec<u8>>,
    counts: Rc<Counts>,
}

impl Read for SpoolCounted {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        self.counts.reads.set(self.counts.reads.get() + 1);
        self.inner.read(bytes)
    }
}

impl Write for SpoolCounted {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.counts.writes.set(self.counts.writes.get() + 1);
        self.inner.write(bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Seek for SpoolCounted {
    fn seek(&mut self, at: SeekFrom) -> std::io::Result<u64> {
        self.counts.seeks.set(self.counts.seeks.get() + 1);
        self.inner.seek(at)
    }
}


fn known_true_false_and_unavailable_observations_roundtrip_through_the_store() -> Result<Value, Failure> {
    let root = tempfile::tempdir().unwrap();
    let adapter = store_in(root.path(), "permissions");
    for observation in [
        ExecutablePermission::PosixNotExecutable,
        ExecutablePermission::PosixExecutable,
        ExecutablePermission::Unknown,
        ExecutablePermission::Unsupported,
    ] {
        let mut kept = stored(1);
        kept.files[0].executable = observation;
        adapter
            .stored()
            .unwrap()
            .keep(kept.clone(), &KeptRows::default())
            .unwrap();
        let bytes = std::fs::read(root.path().join(FILE)).unwrap();
        assert!(bytes.ends_with(TRAILER));
        assert_eq!(adapter.stored().unwrap().tables().files, kept.files);
    }
    Ok(true)
}

fn missing_or_changed_format_tag_is_a_cache_miss_without_mutating_old_bytes() -> Result<Value, Failure> {
    let root = tempfile::tempdir().unwrap();
    let adapter = store_in(root.path(), "permissions");
    adapter
        .stored()
        .unwrap()
        .keep(stored(1), &KeptRows::default())
        .unwrap();
    let path = root.path().join(FILE);
    let valid = std::fs::read(&path).unwrap();
    let untagged = valid[..valid.len() - TRAILER.len()].to_vec();
    let mut changed = valid.clone();
    *changed.last_mut().unwrap() ^= 1;
    for bytes in [untagged, changed] {
        std::fs::write(&path, &bytes).unwrap();
        assert!(adapter.stored().unwrap().tables().files.is_empty());
        assert_eq!(
            std::fs::read(&path).unwrap(),
            bytes,
            "a rejected cache is not deleted or rewritten during read"
        );
    }
    adapter
        .stored()
        .unwrap()
        .keep(stored(1), &KeptRows::default())
        .unwrap();
    assert!(
        !adapter.stored().unwrap().tables().files.is_empty(),
        "ordinary recomputation publishes a usable versioned store"
    );
    Ok(true)
}

fn subject(question: Input) -> Result<Value, Failure> {
    let root = tempfile::tempdir().expect("a directory");
    let directory = root.path().join("cache");
    let store = store_in(&directory, "build-1");
    Ok(match question {
        Question::NormalizedStagingPreservesStoreBytesAndSnapshotLifetime => normalized_staging_preserves_store_bytes_and_snapshot_lifetime()?,
        Question::WarmPartialFrontierKeepsExactProofThroughFactsHandover => warm_partial_frontier_keeps_exact_proof_through_facts_handover()?,
        Question::StagedFreshAndSelectedOldRowsMatchInlineEditAddRemoveAndReorder => staged_fresh_and_selected_old_rows_match_inline_edit_add_remove_and_reorder()?,
        Question::FailedWriteFlushOrRenameNeverPublishesOrRemovesPreviousCache => failed_write_flush_or_rename_never_publishes_or_removes_previous_cache()?,
        Question::OldReadDescriptorsSurviveReplacementAndRejectOtherSnapshots => old_read_descriptors_survive_replacement_and_reject_other_snapshots()?,
        Question::AnOpenSnapshotRefusesExternalPayloadTruncation => an_open_snapshot_refuses_external_payload_truncation()?,
        #[cfg(unix)]
        Question::BucketReaderObservesEachRowAtItsBufferAcquisition => bucket_reader_observes_each_row_at_its_buffer_acquisition()?,
        #[cfg(unix)]
        Question::BucketReaderAcquiresOtherBucketAfterMutationOrTruncation => bucket_reader_acquires_other_bucket_after_mutation_or_truncation()?,
        #[cfg(unix)]
        Question::BucketReadersKeepSnapshotIdentityAcrossConcurrentConsumers => bucket_readers_keep_snapshot_identity_across_concurrent_consumers()?,
        Question::PriorSchemaStoreIsEmpty => {
            store.stored()?.keep(stored(1), &KeptRows::default())?;
            let current = store.stored()?.tables() == tables(stored(1));
            let file = directory.join(super::constants::FILE);
            let bytes = std::fs::read(&file).expect("current store");
            let body = bytes
                .strip_suffix(super::constants::TRAILER)
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
        Question::InlineAndStagedMixedRowsMatchLegacyBytesAtVarintBoundaries => inline_and_staged_mixed_rows_match_legacy_bytes_at_varint_boundaries()?,
        Question::OldSpansAndFreshRowsPreserveSelectedFileRemappingAndTieOrder => old_spans_and_fresh_rows_preserve_selected_file_remapping_and_tie_order()?,
        Question::CorruptHeadersOrderAndTruncatedPayloadsAreRefused => corrupt_headers_order_and_truncated_payloads_are_refused()?,
        Question::StreamedPayloadCopiesHaveBoundedWriteChunksAndPropagateWriteFailure => streamed_payload_copies_have_bounded_write_chunks_and_propagate_write_failure()?,
        Question::ScanningTinyOldRowsReadsAndSeeksByChunks => scanning_tiny_old_rows_reads_and_seeks_by_chunks()?,
        Question::OldDescriptorEnumerationSkipsHugePayloadsAndResolutionReadsOne => old_descriptor_enumeration_skips_huge_payloads_and_resolution_reads_one()?,
        Question::OldPayloadsAndHeadersShareOneExactReadWithoutNeighborPrefetch => old_payloads_and_headers_share_one_exact_read_without_neighbor_prefetch()?,
        #[cfg(unix)]
        Question::PositionalOldPayloadReadPreservesCursorAndRefusesLateTruncation => positional_old_payload_read_preserves_cursor_and_refuses_late_truncation()?,
        Question::SpoolLifetimeAndForeignOverflowTruncationChecksAreNotEmptyFallbacks => spool_lifetime_and_foreign_overflow_truncation_checks_are_not_empty_fallbacks()?,
        Question::FailedCreationWriteFlushAndReadDoNotCommitADescriptor => failed_creation_write_flush_and_read_do_not_commit_a_descriptor()?,
        Question::ContiguousTinyPayloadsUseChunkIoInsteadOfRowIo => contiguous_tiny_payloads_use_chunk_io_instead_of_row_io()?,
        Question::KnownTrueFalseAndUnavailableObservationsRoundtripThroughTheStore => known_true_false_and_unavailable_observations_roundtrip_through_the_store()?,
        Question::MissingOrChangedFormatTagIsACacheMissWithoutMutatingOldBytes => missing_or_changed_format_tag_is_a_cache_miss_without_mutating_old_bytes()?,
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














fn normalized_staging_preserves_store_bytes_and_snapshot_lifetime() -> Result<Value, Failure> {
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
            ordinal: ryo_values::domain::records::types::StagedOrdinal(
                u64::try_from(ordinal).expect("ordinal"),
            ),
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
    let _refused = warm
        .read_staged(&span, &mut loaded)
        .expect_err("a later snapshot cannot resolve a descriptor");
    let before = std::fs::read(staged_path.join("lint.store")).expect("published bytes");
    let _refused = warm
        .keep(original.clone(), &kept)
        .expect_err("a later snapshot cannot keep over the open one");
    assert_eq!(
        std::fs::read(staged_path.join("lint.store")).expect("old cache"),
        before
    );
    let mut malformed = kept.clone();
    malformed.staged[0].words.offset = ryo_values::domain::records::types::PayloadOffset(u64::MAX);
    let _refused = read
        .keep(original, &malformed)
        .expect_err("a malformed staged span is refused");
    assert_eq!(
        std::fs::read(staged_path.join("lint.store")).expect("old cache"),
        before
    );
    Ok(true)
}

fn warm_partial_frontier_keeps_exact_proof_through_facts_handover() -> Result<Value, Failure> {
    use ryo_execution::inbound::use_cases::execution::types::facts::{FactsPort as _, WordsToKeep};
    use ryo_relations::domain::relations::types::FileId;
    use ryo_values::domain::{
        records::types::{FrontierAffected, FrontierCoverage, FrontierMissing},
        stored_facts::types::PackedWords,
        stores::types::ReadWordPayload,
    };

    let root = tempfile::tempdir().expect("directory");
    let store = store_in(root.path(), "partial-frontier");
    let mut original = stored(3);
    let proof = &mut original.handed.frontier[0].proof;
    proof.coverage = FrontierCoverage::Partial(vec![FrontierAffected::CurrentFile(FrontierMissing::Any)]);
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
        .0
        .frontier
        .into_iter()
        .next()
        .expect("retained bucket");
    let ReadWordPayload::Old(descriptor) = row.payload else {
        panic!("postcard frontier must retain snapshot-bound old descriptor")
    };
    let mut bytes = PackedWords::default();
    warm.read_old(&descriptor, &mut bytes)
        .expect("same snapshot resolves bytes");
    let facts = ryo_execution::outbound::facts::factory::memory();
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
        FrontierCoverage::Partial(vec![FrontierAffected::CurrentFile(FrontierMissing::Any)])
    );
    Ok(true)
}

fn staged_fresh_and_selected_old_rows_match_inline_edit_add_remove_and_reorder() -> Result<Value, Failure> {
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
                    ordinal: ryo_values::domain::records::types::StagedOrdinal(
                        u64::try_from(ordinal).expect("ordinal"),
                    ),
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
    Ok(true)
}

fn failed_write_flush_or_rename_never_publishes_or_removes_previous_cache() -> Result<Value, Failure> {
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
        let _refused = result.expect_err("the failed rename is reported");
        assert!(!renamed.get());
        assert!(!temporary.exists());
        assert_eq!(std::fs::read(&target).expect("old cache"), previous);
    }
    std::fs::write(&temporary, b"fully flushed new file").expect("new file");
    let _refused = published(&temporary, Ok(()), || {
        Err(std::io::Error::other("rename failure"))
    })
    .expect_err("a failed rename is reported");
    assert!(!temporary.exists());
    assert_eq!(std::fs::read(&target).expect("old cache"), previous);
    Ok(true)
}

/// Descriptors remain tied to the open old file through atomic replacement and concurrent consumers.
fn old_read_descriptors_survive_replacement_and_reject_other_snapshots() -> Result<Value, Failure> {
    use ryo_values::domain::stored_facts::types::PackedWords;
    use ryo_values::domain::stores::types::ReadWordPayload;
    let root = tempfile::tempdir().expect("directory");
    let store = store_in(root.path(), "old-descriptors");
    let mut original = stored(1);
    let first = original.handed.frontier[0].clone();
    original.handed.frontier.push(first);
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
            .all(|row| matches!(row.payload, ReadWordPayload::Old(_)))
    );
    let descriptors: Vec<_> = handed
        .frontier
        .iter()
        .map(|row| match row.payload {
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
        let _refused = current
            .read_old(descriptor, &mut loaded)
            .expect_err("another snapshot's descriptor is refused");
    }
    let mut bad_bucket = descriptors[0];
    bad_bucket.bucket = Bucket(u32::MAX);
    let _refused = old
        .read_old(&bad_bucket, &mut loaded)
        .expect_err("an absent bucket is refused");
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
    Ok(true)
}

fn an_open_snapshot_refuses_external_payload_truncation() -> Result<Value, Failure> {
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
    let descriptor = match handed.frontier.first().expect("a row").payload {
        ReadWordPayload::Old(words) => words,
        ReadWordPayload::Inline(_) => panic!("postcard retains descriptors"),
    };
    assert!(descriptor.length.0 > 0);
    std::fs::OpenOptions::new()
        .write(true)
        .open(root.path().join("lint.store"))
        .expect("external writer")
        .set_len(descriptor.offset.0 + descriptor.length.0 - 1)
        .expect("truncate payload after snapshot creation");
    let mut loaded = PackedWords::default();
    let _refused = snapshot.read_old(&descriptor, &mut loaded).expect_err(
        "the cached original length must never turn truncated bytes into a successful read",
    );
    Ok(true)
}

#[cfg(unix)]
fn bucket_reader_observes_each_row_at_its_buffer_acquisition() -> Result<Value, Failure> {
    use ryo_values::domain::stored_facts::types::PackedWords;
    use ryo_values::domain::stores::types::ReadWordPayload;
    use std::io::{Seek as _, SeekFrom, Write as _};

    for change_header in [true, false] {
        let root = tempfile::tempdir().expect("directory");
        let store = store_in(root.path(), "bucket-window");
        let mut source = stored(1);
        let first = source.handed.frontier[0].clone();
        source.handed.frontier.push(first);
        store
            .stored()
            .unwrap()
            .keep(source.clone(), &KeptRows::default())
            .unwrap();
        let snapshot = store.stored().expect("snapshot");
        let rows = snapshot
            .handed(&BucketsWanted::All)
            .expect("rows")
            .0
            .frontier;
        let first = match &rows[0].payload {
            ReadWordPayload::Old(words) => *words,
            ReadWordPayload::Inline(_) => panic!("old row"),
        };
        let second = match &rows[1].payload {
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
                second.header.0
            } else {
                second.offset.0
            }))
            .expect("second row");
        file.write_all(&[0x7f]).expect("equal-length edit");
        drop(file);
        let mut direct = PackedWords::default();
        if change_header {
            let _refused = snapshot
                .read_old(&second, &mut direct)
                .expect_err("a corrupt header is refused");
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
    Ok(true)
}

#[cfg(unix)]
fn bucket_reader_acquires_other_bucket_after_mutation_or_truncation() -> Result<Value, Failure> {
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
        let rows = snapshot
            .handed(&BucketsWanted::All)
            .expect("rows")
            .0
            .frontier;
        let first = match &rows[0].payload {
            ReadWordPayload::Old(words) => *words,
            ReadWordPayload::Inline(_) => panic!("old row"),
        };
        let second = match &rows[1].payload {
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
                .set_len(second.offset.0 + second.length.0 - 1)
                .expect("truncate second bucket");
        } else {
            let mut file = std::fs::OpenOptions::new().write(true).open(path).unwrap();
            let _position = file.seek(SeekFrom::Start(second.header.0)).unwrap();
            file.write_all(&[0x7f]).expect("corrupt second header");
        }
        let _refused = batch
            .read(&second, &mut PackedWords::default())
            .expect_err("a corrupt header is refused by the batch");
    }
    Ok(true)
}

#[cfg(unix)]
fn bucket_readers_keep_snapshot_identity_across_concurrent_consumers() -> Result<Value, Failure> {
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
    let rows = snapshot
        .handed(&BucketsWanted::All)
        .expect("rows")
        .0
        .frontier;
    let descriptor = match &rows[0].payload {
        ReadWordPayload::Old(words) => *words,
        ReadWordPayload::Inline(_) => panic!("old row"),
    };
    let foreign = store.stored().expect("other snapshot");
    let foreign_rows = foreign
        .handed(&BucketsWanted::All)
        .expect("foreign rows")
        .0
        .frontier;
    let foreign_descriptor = match &foreign_rows[0].payload {
        ReadWordPayload::Old(words) => *words,
        ReadWordPayload::Inline(_) => panic!("old row"),
    };
    let mut batch = snapshot
        .begin_old_rows(&BucketsWanted::All)
        .unwrap()
        .unwrap();
    let _refused = batch
        .read(&foreign_descriptor, &mut PackedWords::default())
        .expect_err("a foreign descriptor is refused by the batch");
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
    Ok(true)
}
