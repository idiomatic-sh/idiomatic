//! More rows of single lints over the fakes: the hosted relations, the selected provider and the failing ports.
use crate::cases::Matched;
use crate::cases::selects;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::lint::errors::LintError;
use crate::inbound::use_cases::lint::fixtures::given::Given;
use crate::inbound::use_cases::lint::fixtures::given::SOURCES;
use crate::inbound::use_cases::lint::fixtures::given::plan_with_name;
use crate::inbound::use_cases::lint::fixtures::given::violations;
use crate::inbound::use_cases::lint::fixtures::scenario::Scenario;
use crate::inbound::use_cases::lint::fixtures::scenario::scenario;
use crate::outbound::language::fixtures::fake::FakeCatalog;
use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::memory::fixtures::fake::MemoryFake;
use crate::outbound::row_sink::fixtures::fake::Ends;
use ryo_execution::outbound::query_language::fixtures::fake::QueryLanguageFake;
use ryo_execution::outbound::query_language::fixtures::fake::REFUSED;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::row_texts::types::RowTotal;
use ryo_values::domain::runs::types::AnswerShape;
use ryo_values::domain::runs::types::Budget;
use std::sync::Arc;

type Input = Scenario;
type Value = (usize, usize);
type Failure = LintError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("unread_files_are_reported"),
        word: Word::Succeeds,
        input: || Scenario::Lint(violations),
        expect: Expect::Value((2, 1)),
    },
    Case {
        name: CaseName("clean_repository_has_no_findings"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lint(|| Given {
                query: Arc::new(QueryLanguageFake {
                    plan: plan_with_name("nothing"),
                    ..QueryLanguageFake::default()
                }),
                files: SOURCES[..2].to_vec(),
                ..violations()
            })
        },
        expect: Expect::Value((0, 0)),
    },
    Case {
        name: CaseName("a_rule_governs_only_the_files_it_includes_and_the_others_are_not_read"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lint(|| Given {
                include: vec!["a.rs"],
                ..violations()
            })
        },
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("language_parse_failure_is_unread_not_empty"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lint(|| Given {
                files: vec![SOURCES[0], ("broken.rs", "fn main() {}")],
                ..violations()
            })
        },
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("a_file_that_changes_while_it_is_read_is_left_unread_and_not_kept"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lint(|| Given {
                files: vec![SOURCES[0], ("unstable.rs", "fn main() {}")],
                ..violations()
            })
        },
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("rows_of_the_cross_file_steps_are_answered_as_their_own_columns"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lint(|| Given {
                query: Arc::new(QueryLanguageFake::counting_across_files()),
                answer: AnswerShape::Rows(RowTotal(20)),
                ..violations()
            })
        },
        expect: Expect::Value((2, 1)),
    },
    Case {
        name: CaseName("an_unknown_semantic_kind_is_refused_before_a_file_is_read"),
        word: Word::Refuses,
        input: || {
            Scenario::Lint(|| {
                let mut query = QueryLanguageFake::comparing_a_kind();
                for step in &mut query.plan.steps {
                    if let Operation::Select { from, .. } = &mut step.operation {
                        *from = PlanInput::Relation(Relation::SemanticSyntax);
                    }
                }
                Given {
                    query: Arc::new(query),
                    ..violations()
                }
            })
        },
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(error, LintError::UnknownKind { .. })))
        }),
    },
    Case {
        name: CaseName("a_run_that_holds_more_than_its_budget_is_refused_naming_the_rules"),
        word: Word::Refuses,
        input: || {
            Scenario::Lint(|| Given {
                memory: MemoryFake {
                    available: 1 << 30,
                    resident: 2 << 30,
                    fails: false,
                    asked: Budget::Machine,
                },
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(
                error,
                LintError::MemoryBudgetExceeded { .. }
            )))
        }),
    },
    Case {
        name: CaseName("a_glob_that_is_not_one_is_refused"),
        word: Word::Refuses,
        input: || {
            Scenario::Lint(|| Given {
                include: vec!["["],
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, LintError::Rule { .. })))),
    },
    Case {
        name: CaseName("two_rules_with_one_id_are_refused"),
        word: Word::Refuses,
        input: || {
            Scenario::Lint(|| Given {
                rules: vec!["one", "two"],
                ids: vec!["same", "same"],
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(error, LintError::DuplicateRuleId { .. })))
        }),
    },
    Case {
        name: CaseName("rule_that_does_not_validate_is_refused"),
        word: Word::Refuses,
        input: || {
            Scenario::Lint(|| Given {
                rules: vec![REFUSED],
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, LintError::Query { .. })))),
    },
    Case {
        name: CaseName("fails_when_corpus_fails"),
        word: Word::Fails,
        input: || {
            Scenario::Lint(|| Given {
                corpus_fails: true,
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(error, LintError::Corpus { .. })))
        }),
    },
    Case {
        name: CaseName("fails_when_an_operator_refuses"),
        word: Word::Fails,
        input: || {
            Scenario::Lint(|| Given {
                files: vec![SOURCES[0]],
                query: Arc::new(QueryLanguageFake::comparing_a_missing_column()),
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| selects(Matched(matches!(error, LintError::Rule { .. })))),
    },
    Case {
        name: CaseName("fails_when_the_reader_stopped"),
        word: Word::Fails,
        input: || {
            Scenario::Lint(|| Given {
                ends: Ends::Closed,
                ..violations()
            })
        },
        expect: Expect::Error(LintError::OutputClosed),
    },
    Case {
        name: CaseName("fails_when_language_port_fails"),
        word: Word::Fails,
        input: || {
            Scenario::Lint(|| Given {
                language: LanguageFake {
                    catalog: FakeCatalog::Fails,
                    ..LanguageFake::default()
                },
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(error, LintError::Language { .. })))
        }),
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
