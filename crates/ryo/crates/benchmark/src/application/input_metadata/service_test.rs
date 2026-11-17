//! A profile that requires file observations refuses a case that omits them, and a host whose file permissions cannot be
//! trusted, before anything executes; a corpus run is judged by the settings profile of every rule it selects.
use super::constants;
use super::service::{admit, admit_rules};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::discrepancy::fixtures::{case, profile};
use crate::outbound::host::fixtures::fake::HostFake;
use crate::outbound::suite::fixtures::fake::SuiteFake;
use ryo_benchmark_contracts::domain::artifacts::types::FileExecutable;
use ryo_benchmark_contracts::domain::hosts::types::{
    HostFacts, HostFamily, HostIsolation, HostKernel,
};
use ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration;
use ryo_benchmark_contracts::domain::workloads::types::{FileObservation, PreparedCase};
use std::path::Path;

/// What the profile requires and what the case declares.
#[derive(Clone, Copy)]
enum Scenario {
    /// The profile requires nothing.
    NothingRequired,
    /// The profile requires observations and the case declares its file's.
    Declared,
    /// The profile requires observations and the case omits them.
    Omitted,
    /// The rules directory has no parent that could hold a profile.
    NoProfileBoundary,
    /// The profile requires observations, the case declares them, and the host is a Microsoft kernel outside a container.
    RequiredOnUnreliableHost,
    /// The profile requires nothing, on a Microsoft kernel outside a container.
    NothingRequiredOnUnreliableHost,
    /// The profile requires observations, the case declares them, and the Microsoft kernel runs in a container.
    RequiredOnContainedHost,
    /// A corpus run whose one rule's profile requires observations, on an unreliable host.
    CorpusRequiredOnUnreliableHost,
    /// A corpus run whose rules' profiles require nothing, on an unreliable host.
    CorpusNothingRequiredOnUnreliableHost,
    /// A corpus run of a rule without a dedicated profile, whose bench profile requires observations, on an unreliable host.
    CorpusBenchProfileOnUnreliableHost,
}

/// What a row established once admission answered.
#[derive(Debug, PartialEq, Eq)]
enum Admitted {
    Cases,
}

/// What a refused admission states.
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    ObservationsMissing,
    MissingProfileBoundary,
    HostUnreliable,
    Unexpected,
}

type Input = Scenario;
type Value = Admitted;
type Failure = Refusal;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a profile without file observations admits a case without metadata"),
        word: Word::Succeeds,
        input: || Scenario::NothingRequired,
        expect: Expect::Value(Admitted::Cases),
    },
    Case {
        name: CaseName("a profile that requires observations admits a case that declares them"),
        word: Word::Succeeds,
        input: || Scenario::Declared,
        expect: Expect::Value(Admitted::Cases),
    },
    Case {
        name: CaseName("a profile that requires observations refuses a case that omits them"),
        word: Word::Refuses,
        input: || Scenario::Omitted,
        expect: Expect::Error(Refusal::ObservationsMissing),
    },
    Case {
        name: CaseName("a rules directory without a profile boundary is refused"),
        word: Word::Refuses,
        input: || Scenario::NoProfileBoundary,
        expect: Expect::Error(Refusal::MissingProfileBoundary),
    },
    Case {
        name: CaseName("a profile requiring file observations is refused on an unreliable host"),
        word: Word::Refuses,
        input: || Scenario::RequiredOnUnreliableHost,
        expect: Expect::Error(Refusal::HostUnreliable),
    },
    Case {
        name: CaseName("a profile requiring none is admitted on an unreliable host"),
        word: Word::Succeeds,
        input: || Scenario::NothingRequiredOnUnreliableHost,
        expect: Expect::Value(Admitted::Cases),
    },
    Case {
        name: CaseName("required observations are admitted on a host that honours them"),
        word: Word::Succeeds,
        input: || Scenario::RequiredOnContainedHost,
        expect: Expect::Value(Admitted::Cases),
    },
    Case {
        name: CaseName(
            "one rule whose profile requires file observations refuses the run on an unreliable host",
        ),
        word: Word::Refuses,
        input: || Scenario::CorpusRequiredOnUnreliableHost,
        expect: Expect::Error(Refusal::HostUnreliable),
    },
    Case {
        name: CaseName("rules whose profiles declare none are admitted on an unreliable host"),
        word: Word::Succeeds,
        input: || Scenario::CorpusNothingRequiredOnUnreliableHost,
        expect: Expect::Value(Admitted::Cases),
    },
    Case {
        name: CaseName("a rule without a dedicated profile is judged by the bench profile"),
        word: Word::Refuses,
        input: || Scenario::CorpusBenchProfileOnUnreliableHost,
        expect: Expect::Error(Refusal::HostUnreliable),
    },
];

/// A host of the given kernel and isolation.
fn host(kernel: HostKernel, isolation: HostIsolation) -> HostFake {
    HostFake {
        facts: HostFacts {
            family: HostFamily::Unix,
            kernel,
            isolation,
        },
        ..HostFake::default()
    }
}

/// A bench whose root profile requires executable bits and whose rule `B` has a dedicated profile that requires none.
fn bench()
-> Result<tempfile::TempDir, ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
    let root = tempfile::tempdir()?;
    let profile = "tool = 'fixture'\nversion = '1'\nsha256 = ['00']\ndefaults = ['A', 'B']\n";
    std::fs::write(
        root.path().join("benchmark.toml"),
        format!("{profile}file-observation = 'posix-executable'\n"),
    )?;
    std::fs::create_dir_all(root.path().join("profiles/B"))?;
    std::fs::write(root.path().join("profiles/B/benchmark.toml"), profile)?;
    Ok(root)
}

fn prepared(declared: bool) -> PreparedCase {
    let mut prepared = case();
    if declared {
        prepared.request.file_metadata =
            FileMetadataDeclaration::Declared(std::collections::BTreeMap::from([(
                prepared.request.files[0].clone(),
                FileExecutable::Unset,
            )]));
    }
    prepared
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut tool = profile();
    let unreliable = host(HostKernel::Microsoft, HostIsolation::Bare);
    let honouring = HostFake::default();
    let rule = |name: &str| ryo::linter::RuleId::try_from(name).map_err(|_| Refusal::Unexpected);
    let answer = match scenario {
        Scenario::CorpusRequiredOnUnreliableHost
        | Scenario::CorpusNothingRequiredOnUnreliableHost => {
            if matches!(scenario, Scenario::CorpusRequiredOnUnreliableHost) {
                tool.file_observation = FileObservation::PosixExecutable;
            }
            let suite = SuiteFake {
                profile: Some(tool),
                ..Default::default()
            };
            admit_rules(
                &suite,
                &unreliable,
                Path::new("bench"),
                &[rule("A")?, rule("B")?],
            )
        }
        Scenario::CorpusBenchProfileOnUnreliableHost => {
            let bench = bench().map_err(|_| Refusal::Unexpected)?;
            let suite = crate::outbound::suite::factory::local();
            // B's dedicated profile requires nothing, so B alone is admitted; A has none and is judged by the bench's.
            admit_rules(&suite, &unreliable, bench.path(), &[rule("B")?])
                .map_err(|_| Refusal::Unexpected)?;
            admit_rules(&suite, &unreliable, bench.path(), &[rule("A")?])
        }
        Scenario::NothingRequired
        | Scenario::Declared
        | Scenario::Omitted
        | Scenario::NoProfileBoundary
        | Scenario::RequiredOnUnreliableHost
        | Scenario::NothingRequiredOnUnreliableHost
        | Scenario::RequiredOnContainedHost => {
            let (rules, cases, machine) = match scenario {
                Scenario::Declared => {
                    tool.file_observation = FileObservation::PosixExecutable;
                    (Path::new("profile/rules"), prepared(true), honouring)
                }
                Scenario::Omitted => {
                    tool.file_observation = FileObservation::PosixExecutable;
                    (Path::new("profile/rules"), prepared(false), honouring)
                }
                Scenario::NoProfileBoundary => (Path::new(""), prepared(false), honouring),
                Scenario::RequiredOnUnreliableHost => {
                    tool.file_observation = FileObservation::PosixExecutable;
                    (Path::new("profile/rules"), prepared(true), unreliable)
                }
                Scenario::NothingRequiredOnUnreliableHost => {
                    (Path::new("profile/rules"), prepared(false), unreliable)
                }
                Scenario::RequiredOnContainedHost => {
                    tool.file_observation = FileObservation::PosixExecutable;
                    (
                        Path::new("profile/rules"),
                        prepared(true),
                        host(HostKernel::Microsoft, HostIsolation::Container),
                    )
                }
                Scenario::NothingRequired
                | Scenario::CorpusRequiredOnUnreliableHost
                | Scenario::CorpusNothingRequiredOnUnreliableHost
                | Scenario::CorpusBenchProfileOnUnreliableHost => {
                    (Path::new("profile/rules"), prepared(false), honouring)
                }
            };
            let suite = SuiteFake {
                profile: Some(tool),
                ..Default::default()
            };
            admit(&suite, &machine, rules, &[cases])
        }
    };
    answer.map(|()| Admitted::Cases).map_err(|error| {
        let message = error.to_string();
        if message == constants::MISSING_CASE_PROFILE_BOUNDARY {
            Refusal::MissingProfileBoundary
        } else if message == constants::HOST_DOES_NOT_HONOUR_FILE_PERMISSIONS {
            Refusal::HostUnreliable
        } else if message.contains("observation") {
            Refusal::ObservationsMissing
        } else {
            Refusal::Unexpected
        }
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
