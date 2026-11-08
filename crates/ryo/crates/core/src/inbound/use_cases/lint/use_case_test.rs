//! What the `lint` use case promises, against the fakes of the ports it calls.
use crate::cases::Matched;
use crate::cases::selects;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::lint::errors::LintError;
use crate::inbound::use_cases::lint::fixtures::given::Given;
use crate::inbound::use_cases::lint::fixtures::lookups::Between;
use crate::inbound::use_cases::lint::fixtures::scenario::Scenario;
use crate::inbound::use_cases::lint::fixtures::scenario::scenario;
use crate::outbound::corpus::fixtures::fake::CorpusFake;
use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::lint_store::fixtures::fake::LintStoreFake;
use crate::outbound::memory::fixtures::fake::MemoryFake;
use crate::outbound::row_sink;
use crate::outbound::scheduler::fixtures::fake::SchedulerFake;
use crate::outbound::scheduler::types::WorkerCount;
use ryo_execution::outbound::digest::fixtures::fake::DigestFake;
use ryo_execution::outbound::facts::fixtures::fake::FactsFake;
use ryo_execution::outbound::matching::fixtures::fake::MatchingFake;
use ryo_execution::outbound::query_language::fixtures::fake::QueryLanguageFake;
use std::sync::Arc;
use super::fixtures::contract::check;
use super::use_case::LintUseCaseImpl;
use crate::inbound::use_cases::lint::fixtures::given::violations;

mod inapplicable_identity;
mod lifecycle;
mod project_budget;
mod refusals;
mod rules;
mod runs;

type Input = Scenario;
type Value = (usize, usize);
type Failure = LintError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("finding_carries_its_line_and_character"),
        word: Word::Succeeds,
        input: || Scenario::Placed,
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("language_project_is_not_called_when_no_rule_reads_what_it_writes"),
        word: Word::Succeeds,
        input: || Scenario::Projects,
        expect: Expect::Value((0, 1)),
    },
    Case {
        name: CaseName("unchanged_files_are_not_revisited"),
        word: Word::Succeeds,
        input: || Scenario::Warm,
        expect: Expect::Value((0, 1)),
    },
    Case {
        name: CaseName("changed_file_is_visited_again"),
        word: Word::Succeeds,
        input: || Scenario::Edited,
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("deleted_file_drops_its_findings"),
        word: Word::Succeeds,
        input: || Scenario::Deleted,
        expect: Expect::Value((0, 0)),
    },
    Case {
        name: CaseName("a_removed_file_leaves_no_unknown_or_skip_behind_in_any_store"),
        word: Word::Succeeds,
        input: || Scenario::RemovedFile,
        expect: Expect::Value((0, 0)),
    },
    Case {
        name: CaseName("changed_rule_is_evaluated_again"),
        word: Word::Succeeds,
        input: || Scenario::ChangedRule,
        expect: Expect::Value((3, 1)),
    },
    Case {
        name: CaseName(
            "a_warm_run_with_nothing_changed_reads_no_project_and_makes_no_project_rule_again",
        ),
        word: Word::Succeeds,
        input: || Scenario::Consulted(Between::Nothing),
        expect: Expect::Value((0, 0)),
    },
    Case {
        name: CaseName("a_changed_configuration_above_the_root_makes_the_project_rules_again"),
        word: Word::Succeeds,
        input: || Scenario::Consulted(Between::ConfigurationAbove),
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("a_changed_manifest_makes_the_project_rules_again"),
        word: Word::Succeeds,
        input: || Scenario::Consulted(Between::Manifest),
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("a_kind_no_linked_language_has_is_refused_before_a_file_is_read"),
        word: Word::Refuses,
        input: || {
            Scenario::Lint(|| Given {
                query: Arc::new(QueryLanguageFake::comparing_a_kind()),
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, LintError::UnknownKind { .. })))),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    scenario(input)
}

#[test]
fn succeeds() {
    check(&{
        let analysis_corpus: Arc<dyn crate::outbound::corpus::interface::CorpusPort> =
            Arc::new(CorpusFake::default());
        let analysis_language: Arc<dyn crate::outbound::language::interface::LanguagePort> =
            Arc::new(LanguageFake::default());
        let analysis_memory: Arc<dyn crate::outbound::memory::interface::MemoryPort> =
            Arc::new(MemoryFake::default());
        let analysis_digest: Arc<
            dyn ryo_execution::outbound::digest::interface::DigestPort,
        > = Arc::new(DigestFake);
        LintUseCaseImpl::new(
            crate::application::file_analysis::service::FileAnalysis::new(
                Arc::clone(&analysis_corpus),
                Arc::clone(&analysis_language),
                Arc::clone(&analysis_digest),
                Arc::clone(&analysis_memory),
            ),
            crate::application::cross_file::service::CrossFile::new(Arc::clone(&analysis_corpus), std::sync::Arc::new(ryo_execution::inbound::use_cases::publication::use_case::PublicationUseCaseImpl::new())),
            Arc::clone(&analysis_corpus),
            Arc::new(FactsFake::default()),
            Arc::clone(&analysis_language),
            Arc::new(MatchingFake),
            Arc::new(QueryLanguageFake::selecting()),
            Arc::clone(&analysis_memory),
            Arc::new(ryo_execution::outbound::operators::factory::use_case(
                ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())),
            )),
            std::sync::Arc::new(ryo_execution_store::outbound::derived::factory::use_case(
                ryo_execution_store::outbound::derived::factory::semantics(),
            )),
            Arc::new(SchedulerFake::default()),
            WorkerCount(1),
            Arc::new(row_sink::factory::collect(row_sink::factory::CollectorConnection::default())),
            Arc::clone(&analysis_digest),
            Arc::new(LintStoreFake::default()),
        )
    });
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
