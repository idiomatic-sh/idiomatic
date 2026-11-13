//! Python composition provides its linked language only at this outer edge.
use self::commands::reference;
use self::constants::UNSUPPORTED_WORKER_COMMAND;
use self::locations::directory;
use ryo_benchmark::domain::command_line::types::{
    BenchmarkArgument, measure_command::value::MeasureCommand, worker_command::value::WorkerCommand,
};
use ryo_benchmark::outbound::linter::interface::LinterPort as _;
use ryo_benchmark::outbound::suite::types::ArtifactPath;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::constants::{
    DIAGNOSTIC_RYO, MEASURE, MEASURE_ACCOUNTED_VALID, MEASURE_SCOPED, MEASURE_VALID, PROFILE_RYO,
    RUN_RYO,
};
use std::{path::PathBuf, sync::Arc};
mod certification;
mod commands;
pub(crate) mod constants;
mod locations;
mod measure;
mod parity;
pub(crate) mod profile;
mod request_input;
mod targets;
/// Dispatch benchmark commands and retain the complete certification-to-observation timeline.
pub(crate) fn run(
    arguments: Vec<BenchmarkArgument>,
    output: &mut dyn std::io::Write,
    observations: &mut dyn std::io::Write,
) -> Result<(), Error> {
    let mut args = arguments.into_iter().map(|argument| argument.0);
    let suite_source: Arc<dyn ryo_benchmark::outbound::suite::interface::SuitePort> =
        Arc::new(ryo_benchmark::outbound::suite::factory::local());
    let command = args.next();
    match command.as_deref() {
        Some(RUN_RYO | PROFILE_RYO | DIAGNOSTIC_RYO) => self::commands::worker(
            args.map(BenchmarkArgument),
            WorkerCommand(command.as_deref().unwrap_or_default())
                .tracing()
                .ok_or_else(|| Error::Invalid(UNSUPPORTED_WORKER_COMMAND.into()))?,
        ),
        Some(self::constants::VERIFY_CASES) => {
            let count = self::commands::verify_cases(args.map(BenchmarkArgument))?;
            writeln!(output, "{count} cases passed against both contestants")?;
            Ok(())
        }
        Some(self::constants::VERIFY_REFERENCE_CASES) => {
            let count = self::commands::verify_reference_cases(args.map(BenchmarkArgument))?;
            writeln!(
                output,
                "{count} cases matched the reference; no candidate was executed"
            )?;
            Ok(())
        }
        Some(MEASURE | MEASURE_ACCOUNTED_VALID | MEASURE_SCOPED | MEASURE_VALID) => {
            self::measure::measure(
                MeasureCommand(command.as_deref().unwrap_or_default()),
                &mut args.map(BenchmarkArgument),
                observations,
                &suite_source,
            )
        }
        Some(self::constants::PARITY) => self::parity::parity(
            &mut args.map(BenchmarkArgument),
            observations,
            &suite_source,
        ),
        Some(self::constants::AUDIT) => self::commands::audit(&suite_source, output),
        Some(self::constants::VERIFY_TOOL) => {
            let executable = PathBuf::from(
                args.next()
                    .ok_or_else(|| Error::Invalid(self::constants::EXPECTED_EXECUTABLE.into()))?,
            );
            let tool = suite_source.profile(ArtifactPath(&directory()))?;
            let expected = reference(executable.clone(), tool.clone())?.version_line(&tool);
            suite_source.verify_tool(ArtifactPath(&executable), &tool, &expected)
        }
        _ => Err(Error::Invalid(self::constants::USAGE.into())),
    }
}
