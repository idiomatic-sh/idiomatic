//! The harness's profile run, wired at this bench's edge: the heap and CPU profiles when the arguments ask for `profile-ryo`.
use ryo_benchmark::domain::command_line::types::BenchmarkArgument;
use ryo_benchmark::inbound::use_cases::profile::interface::ProfileUseCase as _;
use ryo_benchmark::inbound::use_cases::profile::types::ProfileRequest;
use ryo_benchmark::inbound::use_cases::profile::use_case::ProfileUseCaseImpl;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::sync::Arc;

/// The command this bench runs: cargo's `--bench` removed, after the profile run when the arguments ask for one.
pub(crate) fn command() -> Result<Vec<BenchmarkArgument>, Error> {
    let report = ProfileUseCaseImpl::new(
        Arc::new(ryo_benchmark::outbound::execution::factory::process()),
        Arc::new(ryo_benchmark::outbound::profile_tools::factory::linux()),
    )
    .profile(ProfileRequest {
        worker: std::env::current_exe()?,
        arguments: std::env::args().skip(1).map(BenchmarkArgument).collect(),
    })?;
    Ok(report.command)
}
