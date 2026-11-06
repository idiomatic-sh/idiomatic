//! What the schemas of this entry accept and refuse, one row of `CASES` for each input.

use ryo_core::inbound::use_cases::lint::errors::LintError as CoreError;

use super::super::errors::RunError;
use super::RunErrorSchema;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::selects;
use crate::domain::query::types::RyoQlQuery;

/// What a row offers the schemas.
enum Offered {
    /// The query text a run is asked for.
    Query(&'static str),
    /// A failure of the core, offered to the conversion into the caller's.
    CoreFails(CoreError),
}

/// What a row gives the schemas.
type Input = Offered;
/// How long the text the query holds is.
type Value = usize;
/// Why the schemas end in error.
type Failure = RunError;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_query_with_text_is_accepted"),
        word: Word::Succeeds,
        input: || Offered::Query("emit 1;"),
        expect: Expect::Value(7),
    },
    Case {
        name: CaseName("a_blank_query_is_refused"),
        word: Word::Refuses,
        input: || Offered::Query("  "),
        expect: Expect::Refusal(|error| selects!(error, RunError::Query { .. })),
    },
    Case {
        name: CaseName("fails_when_the_corpus_fails"),
        word: Word::Fails,
        input: || {
            Offered::CoreFails(CoreError::Corpus {
                reason: String::new(),
            })
        },
        expect: Expect::Error(RunError::Corpus {
            reason: String::new(),
        }),
    },
    Case {
        name: CaseName("fails_when_the_facts_fail"),
        word: Word::Fails,
        input: || {
            Offered::CoreFails(CoreError::Facts {
                reason: String::new(),
            })
        },
        expect: Expect::Error(RunError::Facts {
            reason: String::new(),
        }),
    },
    Case {
        name: CaseName("fails_when_the_language_fails"),
        word: Word::Fails,
        input: || {
            Offered::CoreFails(CoreError::Language {
                reason: String::new(),
            })
        },
        expect: Expect::Refusal(|error| selects!(error, RunError::Language { .. })),
    },
    Case {
        name: CaseName("fails_when_an_explicit_source_language_is_unavailable"),
        word: Word::Fails,
        input: || {
            Offered::CoreFails(CoreError::SourceLanguage {
                name: "missing".into(),
            })
        },
        expect: Expect::Refusal(
            |error| selects!(error, RunError::Language { reason } if !reason.is_empty()),
        ),
    },
    Case {
        name: CaseName("fails_when_the_memory_fails"),
        word: Word::Fails,
        input: || {
            Offered::CoreFails(CoreError::Memory {
                reason: String::new(),
            })
        },
        expect: Expect::Error(RunError::Memory {
            reason: String::new(),
        }),
    },
    Case {
        name: CaseName("fails_when_the_repository_does_not_fit"),
        word: Word::Fails,
        input: || Offered::CoreFails(CoreError::TooLarge { size: 2, budget: 1 }),
        expect: Expect::Error(RunError::TooLarge { size: 2, budget: 1 }),
    },
    Case {
        name: CaseName("fails_when_the_scheduler_fails"),
        word: Word::Fails,
        input: || {
            Offered::CoreFails(CoreError::Scheduler {
                reason: String::new(),
            })
        },
        expect: Expect::Error(RunError::Scheduler {
            reason: String::new(),
        }),
    },
    Case {
        name: CaseName("fails_when_an_operator_refuses"),
        word: Word::Fails,
        input: || {
            Offered::CoreFails(CoreError::Rule {
                id: "query".to_owned(),
                reason: String::new(),
            })
        },
        expect: Expect::Error(RunError::Evaluate {
            reason: String::new(),
        }),
    },
    Case {
        name: CaseName("fails_when_the_process_holds_more_than_the_budget"),
        word: Word::Fails,
        input: || {
            Offered::CoreFails(CoreError::MemoryBudgetExceeded {
                used: 3,
                budget: 1,
                plans: vec!["query".to_owned()],
            })
        },
        expect: Expect::Error(RunError::MemoryBudgetExceeded { used: 3, budget: 1 }),
    },
    Case {
        name: CaseName("fails_when_the_output_fails"),
        word: Word::Fails,
        input: || {
            Offered::CoreFails(CoreError::Output {
                reason: String::new(),
            })
        },
        expect: Expect::Error(RunError::Output {
            reason: String::new(),
        }),
    },
    Case {
        name: CaseName("fails_when_whoever_reads_the_answer_stops"),
        word: Word::Fails,
        input: || Offered::CoreFails(CoreError::OutputClosed),
        expect: Expect::Error(RunError::OutputClosed),
    },
];

/// Converts the query text through the schemas and answers how long the text the type holds is, or converts the core's failure.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Offered::Query(text) => RyoQlQuery::try_from(text)
            .map(|query| query.to_string().len())
            .map_err(|error| RunError::Query {
                reason: error.to_string(),
            }),
        Offered::CoreFails(error) => Err(RunError::from(RunErrorSchema(error))),
    }
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
