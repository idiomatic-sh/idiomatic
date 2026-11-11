//! What a file the run uses again hands the cross-file steps, read back from the record or the store, and the staging of fresh outcomes.
use super::super::errors::FileAnalysisError;
use super::super::service::FileAnalysis;
use super::super::types::{FileVisit, RequestVisitSetting, Worker};
use crate::application::file_analysis::constants::STORED_FRONTIER_PROOF_DOES_NOT_MATCH;
use crate::domain::handing::service::hand_rest_again;
use crate::domain::handing::service::remap_to;
use crate::domain::handing::service::rows_of_file;
use crate::domain::keeping::service::remap_packed;
use crate::domain::keeping::types::Remap;
use crate::domain::run_rules::service::governing;
use crate::domain::workers::types::FileOrder;
use crate::domain::workers::types::Held;
use crate::domain::workers::types::Ready;
use crate::domain::workers::types::ReadyBucket;
use ryo_execution::inbound::use_cases::execution::interface::ProgramPreparedExecutionPort as _;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_execution_runtime::outbound::facts::types::WordsToKeep;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Words;
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::records::types::FileOutcome;
use ryo_values::domain::records::types::GoverningRules;
use ryo_values::domain::records::types::HandedNotes;
use ryo_values::domain::records::types::Handing;
use ryo_values::domain::records::types::Record;
use ryo_values::domain::stored_facts::types::ArenaBytes;
use ryo_values::domain::stored_facts::types::Bucket;
use ryo_values::domain::stored_facts::types::PackedWords;
use ryo_values::domain::stored_findings::types::StoredFileNumber;
use ryo_values::domain::stores::types::BucketsWanted;
use ryo_values::domain::table_words::service::pack_onto;
use ryo_values::domain::table_words::types::WordSlice;

impl FileAnalysis {
    /// Only successfully validated and digested fresh outcomes cross this IO boundary.
    pub(in crate::application::file_analysis) fn stage_outcome(
        read: Option<&dyn crate::outbound::lint_store::interface::ReadStore>,
        mut outcome: Option<FileOutcome>,
    ) -> Result<Option<FileOutcome>, FileAnalysisError> {
        let Some(read) = read else {
            return Ok(outcome);
        };
        let ran = match outcome.as_mut() {
            Some(FileOutcome::Read(ran) | FileOutcome::Unread { ran: Some(ran), .. }) => Some(ran),
            None | Some(FileOutcome::Unread { ran: None, .. }) => None,
        };
        if let Some(ran) = ran
            && let Handing::Made(handed) = &mut ran.handed
        {
            for bucket in &mut handed.buckets {
                let words = std::mem::replace(
                    &mut bucket.handed,
                    ryo_values::domain::records::types::RecordWords::Inline(PackedWords::default()),
                );
                bucket.handed = match words {
                    ryo_values::domain::records::types::RecordWords::Inline(words) => {
                        read.stage_words(words)?
                    }
                    staged @ ryo_values::domain::records::types::RecordWords::Staged(_) => staged,
                };
            }
        }
        Ok(outcome)
    }

    /// Hands what a file the run uses again hands the steps that read across files, when they are made again: one path whether the
    /// record holds it (the file was read alone before the run) or the store kept it (read as it is handed, its words moved as they
    /// are unpacked, none copied first). The run is ready to be handed only when it makes those steps again.
    pub(in crate::application::file_analysis) fn hand_again(
        worker: &mut Worker,
        visit: &FileVisit<'_>,
        at: FileOrder,
        number: FileId,
        record: &Record,
    ) -> Result<(), FileAnalysisError> {
        let setting = &visit.setting;
        if !matches!(
            setting.reuse,
            ryo_values::domain::stores::types::Reuse::Cross(_)
        ) {
            return Ok(());
        }
        let ran = match &record.outcome {
            FileOutcome::Read(ran) | FileOutcome::Unread { ran: Some(ran), .. } => ran,
            FileOutcome::Unread { ran: None, .. } => return Ok(()),
        };
        let map = remap_to(&worker.tables, number);
        let mut out = Words::default();
        let (no_text, no_governing) = (ArenaBytes::default(), GoverningRules::default());
        let ready = match &ran.handed {
            Handing::Made(handed) => Ready {
                text: &handed.text,
                notes: match &handed.notes {
                    HandedNotes::Noted(notes) => Some(notes),
                    HandedNotes::Nothing => None,
                },
                governing: &handed.governing,
                partials: &handed.partials,
                observations: match &handed.observations {
                    ryo_values::domain::records::types::HandedReadiness::Recorded(root) => {
                        Some(root)
                    }
                    ryo_values::domain::records::types::HandedReadiness::Unrecorded => None,
                },
                buckets: handed
                    .buckets
                    .iter()
                    .map(|each| {
                        let held = match &each.handed {
                            ryo_values::domain::records::types::RecordWords::Inline(words) => {
                                Held::Words(words)
                            }
                            ryo_values::domain::records::types::RecordWords::Staged(span) => {
                                Held::Staged(span)
                            }
                        };
                        ReadyBucket {
                            bucket: each.bucket,
                            proof: &each.proof,
                            held,
                        }
                    })
                    .collect(),
            },
            Handing::Kept => {
                let Ok(place) = setting
                    .stored
                    .files
                    .binary_search_by(|each| each.path.cmp(&record.path))
                else {
                    return Ok(());
                };
                let file = StoredFileNumber(u32::try_from(place).unwrap_or(u32::MAX));
                let kept = setting.handed;
                let covered = rows_of_file(&kept.covered, file, |each| each.file).first();
                Ready {
                    text: rows_of_file(&kept.texts, file, |each| each.file)
                        .first()
                        .map_or(&no_text, |each| &each.text),
                    notes: rows_of_file(&kept.notes, file, |each| each.file)
                        .first()
                        .map(|each| &each.notes),
                    governing: covered.map_or(&no_governing, |each| &each.governing),
                    partials: covered.map_or(&[][..], |each| &each.partials),
                    observations: match rows_of_file(&kept.observations, file, |each| each.file) {
                        [one] => Some(&one.root),
                        _ => None,
                    },
                    buckets: rows_of_file(&kept.frontier, file, |each| each.file)
                        .iter()
                        .map(|each| {
                            let held = match &each.payload {
                                ryo_values::domain::stores::types::ReadWordPayload::Inline(
                                    words,
                                ) => Held::Words(words),
                                ryo_values::domain::stores::types::ReadWordPayload::Old(span) => {
                                    Held::Old(span)
                                }
                            };
                            ReadyBucket {
                                bucket: each.bucket,
                                proof: &each.proof,
                                held,
                            }
                        })
                        .collect(),
                }
            }
        };
        hand_rest_again(
            &mut worker.tables,
            at,
            number,
            ready.text,
            ready.notes,
            ready.governing,
            ready.partials,
            ready.observations,
        );
        let mut loaded = PackedWords::default();
        for ReadyBucket {
            bucket,
            proof,
            held,
        } in ready.buckets
        {
            // Filtering must precede any staged or old read, including an invalid unrequested descriptor.
            if let BucketsWanted::These(buckets) = setting.wanted_buckets
                && buckets.binary_search(&bucket).is_err()
            {
                continue;
            }
            let words = match held {
                Held::Words(words) => words,
                Held::Staged(span) => {
                    let read = visit
                        .store
                        .ok_or(crate::outbound::lint_store::errors::LintStoreError::NothingRead)?;
                    read.read_staged(span, &mut loaded)?;
                    &loaded
                }
                Held::Old(span) => {
                    let read = visit
                        .store
                        .ok_or(crate::outbound::lint_store::errors::LintStoreError::NothingRead)?;
                    if worker.old_rows.is_none() {
                        worker.old_rows = read.begin_old_rows(setting.wanted_buckets)?;
                    }
                    if let Some(batch) = worker.old_rows.as_mut() {
                        batch.read(span, &mut loaded)?;
                    } else {
                        read.read_old(span, &mut loaded)?;
                    }
                    &loaded
                }
            };
            Self::hand_words_again(
                worker, setting, number, bucket, proof, words, &map, &mut out,
            )?;
        }
        Ok(())
    }

    /// Hands the words a file handed the last run in `bucket`, moved as `map` says, to the worker's bucket, when the run makes the
    /// steps that read the bucket again; words that cannot be moved are left out. `out` is room the words are moved into.
    ///
    /// Order: both callers select the bucket first (`wanted_buckets`) and read no words, staged or old, of an unselected bucket, so
    /// this is reached only for a selected one. It checks the proof of that bucket against the prepared root (version, rule digest,
    /// frontier ordinal, capture flag, first bucket, coverage) before it moves any words, and refuses a stale one; the proof of an
    /// unselected bucket is never checked, as its words are never read.
    pub(in crate::application::file_analysis) fn hand_words_again(
        worker: &mut Worker,
        setting: &RequestVisitSetting<'_>,
        file: FileId,
        bucket: Bucket,
        proof: &ryo_values::domain::records::types::FrontierProof,
        words: &PackedWords,
        map: &Remap,
        out: &mut Words,
    ) -> Result<(), FileAnalysisError> {
        use ryo_values::domain::records::types::FrontierCoverage;
        let rule = RuleIndex(usize::try_from(proof.rule.0).unwrap_or(usize::MAX));
        let input = usize::try_from(proof.input.0).unwrap_or(usize::MAX);
        let valid = setting
            .program
            .rule_digest(rule)
            .is_ok_and(|digest| digest == proof.rule_digest)
            && setting
                .program
                .frontiers(rule)
                .is_ok_and(|frontiers| frontiers.get(input).is_some())
            && setting
                .metadata
                .get(rule.0)
                .and_then(|rule| rule.frontier_capture.get(input))
                .copied()
                == Some(ryo_values::domain::run_metadata::types::FrontierFeed::Captured)
            && setting.first_bucket.get(rule.0).copied().map(|first| {
                ryo_values::domain::stored_facts::service::frontier_bucket(
                    first,
                    InputNumber(input),
                )
            }) == Some(bucket)
            && match &proof.coverage {
                FrontierCoverage::Complete => true,
                FrontierCoverage::Partial(affected) => {
                    ryo_values::domain::records::service::is_well_formed(affected)
                        == ryo_values::domain::records::types::FrontierShape::WellFormed
                }
            };
        if !valid {
            return Err(
                ryo_execution::inbound::use_cases::execution::interface::OperatingError::ObservationIncomplete {
                    reason: STORED_FRONTIER_PROOF_DOES_NOT_MATCH.into(),
                }
                .into(),
            );
        }
        out.0.clear();
        if remap_packed(words, map, out).is_some() {
            let packed = &mut worker.tables.packed;
            packed.0.clear();
            pack_onto(WordSlice(&out.0), packed);
            worker.facts.keep(
                bucket,
                WordsToKeep {
                    words: &worker.tables.packed,
                    file,
                    proof,
                },
            )?;
        }
        Ok(())
    }
}
