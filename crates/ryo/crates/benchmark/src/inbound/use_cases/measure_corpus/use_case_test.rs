//! A corpus run reports every project's refusal after the others, and publishes only accounted samples.
use super::{
    fixtures::{unreliable_host_use_case, use_case, with_cohort},
    interface::MeasureCorpusUseCase as _,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::requests::types::{BenchmarkCoverage, CohortSelection};

/// The rules a certificate covers and the cohort measured under it.
#[derive(Clone, Copy)]
enum Cohort {
    /// A certificate of rule A measures the frozen profile selection.
    Frozen,
    /// A certificate of the `covered` rules measures exactly the `selected` ones.
    Explicit {
        covered: &'static [&'static str],
        selected: &'static [&'static str],
    },
}

/// The host the corpus is measured on.
#[derive(Clone, Copy)]
enum Host {
    /// A host whose file permissions are honoured, over a suite whose profile requires none.
    Honouring,
    /// A Microsoft kernel outside a container, over a suite whose profile requires executable bits.
    Unreliable,
}

/// How many projects the corpus holds, whether the run is held to the accounted inventory, its cohort and its host.
type Input = (BenchmarkCoverage, usize, Cohort, Host);
/// What the observation lines say.
type Value = &'static str;
/// The refusal and the observation lines written before it.
type Failure = (&'static str, &'static str);

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a diagnostic corpus says its samples are not metrics"),
        word: Word::Succeeds,
        input: || {
            (
                BenchmarkCoverage::Interim,
                0,
                Cohort::Frozen,
                Host::Honouring,
            )
        },
        expect: Expect::Value("diagnostic-only: scoped samples are not benchmark metrics\n"),
    },
    Case {
        name: CaseName("an accounted corpus without samples refuses publication"),
        word: Word::Refuses,
        input: || {
            (
                BenchmarkCoverage::Accounted,
                0,
                Cohort::Frozen,
                Host::Honouring,
            )
        },
        expect: Expect::Error(("no measured evidence", "")),
    },
    Case {
        name: CaseName("a project that cannot be prepared is reported after the diagnostic line"),
        word: Word::Fails,
        input: || {
            (
                BenchmarkCoverage::Interim,
                1,
                Cohort::Frozen,
                Host::Honouring,
            )
        },
        expect: Expect::Error((
            "corpus comparisons failed: [ProjectName(\"project-0\")]",
            "project-0: corpus comparison failed: invalid pin\ndiagnostic-only: scoped samples are not benchmark metrics\n",
        )),
    },
    Case {
        name: CaseName("a cohort outside the certificate refuses before any project runs"),
        word: Word::Refuses,
        input: || {
            (
                BenchmarkCoverage::Interim,
                1,
                Cohort::Explicit {
                    covered: &["A"],
                    selected: &["B"],
                },
                Host::Honouring,
            )
        },
        expect: Expect::Error((super::constants::COHORT_OUTSIDE_CERTIFICATE, "")),
    },
    Case {
        name: CaseName("a cohort inside the certificate goes on to its projects"),
        word: Word::Fails,
        input: || {
            (
                BenchmarkCoverage::Interim,
                1,
                Cohort::Explicit {
                    covered: &["A", "B"],
                    selected: &["A"],
                },
                Host::Honouring,
            )
        },
        expect: Expect::Error((
            "corpus comparisons failed: [ProjectName(\"project-0\")]",
            "project-0: corpus comparison failed: invalid pin\ndiagnostic-only: scoped samples are not benchmark metrics\n",
        )),
    },
    Case {
        name: CaseName(
            "a measure whose rules include a profile requiring file observations is refused on an unreliable host",
        ),
        word: Word::Refuses,
        input: || {
            (
                BenchmarkCoverage::Interim,
                1,
                Cohort::Explicit {
                    covered: &["A", "B"],
                    selected: &["A"],
                },
                Host::Unreliable,
            )
        },
        expect: Expect::Error((
            "the pinned reference does not evaluate file permissions on this host",
            "",
        )),
    },
    Case {
        name: CaseName("a frozen cohort is judged by the certificate's authored rules"),
        word: Word::Refuses,
        input: || {
            (
                BenchmarkCoverage::Interim,
                1,
                Cohort::Frozen,
                Host::Unreliable,
            )
        },
        expect: Expect::Error((
            "the pinned reference does not evaluate file permissions on this host",
            "",
        )),
    },
];

fn subject((scope, projects, cohort, host): Input) -> Result<Value, Failure> {
    let mut observations = Vec::new();
    let (covered, cohort) = match cohort {
        Cohort::Frozen => (&["A"][..], CohortSelection::Frozen),
        Cohort::Explicit { covered, selected } => (
            covered,
            CohortSelection::Explicit(
                selected
                    .iter()
                    .map(|rule| ryo::linter::RuleId::try_from(*rule).unwrap())
                    .collect(),
            ),
        ),
    };
    let result = with_cohort(scope, projects, covered, &cohort, |corpus| {
        match host {
            Host::Honouring => use_case(),
            Host::Unreliable => unreliable_host_use_case(),
        }
        .measure_corpus(corpus, &mut observations)
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
