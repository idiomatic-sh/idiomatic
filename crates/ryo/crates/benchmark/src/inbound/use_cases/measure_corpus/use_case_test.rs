//! A corpus run reports every project's refusal after the others, and publishes only accounted samples.
use super::{
    fixtures::{use_case, with_corpus},
    interface::MeasureCorpusUseCase as _,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage;

/// How many projects the corpus holds, and whether the run is held to the accounted inventory.
type Input = (BenchmarkCoverage, usize);
/// What the observation lines say.
type Value = &'static str;
/// The refusal and the observation lines written before it.
type Failure = (&'static str, &'static str);

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a diagnostic corpus says its samples are not metrics"),
        word: Word::Succeeds,
        input: || (BenchmarkCoverage::Interim, 0),
        expect: Expect::Value("diagnostic-only: scoped samples are not benchmark metrics\n"),
    },
    Case {
        name: CaseName("an accounted corpus without samples refuses publication"),
        word: Word::Refuses,
        input: || (BenchmarkCoverage::Accounted, 0),
        expect: Expect::Error(("no measured evidence", "")),
    },
    Case {
        name: CaseName("a project that cannot be prepared is reported after the diagnostic line"),
        word: Word::Fails,
        input: || (BenchmarkCoverage::Interim, 1),
        expect: Expect::Error((
            "corpus comparisons failed: [ProjectName(\"project-0\")]",
            "project-0: corpus comparison failed: invalid pin\ndiagnostic-only: scoped samples are not benchmark metrics\n",
        )),
    },
];

fn subject((scope, projects): Input) -> Result<Value, Failure> {
    let mut observations = Vec::new();
    let result = with_corpus(scope, projects, |corpus| {
        use_case().measure_corpus(corpus, &mut observations)
    });
    let observations = String::from_utf8_lossy(&observations).into_owned().leak();
    match result {
        Ok(()) => Ok(observations),
        Err(error) => Err((
            &*Box::leak(error.to_string().into_boxed_str()),
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
#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
