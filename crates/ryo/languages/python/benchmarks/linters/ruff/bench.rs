//! One executable composes the shared benchmark and its linter: the harness records any profile, then the composition runs the command.
mod adapter;
mod composition;
mod constants;
mod frontend_projection;
mod host;
mod host_policy;
mod schemas;
mod settings;

fn main() -> std::process::ExitCode {
    let mut output = std::io::stdout().lock();
    let mut observations = std::io::stderr().lock();
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
