//! The profile run through its fakes: the children it runs, the reports it asks for and the command it answers.
use super::fixtures::ChildrenFake;
use super::interface::ProfileUseCase as _;
use super::types::ProfileRequest;
use super::use_case::ProfileUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::command_line::types::BenchmarkArgument;
use crate::domain::profiles::types::{DumpTotal, HeapBytes};
use crate::outbound::profile_tools::fixtures::fake::ProfileToolsFake;
use ryo_benchmark_contracts::domain::measurements::types::{CapturedBytes, ProcessExitCode};
use std::borrow::Cow;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Arc;

/// What each row runs and observes.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    Unasked,
    HeapChild,
    CpuChild,
    PeakReport,
    AnsweredCommand,
    DifferingStdout,
    NoDump,
    FailedChild,
}
type Input = Scenario;
type Value = Cow<'static, str>;
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a command line without profile-ryo comes back unasked, cargo's --bench removed"),
        word: Word::Succeeds,
        input: || Scenario::Unasked,
        expect: Expect::Value(Cow::Borrowed("")),
    },
    Case {
        name: CaseName("the heap child runs run-ryo with the heap profiler's configuration"),
        word: Word::Succeeds,
        input: || Scenario::HeapChild,
        expect: Expect::Value(Cow::Borrowed("")),
    },
    Case {
        name: CaseName("the CPU child runs under the CPU profiler at the rate of the heap child's wall time and the threads"),
        word: Word::Succeeds,
        input: || Scenario::CpuChild,
        expect: Expect::Value(Cow::Borrowed("")),
    },
    Case {
        name: CaseName("the largest dump is the peak and its report is asked for"),
        word: Word::Succeeds,
        input: || Scenario::PeakReport,
        expect: Expect::Value(Cow::Borrowed("")),
    },
    Case {
        name: CaseName("the bench runs profile-ryo next, writing its trace into the directory"),
        word: Word::Succeeds,
        input: || Scenario::AnsweredCommand,
        expect: Expect::Value(Cow::Borrowed("")),
    },
    Case {
        name: CaseName("children that print different output are refused"),
        word: Word::Refuses,
        input: || Scenario::DifferingStdout,
        expect: Expect::Error(Cow::Borrowed("")),
    },
    Case {
        name: CaseName("a worker that writes no heap dump is refused, naming the feature"),
        word: Word::Refuses,
        input: || Scenario::NoDump,
        expect: Expect::Error(Cow::Borrowed("")),
    },
    Case {
        name: CaseName("a child that exits unsuccessfully fails the run"),
        word: Word::Fails,
        input: || Scenario::FailedChild,
        expect: Expect::Error(Cow::Borrowed("")),
    },
];

/// The arguments `words` name.
fn arguments(words: &[&str]) -> Vec<BenchmarkArgument> {
    words
        .iter()
        .map(|word| BenchmarkArgument((*word).into()))
        .collect()
}

/// A dump named `name` totalling `bytes`.
fn total(name: &str, bytes: u64) -> DumpTotal {
    DumpTotal {
        dump: name.into(),
        bytes: HeapBytes(bytes),
    }
}

#[expect(clippy::expect_used, reason = "a poisoned fixture lock must fail the test")]
fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut children = ChildrenFake::default();
    match scenario {
        Scenario::DifferingStdout => {
            children.stdout = vec![
                CapturedBytes(b"diagnostics".to_vec()),
                CapturedBytes(b"other".to_vec()),
            ];
        }
        Scenario::FailedChild => children.status = ProcessExitCode(2),
        _ => {}
    }
    let mut tools = ProfileToolsFake {
        threads: NonZeroUsize::new(8).unwrap_or(NonZeroUsize::MIN),
        ..ProfileToolsFake::default()
    };
    if !matches!(scenario, Scenario::NoDump) {
        tools.totals = vec![
            total("jeprof.7.10.i10.heap", 7),
            total("jeprof.7.9.i9.heap", 9),
            total("jeprof.7.11.f.heap", 4),
        ];
    }
    let words: &[&str] = if matches!(scenario, Scenario::Unasked) {
        &["run-ryo", "request.json", "--bench"]
    } else {
        &["profile-ryo", "request.json", "python-target=3.12", "out", "--bench"]
    };
    let report = ProfileUseCaseImpl::new(Arc::new(children.clone()), Arc::new(tools.clone()))
        .profile(ProfileRequest {
            worker: "target/worker".into(),
            arguments: arguments(words),
        })
        .map_err(|error| Cow::Owned(error.to_string()))?;
    let history = children.history.lock().expect("fixture history");
    let answer = match scenario {
        Scenario::HeapChild => format!("{:?}", history.first()),
        Scenario::CpuChild => format!("{:?}", history.get(1)),
        Scenario::PeakReport => format!(
            "{:?}",
            tools
                .files
                .lock()
                .expect("fixture files")
                .get(Path::new("out/heap/peak.txt"))
                .map(|report| String::from_utf8_lossy(&report.0).into_owned())
        ),
        Scenario::Unasked => format!("{:?} {:?}", report.command, report.profile),
        Scenario::AnsweredCommand
        | Scenario::DifferingStdout
        | Scenario::NoDump
        | Scenario::FailedChild => format!("{:?}", report.command),
    };
    Ok(Cow::Owned(answer))
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
