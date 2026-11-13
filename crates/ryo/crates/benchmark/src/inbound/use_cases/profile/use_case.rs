//! The profile run: the worker once under the heap profiler and once under the CPU profiler, then their reports.
use super::constants::{
    CARGO_BENCH_ARGUMENT, CHILD_FAILED, CPU, DIFFERING_STDOUT, DUMP_PREFIX, DUMPS, HEAP,
    HEAP_PROFILER_CONFIGURATION, NO_HEAP_DUMP, PEAK, RECORDING, STDOUT, TOTALS, TRACE,
};
use super::interface::ProfileUseCase;
use super::service::{dump_order, heap_configuration, peak, sampling_rate, split_profile};
use super::types::{Profile, ProfileArguments, ProfileReport, ProfileRequest};
use crate::domain::command_line::types::BenchmarkArgument;
use crate::outbound::execution::interface::ExecutionPort;
use crate::outbound::profile_tools::interface::ProfileToolsPort;
use crate::outbound::profile_tools::types::{DumpTotal, FileContents, ProfileReceipt};
use ryo_benchmark_contracts::domain::measurements::types::{
    Invocation, ProcessOutput, TerminationSignal,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::constants::{PROFILE_RYO, RUN_RYO};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Component, PathBuf};
use std::sync::Arc;

/// Runs the profiled children through the injected execution and the profilers' reports through the injected tools.
pub struct ProfileUseCaseImpl {
    /// Runs the worker's children.
    execution: Arc<dyn ExecutionPort>,
    /// The heap and CPU profilers and the run's files.
    tools: Arc<dyn ProfileToolsPort>,
}
impl ProfileUseCaseImpl {
    /// Bind the execution the children run through and the profilers.
    #[must_use]
    pub fn new(execution: Arc<dyn ExecutionPort>, tools: Arc<dyn ProfileToolsPort>) -> Self {
        Self { execution, tools }
    }

    /// Run one child; refuse it unless it exited with 0.
    fn child(&self, invocation: Invocation) -> Result<ProcessOutput, Error> {
        let output = self.execution.run(invocation)?;
        if output.status.0 == 0 && output.signal == TerminationSignal::Exited {
            Ok(output)
        } else {
            Err(Error::Invalid(format!(
                "{CHILD_FAILED}: exit {}; {}",
                output.status.0,
                String::from_utf8_lossy(&output.stderr.0).trim()
            )))
        }
    }
}
impl ProfileUseCase for ProfileUseCaseImpl {
    fn profile(&self, request: ProfileRequest) -> Result<ProfileReport, Error> {
        let mut command = request.arguments;
        if let Some(at) = command
            .iter()
            .position(|argument| argument.0 == CARGO_BENCH_ARGUMENT)
        {
            drop(command.remove(at));
        }
        if command.first().is_none_or(|word| word.0 != PROFILE_RYO) {
            return Ok(ProfileReport {
                command,
                profile: Profile::Unasked,
            });
        }
        let ProfileArguments { worker, directory } = split_profile(command.split_off(1))?;
        let heap = directory.join(HEAP);
        let dumps = heap.join(DUMPS);
        let cpu = directory.join(CPU);
        self.tools.prepare(&dumps)?;
        self.tools.prepare(&cpu)?;
        let mut arguments: Vec<OsString> = vec![RUN_RYO.into()];
        arguments.extend(worker.iter().map(|argument| OsString::from(&argument.0)));
        let run = Invocation {
            executable: request.worker.clone(),
            arguments,
            environment: BTreeMap::new(),
            directory: PathBuf::from(Component::CurDir.as_os_str()),
        };
        let mut heap_run = run.clone();
        drop(heap_run.environment.insert(
            HEAP_PROFILER_CONFIGURATION.into(),
            heap_configuration(&dumps.join(DUMP_PREFIX)),
        ));
        let heap_output = self.child(heap_run)?;
        let rate = sampling_rate(heap_output.wall, self.tools.threads());
        let recording = cpu.join(RECORDING);
        let cpu_output = self.child(self.tools.cpu_invocation(run, rate, &recording))?;
        if heap_output.stdout != cpu_output.stdout {
            return Err(Error::Invalid(DIFFERING_STDOUT.into()));
        }
        self.tools
            .write(&heap.join(STDOUT), &FileContents(heap_output.stdout.0))?;
        self.tools
            .write(&cpu.join(STDOUT), &FileContents(cpu_output.stdout.0))?;
        let mut totals = Vec::new();
        for dump in dump_order(self.tools.dumps(&dumps)?.0) {
            let bytes = self.tools.heap_total(&request.worker, &dump)?;
            totals.push(DumpTotal { dump, bytes });
        }
        let lines = totals
            .iter()
            .map(|total| format!("{} {}\n", total.bytes.0, total.dump.display()))
            .collect::<String>();
        self.tools
            .write(&heap.join(TOTALS), &FileContents(lines.into_bytes()))?;
        let highest = peak(&totals)
            .cloned()
            .ok_or_else(|| Error::Invalid(NO_HEAP_DUMP.into()))?;
        self.tools
            .heap_report(&request.worker, &highest.dump, &heap.join(PEAK))?;
        self.tools.cpu_report(&recording, &cpu)?;
        let receipt = ProfileReceipt {
            directory: directory.clone(),
            peak: highest,
            rate,
        };
        self.tools.receipt(&directory, &receipt)?;
        let mut next = vec![BenchmarkArgument(PROFILE_RYO.into())];
        next.extend(worker);
        next.push(BenchmarkArgument(
            directory.join(TRACE).to_string_lossy().into_owned(),
        ));
        Ok(ProfileReport {
            command: next,
            profile: Profile::Recorded(receipt),
        })
    }
}
impl std::fmt::Debug for ProfileUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
