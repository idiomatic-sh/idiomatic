//! A corpus run reports every project's refusal after the others, and publishes only accounted samples.
use super::{
    fixtures::{
        bench_of, measuring, project_of, unreliable_host_use_case, use_case, with_certificate,
        with_cohort,
    },
    interface::MeasureCorpusUseCase as _,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::requests::types::CohortSelection;

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
    /// A host whose file permissions are honoured, over one tracked project the fixture contestants hold the certificate of, each
    /// run's resources collected.
    Certified,
}

/// How many projects the corpus holds, whether the run is held to the accounted inventory, its cohort and its host.
type Input = (usize, Cohort, Host);
/// What the observation lines say.
type Value = &'static str;
/// The refusal and the observation lines written before it.
type Failure = (&'static str, &'static str);

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a certified project both contestants agree on publishes its cost"),
        word: Word::Succeeds,
        input: || {
            (
                1,
                Cohort::Explicit {
                    covered: &["A"],
                    selected: &["A"],
                },
                Host::Certified,
            )
        },
        expect: Expect::Value(concat!(
            "project-0: matched Fixture source scope 1 files; excluded default-discovered formats 0 files (source-kind capability gap): \n",
            "project-0: measured source files=1 bytes=2 digest=<sources> input-profile=accepted-fixture\n",
            "project-0: exact matching findings per rule: \n",
            "findings: project=project-0 contestant=Reference condition=empty-tool-cache counts=\n",
            "sample: name=project-0/Reference/empty-tool-cache order=ReferenceFirst wall_seconds=0 cpu_seconds=1 rss_bytes=1048576\n",
            "findings: project=project-0 contestant=ryo condition=empty-tool-cache counts=\n",
            "sample: name=project-0/ryo/empty-tool-cache order=ReferenceFirst wall_seconds=0 cpu_seconds=1 rss_bytes=1048576\n",
            "findings: project=project-0 contestant=Reference condition=retained-tool-cache counts=\n",
            "sample: name=project-0/Reference/retained-tool-cache order=ReferenceFirst wall_seconds=0 cpu_seconds=1 rss_bytes=1048576\n",
            "findings: project=project-0 contestant=ryo condition=retained-tool-cache counts=\n",
            "sample: name=project-0/ryo/retained-tool-cache order=ReferenceFirst wall_seconds=0 cpu_seconds=1 rss_bytes=1048576\n",
            "findings: project=project-0 contestant=Reference condition=edited-tool-cache counts=\n",
            "sample: name=project-0/Reference/edited-tool-cache order=ReferenceFirst wall_seconds=0 cpu_seconds=1 rss_bytes=1048576\n",
            "findings: project=project-0 contestant=ryo condition=edited-tool-cache counts=\n",
            "sample: name=project-0/ryo/edited-tool-cache order=ReferenceFirst wall_seconds=0 cpu_seconds=1 rss_bytes=1048576\n",
            "project-0: exact matching findings per rule: \n",
            "findings: project=project-0 contestant=Reference condition=empty-tool-cache counts=\n",
            "sample: name=project-0/Reference/empty-tool-cache order=CandidateFirst wall_seconds=0 cpu_seconds=1 rss_bytes=1048576\n",
            "findings: project=project-0 contestant=ryo condition=empty-tool-cache counts=\n",
            "sample: name=project-0/ryo/empty-tool-cache order=CandidateFirst wall_seconds=0 cpu_seconds=1 rss_bytes=1048576\n",
            "findings: project=project-0 contestant=Reference condition=retained-tool-cache counts=\n",
            "sample: name=project-0/Reference/retained-tool-cache order=CandidateFirst wall_seconds=0 cpu_seconds=1 rss_bytes=1048576\n",
            "findings: project=project-0 contestant=ryo condition=retained-tool-cache counts=\n",
            "sample: name=project-0/ryo/retained-tool-cache order=CandidateFirst wall_seconds=0 cpu_seconds=1 rss_bytes=1048576\n",
            "findings: project=project-0 contestant=Reference condition=edited-tool-cache counts=\n",
            "sample: name=project-0/Reference/edited-tool-cache order=CandidateFirst wall_seconds=0 cpu_seconds=1 rss_bytes=1048576\n",
            "findings: project=project-0 contestant=ryo condition=edited-tool-cache counts=\n",
            "sample: name=project-0/ryo/edited-tool-cache order=CandidateFirst wall_seconds=0 cpu_seconds=1 rss_bytes=1048576\n",
            "project-0: exact matching findings per rule: \n",
            "findings: project=project-0 contestant=Reference condition=empty-tool-cache counts=\n",
            "sample: name=project-0/Reference/empty-tool-cache order=ReferenceFirst wall_seconds=0 cpu_seconds=1 rss_bytes=1048576\n",
            "findings: project=project-0 contestant=ryo condition=empty-tool-cache counts=\n",
            "sample: name=project-0/ryo/empty-tool-cache order=ReferenceFirst wall_seconds=0 cpu_seconds=1 rss_bytes=1048576\n",
            "findings: project=project-0 contestant=Reference condition=retained-tool-cache counts=\n",
            "sample: name=project-0/Reference/retained-tool-cache order=ReferenceFirst wall_seconds=0 cpu_seconds=1 rss_bytes=1048576\n",
            "findings: project=project-0 contestant=ryo condition=retained-tool-cache counts=\n",
            "sample: name=project-0/ryo/retained-tool-cache order=ReferenceFirst wall_seconds=0 cpu_seconds=1 rss_bytes=1048576\n",
            "findings: project=project-0 contestant=Reference condition=edited-tool-cache counts=\n",
            "sample: name=project-0/Reference/edited-tool-cache order=ReferenceFirst wall_seconds=0 cpu_seconds=1 rss_bytes=1048576\n",
            "findings: project=project-0 contestant=ryo condition=edited-tool-cache counts=\n",
            "sample: name=project-0/ryo/edited-tool-cache order=ReferenceFirst wall_seconds=0 cpu_seconds=1 rss_bytes=1048576\n",
        )),
    },
    Case {
        name: CaseName("a corpus of no project publishes no cost"),
        word: Word::Refuses,
        input: || (0, Cohort::Frozen, Host::Honouring),
        expect: Expect::Error(("no costs", "")),
    },
    Case {
        name: CaseName("a project that cannot be prepared is reported after the diagnostic line"),
        word: Word::Fails,
        input: || (1, Cohort::Frozen, Host::Honouring),
        expect: Expect::Error((
            "corpus comparisons failed: [ProjectName(\"project-0\")]",
            "project-0: corpus comparison failed: invalid pin\n",
        )),
    },
    Case {
        name: CaseName("a cohort outside the certificate refuses before any project runs"),
        word: Word::Refuses,
        input: || {
            (
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
            "project-0: corpus comparison failed: invalid pin\n",
        )),
    },
    Case {
        name: CaseName(
            "a measure whose rules include a profile requiring file observations is refused on an unreliable host",
        ),
        word: Word::Refuses,
        input: || {
            (
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
        input: || (1, Cohort::Frozen, Host::Unreliable),
        expect: Expect::Error((
            "the pinned reference does not evaluate file permissions on this host",
            "",
        )),
    },
];

fn subject((projects, cohort, host): Input) -> Result<Value, Failure> {
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
    let result = match host {
        Host::Certified => certified(projects, covered, &cohort, &mut observations),
        Host::Honouring | Host::Unreliable => with_cohort(projects, covered, &cohort, |corpus| {
            match host {
                Host::Unreliable => unreliable_host_use_case(),
                Host::Honouring | Host::Certified => use_case(),
            }
            .measure_corpus(corpus, &mut observations)
        }),
    };
    let observations = without_digest(&String::from_utf8_lossy(&observations)).leak();
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

/// The corpus of one tracked project the fixture contestants hold the certificate of, which must be what `projects`, `covered`
/// and `cohort` ask for, measured with each run's resources collected.
fn certified(
    projects: usize,
    covered: &[&str],
    cohort: &CohortSelection,
    observations: &mut Vec<u8>,
) -> Result<(), ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
    let files = ["a.fixture"];
    let root = project_of(&files);
    let cache = project_of(&[]);
    let bench = bench_of(covered);
    with_certificate(
        (bench.path(), cache.path()),
        &[super::types::SourceSuffix(".fixture")],
        |corpus| {
            assert_eq!(
                corpus.projects.len(),
                projects,
                "the certified corpus holds one project"
            );
            assert_eq!(
                corpus.cohort, cohort,
                "the certified corpus measures its cohort"
            );
            measuring(root.path(), &files).measure_corpus(corpus, observations)
        },
    )
}

/// `observations` with the digest of the measured sources written as `<sources>`: the digest frames the sources' paths, which
/// lie in a directory made for each run.
fn without_digest(observations: &str) -> String {
    observations
        .split_inclusive('\n')
        .map(|line| match line.split_once(" digest=") {
            Some((before, after)) => format!(
                "{before} digest=<sources>{}",
                after.find(' ').map_or("", |space| &after[space..])
            ),
            None => line.to_owned(),
        })
        .collect()
}
