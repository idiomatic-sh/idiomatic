//! TypeScript composition provides its linked language only at this outer edge.
use self::commands::{audit, reference, verify_cases, verify_reference_cases, worker};
use self::constants::{AUDIT, EXPECTED_EXECUTABLE, UNSUPPORTED_WORKER_COMMAND, USAGE};
use self::locations::directory;
use self::measure::measure;
use benchmark::domain::command_line::types::{
    BenchmarkArgument, measure_command::value::MeasureCommand, worker_command::value::WorkerCommand,
};
use benchmark::outbound::linter::interface::LinterPort as _;
use benchmark::outbound::suite::types::ArtifactPath;
use benchmark_contracts::domain::receipts::constants::VERIFY_CASES;
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use benchmark_contracts::domain::requests::constants::{
    DIAGNOSTIC_RYO, MEASURE, MEASURE_ACCOUNTED_VALID, MEASURE_SCOPED, MEASURE_VALID, RUN_RYO,
    VERIFY_REFERENCE_CASES, VERIFY_TOOL,
};
use std::{path::PathBuf, sync::Arc};
mod certification;
mod commands;
pub(crate) mod constants;
mod locations;
mod measure;
/// Dispatch benchmark commands and retain the complete certification-to-observation timeline.
pub(crate) fn run(
    arguments: Vec<BenchmarkArgument>,
    output: &mut dyn std::io::Write,
    observations: &mut dyn std::io::Write,
) -> Result<(), Error> {
    let mut args = arguments.into_iter().map(|argument| argument.0);
    let suite_source: Arc<dyn benchmark::outbound::suite::interface::SuitePort> =
        Arc::new(benchmark::outbound::suite::factory::local());
    let command = args.next();
    match command.as_deref() {
        Some(RUN_RYO | DIAGNOSTIC_RYO) => worker(
            args.map(BenchmarkArgument),
            WorkerCommand(command.as_deref().unwrap_or_default())
                .tracing()
                .ok_or_else(|| Error::Invalid(UNSUPPORTED_WORKER_COMMAND.into()))?,
        ),
        Some(VERIFY_CASES) => {
            let count = verify_cases(args.map(BenchmarkArgument))?;
            writeln!(output, "{count} cases passed against both contestants")?;
            Ok(())
        }
        Some(VERIFY_REFERENCE_CASES) => {
            let count = verify_reference_cases(args.map(BenchmarkArgument))?;
            writeln!(
                output,
                "{count} cases matched the reference; no candidate was executed"
            )?;
            Ok(())
        }
        Some(MEASURE | MEASURE_ACCOUNTED_VALID | MEASURE_SCOPED | MEASURE_VALID) => measure(
            MeasureCommand(command.as_deref().unwrap_or_default()),
            &mut args.map(BenchmarkArgument),
            observations,
            &suite_source,
        ),
        // No command at all: how `cargo test --all-targets` runs this bench (it passes no argument, not even `--bench`). It
        // audits the authored queries against the shipped defaults, which reads no reference linter, and measures nothing:
        // every other bench of the repository proves itself the same way under `cargo test`.
        Some(AUDIT) | None => audit(&suite_source, output),
        Some(VERIFY_TOOL) => {
            let executable = PathBuf::from(
                args.next()
                    .ok_or_else(|| Error::Invalid(EXPECTED_EXECUTABLE.into()))?,
            );
            let tool = suite_source.profile(ArtifactPath(&directory()))?;
            let expected = reference(executable.clone(), tool.clone()).version_line(&tool);
            suite_source.verify_tool(ArtifactPath(&executable), &tool, &expected)
        }
        _ => Err(Error::Invalid(USAGE.into())),
    }
}
