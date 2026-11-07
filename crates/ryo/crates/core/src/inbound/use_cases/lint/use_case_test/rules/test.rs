//! The rows of single lints over the fakes: rules, files and stores, the answer shapes and the refusals.
use crate::cases::Matched;
use crate::cases::selects;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::lint::errors::LintError;
use crate::inbound::use_cases::lint::fixtures::given::Given;
use crate::inbound::use_cases::lint::fixtures::given::SOURCES;
use crate::inbound::use_cases::lint::fixtures::given::host_placed;
use crate::inbound::use_cases::lint::fixtures::given::plan_over_hosted;
use crate::inbound::use_cases::lint::fixtures::scenario::Scenario;
use crate::inbound::use_cases::lint::fixtures::scenario::scenario;
use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::memory::fixtures::fake::MemoryFake;
use crate::outbound::row_sink::fixtures::fake::Ends;
use crate::outbound::scheduler::fixtures::fake::SchedulerFake;
use ryo_execution::outbound::facts::fixtures::fake::FactsFake;
use ryo_execution::outbound::query_language::fixtures::fake::QueryLanguageFake;
use ryo_values::domain::plan_parts::types::ParameterName;
use ryo_values::domain::query_bindings::types::ParameterType;
use ryo_values::domain::query_bindings::types::PlanParameter;
use ryo_values::domain::row_texts::types::RowTotal;
use ryo_values::domain::runs::types::AnswerShape;
use ryo_values::domain::runs::types::Budget;
use std::sync::Arc;
use crate::inbound::use_cases::lint::fixtures::given::violations;

type Input = Scenario;
type Value = (usize, usize);
type Failure = LintError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("violation_is_reported_with_path_and_span"),
        word: Word::Succeeds,
        input: || Scenario::Lint(violations),
        expect: Expect::Value((2, 1)),
    },
    Case {
        name: CaseName("unsupported_file_is_unread_without_a_failure"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lint(|| Given {
                files: vec![SOURCES[2]],
                ..violations()
            })
        },
        expect: Expect::Value((0, 1)),
    },
    Case {
        name: CaseName("lint_is_scoped_to_the_files_given"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lint(|| Given {
                only: vec!["a.rs"],
                ..violations()
            })
        },
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("a_rule_does_not_govern_the_files_it_leaves_out"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lint(|| Given {
                exclude: vec!["**/*.txt"],
                ..violations()
            })
        },
        expect: Expect::Value((2, 0)),
    },
    Case {
        name: CaseName("the_rows_of_a_relation_the_host_declared_are_read_by_a_rule"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lint(|| Given {
                hosted: vec![host_placed()],
                files: vec![SOURCES[0], SOURCES[2]],
                query: Arc::new(QueryLanguageFake {
                    plan: plan_over_hosted(),
                    ..QueryLanguageFake::default()
                }),
                ..violations()
            })
        },
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("rows_of_the_rules_own_columns_are_written_up_to_the_limit"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lint(|| Given {
                query: Arc::new(QueryLanguageFake::selecting_twice()),
                answer: AnswerShape::Rows(RowTotal(1)),
                ..violations()
            })
        },
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("two_rules_answer_every_row_of_each"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lint(|| Given {
                rules: vec!["one", "two"],
                answer: AnswerShape::Rows(RowTotal(20)),
                ..violations()
            })
        },
        expect: Expect::Value((4, 1)),
    },
    Case {
        name: CaseName("a_language_that_writes_a_node_past_its_file_is_refused_naming_it"),
        word: Word::Refuses,
        input: || {
            Scenario::Lint(|| Given {
                language: LanguageFake {
                    out_of_range: true,
                    ..LanguageFake::default()
                },
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, LintError::Provider { .. })))),
    },
    Case {
        name: CaseName("a_source_provider_the_request_names_but_no_language_links_is_refused"),
        word: Word::Refuses,
        input: || {
            Scenario::Lint(|| Given {
                source_language: Some("not-linked"),
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(error, LintError::SourceLanguage { .. })))
        }),
    },
    Case {
        name: CaseName("a_rule_without_an_id_is_refused"),
        word: Word::Refuses,
        input: || {
            Scenario::Lint(|| Given {
                ids: vec![" "],
                ..violations()
            })
        },
        expect: Expect::Error(LintError::BlankRuleId),
    },
    Case {
        name: CaseName("a_parameter_the_rule_gives_nothing_for_is_refused"),
        word: Word::Refuses,
        input: || {
            Scenario::Lint(|| {
                let mut query = QueryLanguageFake::selecting();
                query.plan.parameters = vec![PlanParameter {
                    name: ParameterName("wanted".into()),
                    kind: ParameterType::Text,
                }];
                Given {
                    query: Arc::new(query),
                    ..violations()
                }
            })
        },
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(error, LintError::MissingParameter { .. })))
        }),
    },
    Case {
        name: CaseName("a_repository_larger_than_the_budget_is_refused"),
        word: Word::Refuses,
        input: || {
            Scenario::Lint(|| Given {
                memory: MemoryFake {
                    available: 4,
                    resident: 0,
                    fails: false,
                    asked: Budget::Machine,
                },
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, LintError::TooLarge { .. })))),
    },
    Case {
        name: CaseName("fails_when_the_file_analysis_execution_dependencies_disagree"),
        word: Word::Fails,
        input: || Scenario::FileAnalysisExecutionRefused,
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, LintError::Execution { .. })))),
    },
    Case {
        name: CaseName("fails_when_the_execution_cannot_admit_an_observation"),
        word: Word::Fails,
        input: || Scenario::CrossFileOperatingRefused,
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, LintError::ObservationExecution { .. })))),
    },
    Case {
        name: CaseName("fails_when_the_observation_bindings_form_a_cycle"),
        word: Word::Fails,
        input: || Scenario::ObservationBindingsRefused,
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, LintError::ObservationBindings { .. })))),
    },
    Case {
        name: CaseName("fails_when_facts_fail"),
        word: Word::Fails,
        input: || {
            Scenario::Lint(|| Given {
                facts: FactsFake::failing(),
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, LintError::Facts { .. })))),
    },
    Case {
        name: CaseName("fails_when_the_findings_cannot_be_written"),
        word: Word::Fails,
        input: || {
            Scenario::Lint(|| Given {
                ends: Ends::Failed,
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, LintError::Output { .. })))),
    },
    Case {
        name: CaseName("fails_when_scheduler_fails"),
        word: Word::Fails,
        input: || {
            Scenario::Lint(|| Given {
                scheduler: SchedulerFake { fails: true },
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, LintError::Scheduler { .. })))),
    },
    Case {
        name: CaseName("fails_when_memory_port_fails"),
        word: Word::Fails,
        input: || {
            Scenario::Lint(|| Given {
                memory: MemoryFake {
                    fails: true,
                    ..MemoryFake::default()
                },
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, LintError::Memory { .. })))),
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
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
