//! What a row of the `lint` cases asks of the use case, and the run that answers it.
use super::super::errors::LintError;
use super::super::interface::LintUseCase as _;
use super::given::Given;
use super::given::Lint;
use super::given::SOURCES;
use super::given::request;
use super::given::violations;
use super::lifecycle::crossed_after;
use super::lifecycle::deleted;
use super::lifecycle::edited;
use super::lifecycle::failed_run;
use super::lifecycle::many_runs;
use super::lifecycle::placed;
use super::lifecycle::unchanged;
use super::lookups::Between;
use super::lookups::changed_fragment_plan;
use super::lookups::changed_nested_module;
use super::lookups::compiles_per_session;
use super::lookups::consulted;
use super::lookups::edited_beside_an_excluded_file;
use super::lookups::interleaved_runs;
use super::lookups::unreadable_again;
use super::skips::a_syntax_rejected_file_is_skipped_with_its_reason_and_never_undecided_or_complete;
use super::skips::a_syntax_skipped_file_does_not_poison_unrelated_partitions_cold_or_warm;
use super::skips::accepted_files_answer_as_a_rule_that_always_applies;
use super::skips::an_undetermined_file_answers_unknown_and_the_lint_finishes;
use super::skips::cold_warm_and_edited_runs_answer_the_same_in_every_store;
use super::skips::consulted_equal_byte_files_revalidate_on_each_executable_metadata_change;
use super::skips::removed_file_leaves_nothing;
use super::stores::accepted_syntax_applicability_replays_skips_and_invalidates_on_policy_change;
use super::stores::project_entry_observations_invalidate_persisted_absence_and_presence;
use super::stores::project_reuse_frames_root_and_repository_and_keeps_dependency_evidence;
use super::worlds::again;
use super::worlds::lint;
use super::worlds::read_and_unread;
use super::worlds::serial_and_parallel_agree;
use super::worlds::world;
use crate::outbound::lint_store::fixtures::fake::LintStoreFake;
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// What a row of the cases asks of the use case.
#[derive(Clone, Copy)]
pub(crate) enum Scenario {
    /// One lint over the fakes the function builds.
    Lint(fn() -> Given),
    /// Lints the same files twice over one store.
    Warm,
    /// Lints the files again after one of them was edited.
    Edited,
    /// Lints the files again after two of them were deleted.
    Deleted,
    /// Lints a file that records a skip or an unknown, then the run without it, in the memory and the postcard store.
    RemovedFile,
    /// Lints the files again with another rule.
    ChangedRule,
    /// Lints with a store that cannot be written or read.
    BrokenStore(fn() -> LintStoreFake),
    /// Lints twice with two rules, with one use case.
    CompilesPerSession,
    /// Lints once with two rules.
    CompilesPerRule,
    /// Lints many times in a row over one store, a file edited back and forth: what the store keeps stops growing.
    ManyRuns,
    /// Lints a rule, then the rule of the same id with another query, over plans one session shares.
    ChangedQueryKeepsOnePlan,
    /// Changes a directly imported module with the real compiler and checks changed findings.
    ChangedFragmentPlan,
    /// Changes only a nested query module and checks both plan and persisted-result invalidation.
    ChangedNestedModule,
    /// Edits a source listed after a file the rules' scope excludes: the edit run answers what a cold run answers, and no row names the
    /// excluded file.
    EditedBesideAnExcludedFile,
    /// Two runs naming different rules, one after the other twice over plans one session shares, as concurrent `tm validate` calls do.
    InterleavedRuns,
    /// Lints with the host's relation, counting how many times the language was asked to read the manifests.
    Projects,
    /// Lints and reads the line and the character the first finding is at.
    Placed,
    /// The pipeline's lifecycle: a lint, then another over the same files and store.
    Unchanged,
    /// The pipeline's lifecycle: a lint with a rule that crosses files, then one after a file's body is edited and what it hands
    /// over is as it was.
    BodyEdited,
    /// The same, after the function a file declares is renamed: the changed file is read on its own, and the cross-file steps are
    /// made again from what the other files handed the last run.
    ExportRenamed,
    /// The pipeline's lifecycle: a lint, a lint that fails while it writes its findings, then a lint over the same files.
    FailedRun,
    /// Lints twice a file whose bytes cannot be read alike twice: the second lint reads it again.
    UnreadableAgain,
    /// Lints a rule that reads the project twice over one store, with what changed between the two.
    Consulted(Between),
    /// The file analysis refuses a run for execution dependencies that disagree.
    FileAnalysisExecutionRefused,
    /// The cross-file publication refuses an observation the execution cannot admit.
    CrossFileOperatingRefused,
    /// Rule compilation refuses named-root bindings that form a cycle.
    ObservationBindingsRefused,
    /// accepted syntax applicability replays skips and invalidates on policy change.
    AcceptedSyntaxApplicabilityReplaysSkipsAndInvalidatesOnPolicyChange,
    /// project entry observations invalidate persisted absence and presence.
    ProjectEntryObservationsInvalidatePersistedAbsenceAndPresence,
    /// project reuse frames root and repository and keeps dependency evidence.
    ProjectReuseFramesRootAndRepositoryAndKeepsDependencyEvidence,
    /// consulted equal byte files revalidate on each executable metadata change.
    ConsultedEqualByteFilesRevalidateOnEachExecutableMetadataChange,
    /// cold warm and edited runs answer the same in every store.
    ColdWarmAndEditedRunsAnswerTheSameInEveryStore,
    /// a syntax skipped file does not poison unrelated partitions cold or warm.
    ASyntaxSkippedFileDoesNotPoisonUnrelatedPartitionsColdOrWarm,
    /// a syntax rejected file is skipped with its reason and never undecided or complete.
    ASyntaxRejectedFileIsSkippedWithItsReasonAndNeverUndecidedOrComplete,
    /// an undetermined file answers unknown and the lint finishes.
    AnUndeterminedFileAnswersUnknownAndTheLintFinishes,
    /// accepted files answer as a rule that always applies.
    AcceptedFilesAnswerAsARuleThatAlwaysApplies,
}

pub(crate) fn scenario(scenario: Scenario) -> Result<(usize, usize), LintError> {
    let fresh = || Arc::new(LintStoreFake::default());
    match scenario {
        Scenario::AcceptedSyntaxApplicabilityReplaysSkipsAndInvalidatesOnPolicyChange => accepted_syntax_applicability_replays_skips_and_invalidates_on_policy_change(),
        Scenario::ProjectEntryObservationsInvalidatePersistedAbsenceAndPresence => project_entry_observations_invalidate_persisted_absence_and_presence(),
        Scenario::ProjectReuseFramesRootAndRepositoryAndKeepsDependencyEvidence => project_reuse_frames_root_and_repository_and_keeps_dependency_evidence(),
        Scenario::ConsultedEqualByteFilesRevalidateOnEachExecutableMetadataChange => consulted_equal_byte_files_revalidate_on_each_executable_metadata_change(),
        Scenario::ColdWarmAndEditedRunsAnswerTheSameInEveryStore => cold_warm_and_edited_runs_answer_the_same_in_every_store(),
        Scenario::ASyntaxSkippedFileDoesNotPoisonUnrelatedPartitionsColdOrWarm => a_syntax_skipped_file_does_not_poison_unrelated_partitions_cold_or_warm(),
        Scenario::ASyntaxRejectedFileIsSkippedWithItsReasonAndNeverUndecidedOrComplete => a_syntax_rejected_file_is_skipped_with_its_reason_and_never_undecided_or_complete(),
        Scenario::AnUndeterminedFileAnswersUnknownAndTheLintFinishes => an_undetermined_file_answers_unknown_and_the_lint_finishes(),
        Scenario::AcceptedFilesAnswerAsARuleThatAlwaysApplies => accepted_files_answer_as_a_rule_that_always_applies(),
        Scenario::FileAnalysisExecutionRefused => Err(LintError::from(
            crate::application::file_analysis::errors::FileAnalysisError::Execution(
                ryo_execution::inbound::use_cases::execution::interface::OperatingError::SemanticBatchShape,
            ),
        )),
        Scenario::CrossFileOperatingRefused => Err(LintError::from(
            crate::application::cross_file::errors::CrossFileError::Operating(
                ryo_execution::inbound::use_cases::execution::interface::OperatingError::SemanticBatchShape,
            ),
        )),
        Scenario::ObservationBindingsRefused => Err(LintError::from(
            ryo_execution::inbound::use_cases::compile_rules::interface::RuleCompilationError::Observations(
                ryo_execution::inbound::use_cases::execution::interface::RuleIntentError::Cycle,
            ),
        )),
        Scenario::Lint(given) => {
            let made = lint(given());
            let findings = made.rows.len();
            made.report.map(|unread| (findings, unread))
        }
        Scenario::Placed => placed(),
        Scenario::Projects => {
            let made = lint(violations());
            made.report.map(|unread| (made.projects, unread))
        }
        Scenario::Warm => {
            serial_and_parallel_agree();
            let store = fresh();
            let cold = again(&SOURCES, "root", "a rule", &store);
            let warm = again(&SOURCES, "root", "a rule", &store);
            assert_eq!(cold.parses, 3, "a cold run reads every file");
            assert_eq!(warm.rows, cold.rows, "and answers the same findings");
            assert_eq!(&warm.report, &cold.report, "and the same unread files");
            read_and_unread(warm)
        }
        Scenario::Edited => edited(),
        Scenario::Unchanged => Ok(unchanged()),
        Scenario::BodyEdited => crossed_after(("a.rs", "fn a(){ }")),
        Scenario::ExportRenamed => crossed_after(("a.rs", "fn z() {}")),
        Scenario::FailedRun => Ok(failed_run()),
        Scenario::UnreadableAgain => unreadable_again(),
        Scenario::Consulted(between) => consulted(between),
        Scenario::Deleted => deleted(),
        Scenario::RemovedFile => removed_file_leaves_nothing(),
        Scenario::ChangedRule => {
            let store = fresh();
            let cold = again(&SOURCES, "root", "a rule", &store);
            assert_eq!(cold.parses, 3, "a cold run reads every file");
            read_and_unread(again(&SOURCES, "root", "another rule", &store))
        }
        Scenario::BrokenStore(store) => {
            read_and_unread(again(&SOURCES, "root", "a rule", &Arc::new(store())))
        }
        Scenario::CompilesPerSession => compiles_per_session(),
        Scenario::ChangedQueryKeepsOnePlan => {
            let rule = |query| Given {
                files: vec![SOURCES[0]],
                rules: vec![query],
                ..violations()
            };
            let (first, second) = (request(&rule("one")), request(&rule("two")));
            let world = world(rule("one"));
            let _first = world.use_case.lint(first)?;
            let _second = world.use_case.lint(second.clone())?;
            let _cached_second = world.use_case.lint(second)?;
            Ok((
                usize::from(world.query.calls.load(Ordering::SeqCst) == 2),
                0,
            ))
        }
        Scenario::ChangedFragmentPlan => changed_fragment_plan(),
        Scenario::ChangedNestedModule => changed_nested_module(),
        Scenario::EditedBesideAnExcludedFile => edited_beside_an_excluded_file(),
        Scenario::InterleavedRuns => interleaved_runs(),
        Scenario::ManyRuns => many_runs(),
        Scenario::CompilesPerRule => {
            let made = lint(Given {
                rules: vec!["one", "two"],
                ..violations()
            });
            made.report.map(|unread| (made.compiles, unread))
        }
    }
}
