//! Behavioral proof of `process_failure`: the exact-binary archive group binds the receipt.
use super::errors::ProcessFailureError;
use super::fixtures::{archived, of, process};
use super::service::{check_archives, group_digest, output_digest};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::discrepancy::fixtures::profile;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::measurements::types::ProcessExitCode;
use ryo_benchmark_contracts::domain::process::types::ProcessFailure;
use ryo_benchmark_contracts::domain::workloads::types::Tool;

/// What a row asks of the process failure.
#[derive(Clone, Copy, Debug)]
enum Operation {
    GroupOf(&'static str),
    Archives(&'static str),
    OutputOfEditedExit,
}

/// The failure, its profile and the question.
struct Request {
    process: ProcessFailure,
    profile: Tool,
    operation: Operation,
}

/// What an accepted row answers.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Outcome {
    Accepted,
    Different,
}

type Input = Request;
type Value = Outcome;
type Failure = ProcessFailureError;

fn asking(operation: Operation, process: ProcessFailure) -> Request {
    Request {
        process,
        profile: profile(),
        operation,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("two captures of the actual binary bind a group digest"),
        word: Word::Succeeds,
        input: || asking(Operation::GroupOf("a"), process()),
        expect: Expect::Value(Outcome::Accepted),
    },
    Case {
        name: CaseName("two approved captures of the actual binary are archived evidence"),
        word: Word::Succeeds,
        input: || asking(Operation::Archives("a"), process()),
        expect: Expect::Value(Outcome::Accepted),
    },
    Case {
        name: CaseName("another exit status is another template digest"),
        word: Word::Succeeds,
        input: || asking(Operation::OutputOfEditedExit, process()),
        expect: Expect::Value(Outcome::Different),
    },
    Case {
        name: CaseName("a binary without captures has no group"),
        word: Word::Refuses,
        input: || asking(Operation::GroupOf("b"), process()),
        expect: Expect::Error(ProcessFailureError::NoReproducibleGroup),
    },
    Case {
        name: CaseName("one capture repeated is not two captures"),
        word: Word::Refuses,
        input: || {
            asking(
                Operation::GroupOf("a"),
                archived(vec![of(0, "a"), of(0, "a")]),
            )
        },
        expect: Expect::Error(ProcessFailureError::GroupsShareCaptures),
    },
    Case {
        name: CaseName("a single capture of the actual binary is not a pair"),
        word: Word::Refuses,
        input: || asking(Operation::GroupOf("a"), archived(vec![of(0, "a")])),
        expect: Expect::Error(ProcessFailureError::NoReproducibleGroup),
    },
    Case {
        name: CaseName("a capture of an unapproved binary is refused"),
        word: Word::Refuses,
        input: || {
            asking(
                Operation::Archives("a"),
                archived(vec![of(0, "9"), of(1, "9")]),
            )
        },
        expect: Expect::Error(ProcessFailureError::UnapprovedExactBinary),
    },
    Case {
        name: CaseName("two observations sharing a raw artifact are refused"),
        word: Word::Refuses,
        input: || {
            asking(
                Operation::Archives("a"),
                archived(vec![of(0, "a"), of(0, "a")]),
            )
        },
        expect: Expect::Error(ProcessFailureError::ObservationsShareArtifacts),
    },
    Case {
        name: CaseName("an archive without the actual binary needs two captures of it"),
        word: Word::Refuses,
        input: || asking(Operation::Archives("b"), process()),
        expect: Expect::Error(ProcessFailureError::ActualBinaryNeedsTwoCaptures),
    },
    Case {
        name: CaseName("an archive without any observation needs two captures"),
        word: Word::Refuses,
        input: || asking(Operation::Archives("a"), archived(Vec::new())),
        expect: Expect::Error(ProcessFailureError::ActualBinaryNeedsTwoCaptures),
    },
];

fn digest(letter: &str) -> ArtifactDigest {
    ArtifactDigest(letter.repeat(64))
}

fn subject(input: Input) -> Result<Value, Failure> {
    match input.operation {
        Operation::GroupOf(executable) => {
            group_digest(&input.process, &digest(executable)).map(|_| Outcome::Accepted)
        }
        Operation::Archives(actual) => {
            check_archives(&input.process, &input.profile, &digest(actual))
                .map(|()| Outcome::Accepted)
        }
        Operation::OutputOfEditedExit => {
            let mut edited = input.process.clone();
            edited.exit = ProcessExitCode(3);
            Ok(if output_digest(&edited) == output_digest(&input.process) {
                Outcome::Accepted
            } else {
                Outcome::Different
            })
        }
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
