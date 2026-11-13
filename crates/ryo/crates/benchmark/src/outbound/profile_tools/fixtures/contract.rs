//! What every `profile_tools` implementation must pass.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::profile_tools::interface::ProfileToolsPort;
use crate::outbound::profile_tools::types::{
    DumpTotal, FileContents, HeapBytes, ProfileReceipt, SamplingRate,
};
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;

/// What a row asks of the implementation.
#[derive(Clone, Copy, Debug)]
pub enum Row {
    /// The threads the worker spreads over are asked.
    Threads,
    /// A heap dump is written into a prepared folder, then the folder is listed.
    DumpWritten,
    /// A file that is not a heap dump is written into a prepared folder, then the folder is listed.
    OtherFileWritten,
    /// A folder never prepared is listed.
    NeverPrepared,
    /// The worker's invocation is put under the CPU profiler.
    CpuInvocation,
    /// A receipt is written into a prepared folder.
    Receipt,
    /// The total of a dump that was never written is asked.
    TotalOfNoDump,
    /// The report of a dump that was never written is asked.
    ReportOfNoDump,
    /// The reports of a recording no CPU profiler wrote are asked.
    ReportOfNoRecording,
}
/// The row an implementation is asked.
type Input = Row;
/// What a row observes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// Whether at least one thread is answered.
    Threads(bool),
    /// How many heap dumps the folder lists.
    Dumps(usize),
    /// Whether the profiled invocation still runs the worker's arguments, last, with its environment and directory.
    KeepsWorker(bool),
    /// The operation completed.
    Done,
}
type Value = Answer;
/// What a contract row is refused with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The disk refused the operation.
    Io,
    /// The profiler refused or failed.
    Other,
}
type Failure = Refusal;

/// The contract's rows: every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("at least one thread is answered"),
        word: Word::Succeeds,
        input: || Row::Threads,
        expect: Expect::Value(Answer::Threads(true)),
    },
    Case {
        name: CaseName("a heap dump written into a prepared folder is listed"),
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
        name: CaseName(
            "the profiled invocation runs the worker's arguments last, in its environment and directory",
        ),
        word: Word::Succeeds,
        input: || Row::CpuInvocation,
        expect: Expect::Value(Answer::KeepsWorker(true)),
    },
    Case {
        name: CaseName("a receipt is written into a prepared folder"),
        word: Word::Succeeds,
        input: || Row::Receipt,
        expect: Expect::Value(Answer::Done),
    },
    Case {
        name: CaseName("listing a folder never prepared fails"),
        word: Word::Fails,
        input: || Row::NeverPrepared,
        expect: Expect::Error(Refusal::Io),
    },
    Case {
        name: CaseName("the total of a dump never written is refused"),
        word: Word::Refuses,
        input: || Row::TotalOfNoDump,
        expect: Expect::Error(Refusal::Other),
    },
    Case {
        name: CaseName("the report of a dump never written is refused"),
        word: Word::Refuses,
        input: || Row::ReportOfNoDump,
        expect: Expect::Error(Refusal::Other),
    },
    Case {
        name: CaseName("the reports of a recording never made are refused"),
        word: Word::Refuses,
        input: || Row::ReportOfNoRecording,
        expect: Expect::Error(Refusal::Other),
    },
];

/// Run the shared rows against one implementation, keeping its files under `root`.
///
/// A profiler a machine lacks fails to start, which a row counts as the profiler's refusal.
///
/// # Panics
///
/// Panics when the implementation violates a contract row.
pub fn check(port: &impl ProfileToolsPort, root: &Path) {
    let classify = |error: Error| match error {
        Error::Io(_) => Refusal::Io,
        Error::Invalid(_) | Error::Json(_) | Error::Toml(_) => Refusal::Other,
    };
    let subject = |row: Row| -> Result<Value, Failure> {
        let folder = root.join(format!("{row:?}"));
        let worker = root.join("worker");
        let missing = folder.join("jeprof.1.0.f.heap");
        let refused = |_error: Error| Refusal::Other;
        match row {
            Row::Threads => Ok(Answer::Threads(port.threads().get() >= 1)),
            Row::DumpWritten | Row::OtherFileWritten => {
                port.prepare(&folder).map_err(classify)?;
                let name = if matches!(row, Row::DumpWritten) {
                    "jeprof.1.0.f.heap"
                } else {
                    "notes.txt"
                };
                port.write(&folder.join(name), &FileContents::default())
                    .map_err(classify)?;
                Ok(Answer::Dumps(
                    port.dumps(&folder).map_err(classify)?.0.len(),
                ))
            }
            Row::NeverPrepared => Ok(Answer::Dumps(
                port.dumps(&folder).map_err(classify)?.0.len(),
            )),
            Row::CpuInvocation => {
                let invocation = Invocation {
                    executable: worker,
                    arguments: vec!["run-ryo".into(), "request.json".into()],
                    environment: BTreeMap::from([(OsString::from("KEY"), OsString::from("value"))]),
                    directory: root.into(),
                };
                let profiled = port.cpu_invocation(
                    invocation.clone(),
                    SamplingRate(99),
                    &folder.join("perf.data"),
                );
                Ok(Answer::KeepsWorker(
                    profiled.arguments.ends_with(&invocation.arguments)
                        && profiled.environment == invocation.environment
                        && profiled.directory == invocation.directory,
                ))
            }
            Row::Receipt => {
                port.prepare(&folder).map_err(classify)?;
                port.receipt(
                    &folder,
                    &ProfileReceipt {
                        directory: folder.clone(),
                        peak: DumpTotal {
                            dump: missing,
                            bytes: HeapBytes(64),
                        },
                        rate: SamplingRate(99),
                    },
                )
                .map_err(classify)?;
                Ok(Answer::Done)
            }
            Row::TotalOfNoDump => {
                let _total = port.heap_total(&worker, &missing).map_err(refused)?;
                Ok(Answer::Done)
            }
            Row::ReportOfNoDump => {
                port.prepare(&folder).map_err(classify)?;
                port.heap_report(&worker, &missing, &folder.join("peak.txt"))
                    .map_err(refused)?;
                Ok(Answer::Done)
            }
            Row::ReportOfNoRecording => {
                port.prepare(&folder).map_err(classify)?;
                port.cpu_report(&folder.join("perf.data"), &folder)
                    .map_err(refused)?;
                Ok(Answer::Done)
            }
        }
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
    run(CASES, Word::Fails, subject);
}
