//! Analysis uses its injected language and propagates required failures.
use crate::cases::Case;
use crate::cases::CaseName;
use crate::cases::Expect;
use crate::cases::Selection;
use crate::cases::Word;
use crate::cases::run;
use crate::outbound::corpus::fixtures::fake::CorpusFake;
use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::language::types::ProjectNotes;
use crate::outbound::language::types::ProjectToRead;
use crate::outbound::memory::fixtures::fake::MemoryFake;
use ryo_execution::outbound::digest::fixtures::fake::DigestFake;
use ryo_execution::outbound::facts::fixtures::fake::FactsFake;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::inbound::use_cases::project::types::ManifestFile;
use ryo_language::inbound::use_cases::project::types::ManifestText;
use ryo_language::inbound::use_cases::project::types::RootPath;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use super::errors::FileAnalysisError;
use super::service::FileAnalysis;
#[cfg(test)]
mod capture_test;
#[cfg(test)]
mod failure_test;
#[cfg(test)]
mod gating_test;
#[cfg(test)]
mod manifest_test;
#[cfg(test)]
mod program_test;
#[cfg(test)]
mod replay_test;
#[cfg(test)]
mod resident_test;
#[cfg(test)]
mod visit_test;







type Input = &'static str;
type Value = usize;
type Failure = FileAnalysisError;

fn subject(path: Input) -> Result<Value, Failure> {
    if path == "a_selected_bucket_under_a_stale_proof_is_refused_and_an_unselected_one_is_not_read" {
        return replay_test::a_selected_bucket_under_a_stale_proof_is_refused_and_an_unselected_one_is_not_read();
    }
    if path == "syntax_applicability_skips_only_gated_rules_and_names_the_reason" {
        return gating_test::syntax_applicability_skips_only_gated_rules_and_names_the_reason();
    }
    if path == "only_incomplete_diagnostics_mark_code_relations_as_recovery" {
        return gating_test::only_incomplete_diagnostics_mark_code_relations_as_recovery();
    }
    if path == "required_dependency_stat_and_read_errors_cannot_disappear" {
        return gating_test::required_dependency_stat_and_read_errors_cannot_disappear();
    }
    if path == "unstable_same_shape_resident_populations_remain_distinct" {
        return resident_test::unstable_same_shape_resident_populations_remain_distinct();
    }
    if path == "file_visits_preserve_stable_unstable_and_unread_provider_outcomes" {
        return resident_test::file_visits_preserve_stable_unstable_and_unread_provider_outcomes();
    }
    if path == "a_bad_last_captured_bucket_calls_no_digest_even_after_a_valid_rule_bucket" {
        return capture_test::a_bad_last_captured_bucket_calls_no_digest_even_after_a_valid_rule_bucket();
    }
    if path == "successful_outcomes_stage_once_and_keep_digests_without_retaining_payloads" {
        return capture_test::successful_outcomes_stage_once_and_keep_digests_without_retaining_payloads();
    }
    if path == "staged_replay_reads_only_selected_buckets_after_the_producing_worker_drops" {
        return replay_test::staged_replay_reads_only_selected_buckets_after_the_producing_worker_drops();
    }
    if path == "read_failure_demand_retries_unchanged_metadata_and_clears_reused_worker_bytes" {
        return failure_test::read_failure_demand_retries_unchanged_metadata_and_clears_reused_worker_bytes();
    }
    if path == "read_failure_demand_is_limited_to_governing_rules" {
        return failure_test::read_failure_demand_is_limited_to_governing_rules();
    }
    if path == "discovery_failure_runs_without_parser_bytes_or_permission_invention" {
        return manifest_test::discovery_failure_runs_without_parser_bytes_or_permission_invention();
    }
    if path == "primary_manifest_read_failure_retains_real_metadata_without_empty_project_source" {
        return manifest_test::primary_manifest_read_failure_retains_real_metadata_without_empty_project_source();
    }
    if path == "project_entry_acquisition_deduplicates_provider_candidates_before_observing" {
        return manifest_test::project_entry_acquisition_deduplicates_provider_candidates_before_observing();
    }
    if path == "forgotten-entry-receipt" {
        return manifest_test::a_receipt_that_forgets_the_entries_is_refused();
    }
    if path == "invalid-provider.rs" {
        return visit_test::invalid_provider();
    }
    if path == "manifest-and-stray" {
        return visit_test::manifest_and_stray();
    }
    if path == "digests" {
        return visit_test::digested();
    }
    if path == "forgetful.manifest.fake" {
        return manifest_test::a_project_answer_that_drops_an_observed_entry_is_refused();
    }
    if path == "unlinked-source-provider" {
        let service = FileAnalysis::new(
            Arc::new(CorpusFake::default()),
            Arc::new(LanguageFake::default()),
            Arc::new(DigestFake),
            Arc::new(MemoryFake::default()),
        );
        service.validate_source_language(Some(&ryo_values::domain::descriptions::types::LanguageName(
            "not-linked".into(),
        )))?;
        return Ok(0);
    }
    let language = Arc::new(LanguageFake::default());
    let service = FileAnalysis::new(
        Arc::new(CorpusFake::default()),
        Arc::<LanguageFake>::clone(&language),
        Arc::new(DigestFake),
        Arc::new(MemoryFake::default()),
    );
    let input = ProjectToRead {
        search_scope: &Default::default(),
        given_inventory: &Default::default(),
        observed_entries: &Default::default(),
        repository: ryo_language::inbound::use_cases::project::types::Repository::Disk,
        root: &RootPath(".".into()),
        manifests: &[ManifestFile {
            path: FilePath(path.into()),
            text: ManifestText("".into()),
        }],
        files: &[],
        notes: &ProjectNotes(Vec::new()),
        wanted: &[],
    };
    let mut facts = FactsFake::default().worker()?;
    let projected = service.project_now(&input, &mut Symbols::default(), &mut *facts)?;
    assert!(
        projected.refused.is_empty(),
        "a manifest the language answers is not refused"
    );
    Ok(language.projects.load(Ordering::Relaxed))
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_run_whose_store_keeps_nothing_digests_no_file"),
        word: Word::Succeeds,
        input: || "digests",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("invalid_injected_provider_rows_prevent_an_analysis_result"),
        word: Word::Refuses,
        input: || "invalid-provider.rs",
        expect: Expect::Refusal(|error| {
            if matches!(error, FileAnalysisError::Provider { code: crate::domain::admission::errors::AdmissionError::NodeOutsideTable, provider } if provider == "fake ")
            {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_project_receipt_that_forgets_the_observed_entries_is_refused"),
        word: Word::Refuses,
        input: || "forgotten-entry-receipt",
        expect: Expect::Refusal(|error| {
            if matches!(error, FileAnalysisError::ProjectObservationMismatch) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_source_provider_no_language_links_is_refused_before_any_file_is_read"),
        word: Word::Refuses,
        input: || "unlinked-source-provider",
        expect: Expect::Refusal(|error| {
            if matches!(error, FileAnalysisError::SourceLanguage { name } if name == "not-linked") {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("the_injected_language_performs_project_analysis"),
        word: Word::Succeeds,
        input: || "manifest.fake",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_lint_leaves_unread_the_stray_file_and_not_the_manifest_a_language_reads"),
        word: Word::Succeeds,
        input: || "manifest-and-stray",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_project_answer_that_drops_an_observed_entry_is_refused"),
        word: Word::Refuses,
        input: || "forgetful.manifest.fake",
        expect: Expect::Refusal(|error| {
            if matches!(error, FileAnalysisError::ProjectObservationMismatch) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_required_language_failure_prevents_an_analysis_result"),
        word: Word::Fails,
        input: || "broken.manifest.fake",
        expect: Expect::Refusal(|error| {
            if matches!(error, FileAnalysisError::Language(_)) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_selected_bucket_under_a_stale_proof_is_refused_and_an_unselected_one_is_not_read"),
        word: Word::Succeeds,
        input: || "a_selected_bucket_under_a_stale_proof_is_refused_and_an_unselected_one_is_not_read",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("syntax_applicability_skips_only_gated_rules_and_names_the_reason"),
        word: Word::Succeeds,
        input: || "syntax_applicability_skips_only_gated_rules_and_names_the_reason",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("only_incomplete_diagnostics_mark_code_relations_as_recovery"),
        word: Word::Succeeds,
        input: || "only_incomplete_diagnostics_mark_code_relations_as_recovery",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("required_dependency_stat_and_read_errors_cannot_disappear"),
        word: Word::Succeeds,
        input: || "required_dependency_stat_and_read_errors_cannot_disappear",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("unstable_same_shape_resident_populations_remain_distinct"),
        word: Word::Succeeds,
        input: || "unstable_same_shape_resident_populations_remain_distinct",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("file_visits_preserve_stable_unstable_and_unread_provider_outcomes"),
        word: Word::Succeeds,
        input: || "file_visits_preserve_stable_unstable_and_unread_provider_outcomes",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_bad_last_captured_bucket_calls_no_digest_even_after_a_valid_rule_bucket"),
        word: Word::Succeeds,
        input: || "a_bad_last_captured_bucket_calls_no_digest_even_after_a_valid_rule_bucket",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("successful_outcomes_stage_once_and_keep_digests_without_retaining_payloads"),
        word: Word::Succeeds,
        input: || "successful_outcomes_stage_once_and_keep_digests_without_retaining_payloads",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("staged_replay_reads_only_selected_buckets_after_the_producing_worker_drops"),
        word: Word::Succeeds,
        input: || "staged_replay_reads_only_selected_buckets_after_the_producing_worker_drops",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("read_failure_demand_retries_unchanged_metadata_and_clears_reused_worker_bytes"),
        word: Word::Succeeds,
        input: || "read_failure_demand_retries_unchanged_metadata_and_clears_reused_worker_bytes",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("read_failure_demand_is_limited_to_governing_rules"),
        word: Word::Succeeds,
        input: || "read_failure_demand_is_limited_to_governing_rules",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("discovery_failure_runs_without_parser_bytes_or_permission_invention"),
        word: Word::Succeeds,
        input: || "discovery_failure_runs_without_parser_bytes_or_permission_invention",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("primary_manifest_read_failure_retains_real_metadata_without_empty_project_source"),
        word: Word::Succeeds,
        input: || "primary_manifest_read_failure_retains_real_metadata_without_empty_project_source",
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("project_entry_acquisition_deduplicates_provider_candidates_before_observing"),
        word: Word::Succeeds,
        input: || "project_entry_acquisition_deduplicates_provider_candidates_before_observing",
        expect: Expect::Value(1),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
