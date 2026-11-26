//! The linter bench composes the shared benchmark and its candidate and reference.
mod adapter;
mod composition;
mod constants;
mod frontend_projection;
#[allow(dead_code, reason = "shared reader also serves the vector host")]
#[path = "../../../../../crates/engine/src/executor/json_input.rs"]
mod json_input;
mod schemas;
mod settings;

fn main() -> std::process::ExitCode {
    let mut output = std::io::stdout().lock();
    // Not locked for the run: the new engine's scan workers write their notices to stderr themselves (ryo-fmx3.1).
    let mut observations = std::io::stderr();
    let command = benchmark::domain::command_line::arguments(std::env::args().skip(1));
    let outcome = composition::run(command, &mut output, &mut observations);
    match outcome {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            drop(std::io::Write::write_fmt(
                &mut observations,
                format_args!("{error}\n"),
            ));
            std::process::ExitCode::FAILURE
        }
    }
}
