//! What the `compare_corpus` use case promises, against the fakes of the ports it calls, from its fixtures.
use super::fixtures::{Scenario, configured, corpus, use_case};
use super::interface::CompareCorpusUseCase as _;
use super::types::CorpusParity;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::reports::types::ContestantName;

type Input = Scenario;
/// What the run wrote.
type Value = &'static str;
/// The refusal, the corpus digest spelled `<digest>`, and what the run wrote before it.
type Failure = (&'static str, &'static str);

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a difference the accepted differences name passes"),
        word: Word::Succeeds,
        input: || Scenario {
            configured: true,
            this_corpus: true,
            accepted_column: 3,
        },
        expect: Expect::Value("fixture parity: 0 identical of 1 rules; 0 unaccepted\n"),
    },
    Case {
        name: CaseName("a difference at another place fails after reporting each problem"),
        word: Word::Refuses,
        input: || Scenario {
            configured: true,
            this_corpus: true,
            accepted_column: 1,
        },
        expect: Expect::Error((
            "2 unaccepted parity differences",
            "fixture parity: 0 identical of 1 rules; 2 unaccepted\nA: extra case.py:1:3 not accepted\nA: accepted extra case.py:1:1 no longer occurs\n",
        )),
    },
    Case {
        name: CaseName(
            "a request selecting other than the configured rules is refused before either contestant runs",
        ),
        word: Word::Refuses,
        input: || Scenario {
            configured: false,
            this_corpus: true,
            accepted_column: 3,
        },
        expect: Expect::Error((
            "parity requires the request to select exactly the configured rules: 1 configured, 1 selected; unselected [RuleId(\"A\")], unconfigured [RuleId(\"B\")]",
            "",
        )),
    },
    Case {
        name: CaseName(
            "accepted differences written for another corpus are refused before either contestant runs",
        ),
        word: Word::Refuses,
        input: || Scenario {
            configured: true,
            this_corpus: false,
            accepted_column: 3,
        },
        expect: Expect::Error((
            "accepted differences are for corpus 0000000000000000000000000000000000000000000000000000000000000000, the request reads corpus <digest>",
            "",
        )),
    },
];

/// Compares the fixture corpus under one scenario.
fn subject(input: Input) -> Result<Value, Failure> {
    let corpus = corpus(input);
    let mut observations = Vec::new();
    let result = use_case().compare_corpus(
        &CorpusParity {
            request: &corpus.request,
            configured: &configured(),
            accepted: &corpus.accepted,
            reference: ContestantName("fixture"),
        },
        &mut observations,
    );
    let observations: &'static str = String::from_utf8_lossy(&observations).into_owned().leak();
    match result {
        Ok(()) => Ok(observations),
        Err(error) => Err((
            error
                .to_string()
                .replace(&corpus.digest.0, "<digest>")
                .leak(),
            observations,
        )),
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
