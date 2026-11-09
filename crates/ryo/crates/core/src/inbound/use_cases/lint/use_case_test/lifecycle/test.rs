//! The rows of runs over one store: warm, edited, deleted, changed and failed runs, lookups and kept answers.
use crate::cases::Matched;
use crate::cases::selects;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::lint::errors::LintError;
use crate::inbound::use_cases::lint::fixtures::lookups::Between;
use crate::inbound::use_cases::lint::fixtures::scenario::Scenario;
use crate::inbound::use_cases::lint::fixtures::scenario::scenario;
use crate::outbound::lint_store::fixtures::fake::LintStoreFake;

type Input = Scenario;
type Value = (usize, usize);
type Failure = LintError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_configuration_the_lookup_read_that_is_gone_makes_the_project_rules_again",
        ),
        word: Word::Succeeds,
        input: || Scenario::Consulted(Between::ConfigurationGone),
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName(
            "a_configuration_the_lookup_found_none_of_and_is_there_now_makes_the_project_rules_again",
        ),
        word: Word::Succeeds,
        input: || Scenario::Consulted(Between::ConfigurationAppears),
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName(
            "a_newer_configuration_before_the_older_one_the_lookup_found_makes_the_project_rules_again",
        ),
        word: Word::Succeeds,
        input: || Scenario::Consulted(Between::OlderBecomesNewer),
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("compiles_each_rule_once_per_session"),
        word: Word::Succeeds,
        input: || Scenario::CompilesPerSession,
        expect: Expect::Value((2, 0)),
    },
    Case {
        name: CaseName("unchanged_reuse_reads_no_file"),
        word: Word::Succeeds,
        input: || Scenario::Unchanged,
        expect: Expect::Value((0, 0)),
    },
    Case {
        name: CaseName("a_body_edit_reads_only_that_file_and_keeps_the_cross_file_findings"),
        word: Word::Succeeds,
        input: || Scenario::BodyEdited,
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName(
            "renaming_an_export_reads_that_file_alone_and_makes_the_cross_file_findings_again",
        ),
        word: Word::Succeeds,
        input: || Scenario::ExportRenamed,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("a_failed_run_keeps_the_previous_state"),
        word: Word::Succeeds,
        input: || Scenario::FailedRun,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("a_file_whose_bytes_could_not_be_read_is_read_again_by_the_next_run"),
        word: Word::Succeeds,
        input: || Scenario::UnreadableAgain,
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("a_changed_query_recompiles_once_and_then_reuses_the_plan"),
        word: Word::Succeeds,
        input: || Scenario::ChangedQueryKeepsOnePlan,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("a_changed_fragment_replaces_the_retained_compiled_plan"),
        word: Word::Succeeds,
        input: || Scenario::ChangedFragmentPlan,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("editing_a_nested_query_module_invalidates_plan_and_persisted_results"),
        word: Word::Succeeds,
        input: || Scenario::ChangedNestedModule,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName(
            "an_edited_file_read_alone_names_its_own_path_and_never_the_excluded_file_listed_before_it",
        ),
        word: Word::Succeeds,
        input: || Scenario::EditedBesideAnExcludedFile,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("a_reused_file_read_in_part_keeps_its_part_when_a_file_is_listed_before_it"),
        word: Word::Succeeds,
        input: || Scenario::PartialOfAReusedFileAfterAFileIsListedBeforeIt,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("interleaved_runs_naming_different_rules_compile_each_rule_once"),
        word: Word::Succeeds,
        input: || Scenario::InterleavedRuns,
        expect: Expect::Value((2, 0)),
    },
    Case {
        name: CaseName("many_runs_in_a_row_leave_a_bounded_store"),
        word: Word::Succeeds,
        input: || Scenario::ManyRuns,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("compiles_each_rule_once"),
        word: Word::Succeeds,
        input: || Scenario::CompilesPerRule,
        expect: Expect::Value((2, 1)),
    },
    Case {
        name: CaseName("fails_when_store_port_cannot_keep"),
        word: Word::Fails,
        input: || {
            Scenario::BrokenStore(|| LintStoreFake {
                unwritable: true,
                ..LintStoreFake::default()
            })
        },
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, LintError::Store { .. })))),
    },
    Case {
        name: CaseName("fails_when_store_port_cannot_read"),
        word: Word::Fails,
        input: || {
            Scenario::BrokenStore(|| LintStoreFake {
                unreadable: true,
                ..LintStoreFake::default()
            })
        },
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, LintError::Store { .. })))),
    },
    Case {
        name: CaseName(
            "accepted_syntax_applicability_replays_skips_and_invalidates_on_policy_change",
        ),
        word: Word::Succeeds,
        input: || Scenario::AcceptedSyntaxApplicabilityReplaysSkipsAndInvalidatesOnPolicyChange,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("project_entry_observations_invalidate_persisted_absence_and_presence"),
        word: Word::Succeeds,
        input: || Scenario::ProjectEntryObservationsInvalidatePersistedAbsenceAndPresence,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("project_reuse_frames_root_and_repository_and_keeps_dependency_evidence"),
        word: Word::Succeeds,
        input: || Scenario::ProjectReuseFramesRootAndRepositoryAndKeepsDependencyEvidence,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("consulted_equal_byte_files_revalidate_on_each_executable_metadata_change"),
        word: Word::Succeeds,
        input: || Scenario::ConsultedEqualByteFilesRevalidateOnEachExecutableMetadataChange,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("cold_warm_and_edited_runs_answer_the_same_in_every_store"),
        word: Word::Succeeds,
        input: || Scenario::ColdWarmAndEditedRunsAnswerTheSameInEveryStore,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("a_syntax_skipped_file_does_not_poison_unrelated_partitions_cold_or_warm"),
        word: Word::Succeeds,
        input: || Scenario::ASyntaxSkippedFileDoesNotPoisonUnrelatedPartitionsColdOrWarm,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName(
            "a_syntax_rejected_file_is_skipped_with_its_reason_and_never_undecided_or_complete",
        ),
        word: Word::Succeeds,
        input: || Scenario::ASyntaxRejectedFileIsSkippedWithItsReasonAndNeverUndecidedOrComplete,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("an_undetermined_file_answers_unknown_and_the_lint_finishes"),
        word: Word::Succeeds,
        input: || Scenario::AnUndeterminedFileAnswersUnknownAndTheLintFinishes,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("accepted_files_answer_as_a_rule_that_always_applies"),
        word: Word::Succeeds,
        input: || Scenario::AcceptedFilesAnswerAsARuleThatAlwaysApplies,
        expect: Expect::Value((1, 0)),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    scenario(input)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
