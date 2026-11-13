//! A Python benchmark admits a Python corpus of two or three projects and refuses any other in its own words.
use super::super::LanguageTitle;
use super::value::CorpusShape;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::{Project, Suite};

/// The corpus's language and project count.
type Input = (&'static str, usize);
/// What the shape answered.
type Value = &'static str;
/// The refusal's message.
type Failure = &'static str;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a corpus of two projects of the language is admitted"),
        word: Word::Succeeds,
        input: || ("python", 2),
        expect: Expect::Value("admitted"),
    },
    Case {
        name: CaseName("a corpus of three projects of the language is admitted"),
        word: Word::Succeeds,
        input: || ("python", 3),
        expect: Expect::Value("admitted"),
    },
    Case {
        name: CaseName("a corpus of four projects is refused"),
        word: Word::Refuses,
        input: || ("python", 4),
        expect: Expect::Error("Python suite requires 2–3 projects"),
    },
    Case {
        name: CaseName("another language's corpus is refused"),
        word: Word::Refuses,
        input: || ("typescript", 2),
        expect: Expect::Error("Python suite requires 2–3 projects"),
    },
];

fn subject((language, count): Input) -> Result<Value, Failure> {
    let python = "python".into();
    let suite = Suite {
        language: language.into(),
        projects: (0..count)
            .map(|index| Project {
                name: format!("project-{index}").into(),
                repository: "https://example.invalid/project.git".into(),
                revision: "0000000000000000000000000000000000000000".into(),
                patch: "project.patch".into(),
            })
            .collect(),
    };
    CorpusShape {
        language: &python,
        display: LanguageTitle("Python"),
    }
    .admit(&suite)
    .map(|()| "admitted")
    .map_err(|error| match error {
        Error::Invalid(message) => &*Box::leak(message.into_boxed_str()),
        _ => "unexpected",
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
