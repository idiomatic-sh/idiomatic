//! TypeScript composition provides its linked language only at this outer edge.
use self::commands::{audit, reference, verify_cases, verify_reference_cases, worker};
use self::compare::parity;
use self::constants::{
    AUDIT, CARGO_BENCH_ARGUMENT, DIAGNOSTIC_RYO, EXPECTED_EXECUTABLE, MEASURE,
    MEASURE_ACCOUNTED_VALID, MEASURE_SCOPED, MEASURE_VALID, PARITY, PROFILE_RYO, RUN_RYO,
    UNSUPPORTED_WORKER_COMMAND, USAGE, VERIFY_CASES, VERIFY_REFERENCE_CASES, VERIFY_TOOL,
};
use self::locations::directory;
use self::measure::measure;
use ryo_benchmark::domain::command_line::types::{
    BenchmarkArgument, measure_command::value::MeasureCommand, worker_command::value::WorkerCommand,
};
use ryo_benchmark::outbound::linter::interface::LinterPort as _;
use ryo_benchmark::outbound::suite::types::ArtifactPath;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::{path::PathBuf, sync::Arc};
mod certification;
mod commands;
mod compare;
pub(crate) mod constants;
mod locations;
mod measure;
/// Dispatch benchmark commands and retain the complete certification-to-observation timeline.
pub(crate) fn run(
    output: &mut dyn std::io::Write,
    observations: &mut dyn std::io::Write,
) -> Result<(), Error> {
    let mut args = std::env::args()
        .skip(1)
        .filter(|argument| argument != CARGO_BENCH_ARGUMENT);
    let suite_source: Arc<dyn ryo_benchmark::outbound::suite::interface::SuitePort> =
        Arc::new(ryo_benchmark::outbound::suite::factory::local());
    let command = args.next();
    match command.as_deref() {
        Some(RUN_RYO | PROFILE_RYO | DIAGNOSTIC_RYO) => worker(
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
        Some(PARITY) => parity(
            &mut args.map(BenchmarkArgument),
            observations,
            &suite_source,
        ),
        Some(AUDIT) => audit(&suite_source, output),
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
