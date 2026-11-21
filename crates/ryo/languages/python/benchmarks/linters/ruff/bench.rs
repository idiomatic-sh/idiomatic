//! One executable composes the shared benchmark and its linter: the harness records any profile, then the composition runs the command.
mod adapter;
mod composition;
mod constants;
mod frontend_projection;
mod schemas;
mod settings;

fn main() -> std::process::ExitCode {
    let mut output = std::io::stdout().lock();
    // Not locked for the run: the new engine's scan workers write their notices to stderr themselves (ryo-fmx3.1).
    let mut observations = std::io::stderr();
    let outcome = composition::profile::command()
        .and_then(|command| composition::run(command, &mut output, &mut observations));
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
