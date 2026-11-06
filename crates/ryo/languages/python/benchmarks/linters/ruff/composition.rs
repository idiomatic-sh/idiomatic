//! Python composition provides its linked language only at this outer edge.
use self::constants::UNSUPPORTED_WORKER_COMMAND;
use ryo_benchmark::outbound::suite::types::ArtifactPath;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::{path::PathBuf, sync::Arc};
mod commands;
mod constants;
mod corpus;
mod diagnostic_protocol;
mod measure;
mod output;
mod request_input;
mod targets;
/// Dispatch benchmark commands and retain the complete certification-to-observation timeline.
pub(crate) fn run(
    output: &mut dyn std::io::Write,
    observations: &mut dyn std::io::Write,
) -> Result<(), Error> {
    let mut args = std::env::args()
        .skip(1)
        .filter(|argument| argument != self::constants::CARGO_BENCH_ARGUMENT);
    let suite_source: Arc<dyn ryo_benchmark::outbound::suite::interface::SuitePort> =
        Arc::new(ryo_benchmark::outbound::suite::factory::local());
    let command = args.next();
    match command.as_deref() {
        Some(
            self::constants::RUN_RYO
            | self::constants::PROFILE_RYO
            | self::constants::DIAGNOSTIC_RYO,
        ) => self::commands::worker(
            args.map(self::request_input::BenchmarkArgument),
            self::diagnostic_protocol::tracing_for_command(command.as_deref().unwrap_or_default())
                .ok_or_else(|| Error::Invalid(UNSUPPORTED_WORKER_COMMAND.into()))?,
        ),
        Some(self::constants::VERIFY_CASES) => {
            let count =
                self::commands::verify_cases(args.map(self::request_input::BenchmarkArgument))?;
            writeln!(output, "{count} cases passed against both contestants")?;
            Ok(())
        }
        Some(self::constants::VERIFY_REFERENCE_CASES) => {
            let count = self::commands::verify_reference_cases(
                args.map(self::request_input::BenchmarkArgument),
            )?;
            writeln!(
                output,
                "{count} cases matched the reference; no candidate was executed"
            )?;
            Ok(())
        }
        Some(
            self::constants::MEASURE
            | self::constants::MEASURE_ACCOUNTED_VALID
            | self::constants::MEASURE_SCOPED
            | self::constants::MEASURE_VALID,
        ) => self::measure::measure(
            command.as_deref().unwrap_or_default(),
            &mut args,
            observations,
            &suite_source,
        ),
        Some(self::constants::AUDIT) => self::commands::audit(&suite_source, output),
        Some(self::constants::VERIFY_TOOL) => {
            let executable = PathBuf::from(
                args.next()
                    .ok_or_else(|| Error::Invalid(self::constants::EXPECTED_EXECUTABLE.into()))?,
            );
            suite_source.verify_tool(
                ArtifactPath(&executable),
                &suite_source.profile(ArtifactPath(&self::commands::directory()))?,
            )
        }
        _ => Err(Error::Invalid(self::constants::USAGE.into())),
    }
}
