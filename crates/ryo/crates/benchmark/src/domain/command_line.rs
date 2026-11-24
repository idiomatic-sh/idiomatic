//! The command line of a linter's benchmark executable: its arguments, worker commands and measure commands.
/// The values a command line is read into.
pub mod types;

mod constants;

/// Read the benchmark's command, dropping Cargo's harness marker when present.
#[must_use]
pub fn arguments() -> Vec<types::BenchmarkArgument> {
    let mut command: Vec<_> = std::env::args()
        .skip(1)
        .map(types::BenchmarkArgument)
        .collect();
    if let Some(index) = command.iter().position(|arg| arg.0 == "--bench") {
        drop(command.remove(index));
    }
    command
}
