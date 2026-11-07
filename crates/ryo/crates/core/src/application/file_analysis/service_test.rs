//! Analysis uses its injected language and propagates required failures.
use super::errors::FileAnalysisError;
use super::service::{FileAnalysis, recovery_of};
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::outbound::corpus::fixtures::fake::CorpusFake;
use ryo_execution::outbound::digest::fixtures::fake::DigestFake;
use ryo_execution::outbound::facts::fixtures::fake::FactsFake;
use ryo_execution::inbound::use_cases::execution::types::facts::FactsPort as _;

use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::language::types::{ProjectToRead, ProjectNotes};
use crate::outbound::memory::fixtures::fake::MemoryFake;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_language::inbound::use_cases::project::types::{ManifestFile, ManifestText};
use std::sync::{Arc, atomic::Ordering};

use super::types::{RequestVisitSetting, RuleGate};
use crate::domain::workers::types::Gating;
use crate::domain::workers::service::tables_for;
use ryo_execution::inbound::use_cases::execution::types::Counting;
use ryo_execution::inbound::use_cases::execution::types::matching::MatchingPort as _;
use ryo_execution::inbound::use_cases::execution::{
    interface::ExecutionUseCase,
    types::{ObservationIdentity, PreparedRequest},
    use_case::ExecutionUseCaseImpl,
};
use ryo_execution::inbound::use_cases::metadata::interface::{Mask, PlacedScopes, ScopePlacement};
use ryo_execution::inbound::use_cases::metadata::interface::{Needs, RelationRead};
use ryo_language::domain::elements::types::SyntaxAcceptance;
use ryo_relations::domain::relations::types::{Relation, WorkerNumber};
use ryo_values::domain::plan_parts::types::{ColumnIndex, StepId};
use ryo_values::domain::plans::types::{Emit, Plan};
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};

use ryo_values::domain::stores::types::BucketsWanted;

/// What the last run kept, for a run that uses nothing of it.
static NOTHING_STORED: std::sync::LazyLock<ryo_values::domain::stores::types::Stored> =
    std::sync::LazyLock::new(ryo_values::domain::stores::types::Stored::default);

/// What the files of the last run handed, for a run that uses nothing of it.
static NOTHING_HANDED: std::sync::LazyLock<ryo_values::domain::stores::types::ReadHanded> =
    std::sync::LazyLock::new(ryo_values::domain::stores::types::ReadHanded::default);

/// No bucket of what the files handed, for a run that reads none.
static NOTHING_WANTED: ryo_values::domain::stores::types::BucketsWanted =
    ryo_values::domain::stores::types::BucketsWanted::These(Vec::new());

type Input = &'static str;
type Value = usize;
type Failure = FileAnalysisError;

fn subject(path: Input) -> Result<Value, Failure> {
    if path == "a_selected_bucket_under_a_stale_proof_is_refused_and_an_unselected_one_is_not_read" {
        return a_selected_bucket_under_a_stale_proof_is_refused_and_an_unselected_one_is_not_read();
    }
    if path == "syntax_applicability_skips_only_gated_rules_and_names_the_reason" {
        return syntax_applicability_skips_only_gated_rules_and_names_the_reason();
    }
    if path == "only_incomplete_diagnostics_mark_code_relations_as_recovery" {
        return only_incomplete_diagnostics_mark_code_relations_as_recovery();
    }
    if path == "required_dependency_stat_and_read_errors_cannot_disappear" {
        return required_dependency_stat_and_read_errors_cannot_disappear();
    }
    if path == "unstable_same_shape_resident_populations_remain_distinct" {
        return unstable_same_shape_resident_populations_remain_distinct();
    }
    if path == "file_visits_preserve_stable_unstable_and_unread_provider_outcomes" {
        return file_visits_preserve_stable_unstable_and_unread_provider_outcomes();
    }
    if path == "a_bad_last_captured_bucket_calls_no_digest_even_after_a_valid_rule_bucket" {
        return a_bad_last_captured_bucket_calls_no_digest_even_after_a_valid_rule_bucket();
    }
    if path == "successful_outcomes_stage_once_and_keep_digests_without_retaining_payloads" {
        return successful_outcomes_stage_once_and_keep_digests_without_retaining_payloads();
    }
    if path == "staged_replay_reads_only_selected_buckets_after_the_producing_worker_drops" {
        return staged_replay_reads_only_selected_buckets_after_the_producing_worker_drops();
    }
    if path == "read_failure_demand_retries_unchanged_metadata_and_clears_reused_worker_bytes" {
        return read_failure_demand_retries_unchanged_metadata_and_clears_reused_worker_bytes();
    }
    if path == "read_failure_demand_is_limited_to_governing_rules" {
        return read_failure_demand_is_limited_to_governing_rules();
    }
    if path == "discovery_failure_runs_without_parser_bytes_or_permission_invention" {
        return discovery_failure_runs_without_parser_bytes_or_permission_invention();
    }
    if path == "primary_manifest_read_failure_retains_real_metadata_without_empty_project_source" {
        return primary_manifest_read_failure_retains_real_metadata_without_empty_project_source();
    }
    if path == "project_entry_acquisition_deduplicates_provider_candidates_before_observing" {
        return project_entry_acquisition_deduplicates_provider_candidates_before_observing();
    }
    if path == "invalid-provider.rs" {
        return invalid_provider();
    }
    if path == "manifest-and-stray" {
        return manifest_and_stray();
    }
    if path == "digests" {
        return digested();
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

mod a_selected_bucket_under_a_stale_proof_is_refused_and_an_unselected_one_is_not_read_test;
use a_selected_bucket_under_a_stale_proof_is_refused_and_an_unselected_one_is_not_read_test::a_selected_bucket_under_a_stale_proof_is_refused_and_an_unselected_one_is_not_read;
mod syntax_applicability_skips_only_gated_rules_and_names_the_reason_test;
use syntax_applicability_skips_only_gated_rules_and_names_the_reason_test::syntax_applicability_skips_only_gated_rules_and_names_the_reason;
mod only_incomplete_diagnostics_mark_code_relations_as_recovery_test;
use only_incomplete_diagnostics_mark_code_relations_as_recovery_test::only_incomplete_diagnostics_mark_code_relations_as_recovery;
mod required_dependency_stat_and_read_errors_cannot_disappear_test;
use required_dependency_stat_and_read_errors_cannot_disappear_test::required_dependency_stat_and_read_errors_cannot_disappear;
mod unstable_same_shape_resident_populations_remain_distinct_test;
use unstable_same_shape_resident_populations_remain_distinct_test::unstable_same_shape_resident_populations_remain_distinct;
mod file_visits_preserve_stable_unstable_and_unread_provider_outcomes_test;
use file_visits_preserve_stable_unstable_and_unread_provider_outcomes_test::file_visits_preserve_stable_unstable_and_unread_provider_outcomes;
mod a_bad_last_captured_bucket_calls_no_digest_even_after_a_valid_rule_bucket_test;
use a_bad_last_captured_bucket_calls_no_digest_even_after_a_valid_rule_bucket_test::a_bad_last_captured_bucket_calls_no_digest_even_after_a_valid_rule_bucket;
mod successful_outcomes_stage_once_and_keep_digests_without_retaining_payloads_test;
use successful_outcomes_stage_once_and_keep_digests_without_retaining_payloads_test::successful_outcomes_stage_once_and_keep_digests_without_retaining_payloads;
mod staged_replay_reads_only_selected_buckets_after_the_producing_worker_drops_test;
use staged_replay_reads_only_selected_buckets_after_the_producing_worker_drops_test::staged_replay_reads_only_selected_buckets_after_the_producing_worker_drops;
mod read_failure_demand_retries_unchanged_metadata_and_clears_reused_worker_bytes_test;
use read_failure_demand_retries_unchanged_metadata_and_clears_reused_worker_bytes_test::read_failure_demand_retries_unchanged_metadata_and_clears_reused_worker_bytes;
mod read_failure_demand_is_limited_to_governing_rules_test;
use read_failure_demand_is_limited_to_governing_rules_test::read_failure_demand_is_limited_to_governing_rules;
mod discovery_failure_runs_without_parser_bytes_or_permission_invention_test;
use discovery_failure_runs_without_parser_bytes_or_permission_invention_test::discovery_failure_runs_without_parser_bytes_or_permission_invention;
mod primary_manifest_read_failure_retains_real_metadata_without_empty_project_source_test;
use primary_manifest_read_failure_retains_real_metadata_without_empty_project_source_test::primary_manifest_read_failure_retains_real_metadata_without_empty_project_source;
mod project_entry_acquisition_deduplicates_provider_candidates_before_observing_test;
use project_entry_acquisition_deduplicates_provider_candidates_before_observing_test::project_entry_acquisition_deduplicates_provider_candidates_before_observing;

mod visits_test;
use visits_test::{visited, visited_as, visited_as_with_facts, visited_inputs, visited_inputs_with_rule_demands, UnstableResidentFacts};

mod replays_test;
use replays_test::{prepared_test_program, one_capture_plan, replay_under, replay_under_capture};

mod gating_test;
use gating_test::{assert_gated_acceptance, assert_accepted_and_missing_applicability};

mod providers_test;
use providers_test::{invalid_provider, manifest_and_stray, digested};

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



use ryo_values::domain::rows::types::{RowCount, Table};










use ryo_execution::inbound::use_cases::metadata::{
    interface::MetadataUseCase, use_case::MetadataUseCaseImpl,
};

