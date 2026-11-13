//! What every `profile_tools` implementation must pass.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::profile_tools::interface::ProfileToolsPort;
use crate::outbound::profile_tools::types::FileContents;
use crate::domain::profiles::types::SamplingRate;
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;

/// What a row asks of the implementation.
#[derive(Clone, Copy, Debug)]
pub enum Row {
    /// A heap dump is written into a prepared directory, then the directory is listed.
    DumpWritten,
    /// A file that is not a heap dump is written into a prepared directory, then the directory is listed.
    OtherFileWritten,
    /// A directory never prepared is listed.
    NeverPrepared,
    /// The worker's invocation is put under the CPU profiler.
    CpuInvocation,
}
/// The row an implementation is asked.
type Input = Row;
/// What a row observes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// How many heap dumps the directory lists.
    Dumps(usize),
    /// Whether the profiled invocation still runs the worker's arguments, last, with its environment and directory.
    KeepsWorker(bool),
}
type Value = Answer;
/// What a contract row is refused with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The disk refused the operation.
    Io,
    /// The implementation failed for another reason.
    Other,
}
type Failure = Refusal;

/// The contract's rows: every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a heap dump written into a prepared directory is listed"),
        word: Word::Succeeds,
        input: || Row::DumpWritten,
        expect: Expect::Value(Answer::Dumps(1)),
    },
    Case {
        name: CaseName("a file that is not a heap dump is not listed"),
        word: Word::Succeeds,
        input: || Row::OtherFileWritten,
        expect: Expect::Value(Answer::Dumps(0)),
    },
    Case {
        name: CaseName("the profiled invocation runs the worker's arguments last, in its environment and directory"),
        word: Word::Succeeds,
        input: || Row::CpuInvocation,
        expect: Expect::Value(Answer::KeepsWorker(true)),
    },
    Case {
        name: CaseName("listing a directory never prepared fails"),
        word: Word::Fails,
        input: || Row::NeverPrepared,
        expect: Expect::Error(Refusal::Io),
    },
];

/// Run the shared rows against one implementation, keeping its files under `root`.
///
/// # Panics
///
/// Panics when the implementation violates a contract row.
pub fn check(port: &impl ProfileToolsPort, root: &Path) {
    let subject = |row: Row| -> Result<Value, Failure> {
        let classify = |error: Error| match error {
            Error::Io(_) => Refusal::Io,
            Error::Invalid(_) | Error::Json(_) | Error::Toml(_) => Refusal::Other,
        };
        let directory = root.join(format!("{row:?}"));
        match row {
            Row::DumpWritten | Row::OtherFileWritten => {
                port.prepare(&directory).map_err(classify)?;
                let name = if matches!(row, Row::DumpWritten) {
                    "jeprof.1.0.f.heap"
                } else {
                    "notes.txt"
                };
                port.write(&directory.join(name), &FileContents::default())
                    .map_err(classify)?;
                Ok(Answer::Dumps(port.dumps(&directory).map_err(classify)?.0.len()))
            }
            Row::NeverPrepared => Ok(Answer::Dumps(
                port.dumps(&directory).map_err(classify)?.0.len(),
            )),
            Row::CpuInvocation => {
                let worker = Invocation {
                    executable: "worker".into(),
                    arguments: vec!["run-ryo".into(), "request.json".into()],
                    environment: BTreeMap::from([(OsString::from("KEY"), OsString::from("value"))]),
                    directory: root.into(),
                };
                let profiled = port.cpu_invocation(
                    worker.clone(),
                    SamplingRate(99),
                    &directory.join("perf.data"),
                );
                Ok(Answer::KeepsWorker(
                    profiled.arguments.ends_with(&worker.arguments)
                        && profiled.environment == worker.environment
                        && profiled.directory == worker.directory,
                ))
            }
        }
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Fails, subject);
}
