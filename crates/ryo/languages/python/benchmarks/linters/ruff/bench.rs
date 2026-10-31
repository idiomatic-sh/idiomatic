//! One executable composes the shared benchmark and the Python SDK provider.
mod adapter;
mod composition;
mod constants;
mod schemas;
mod settings;

fn main() -> std::process::ExitCode {
    use std::io::Write;
    let mut output = std::io::stdout().lock();
    let mut observations = std::io::stderr().lock();
    match composition::run(&mut output, &mut observations) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            drop(writeln!(observations, "{error}"));
            std::process::ExitCode::FAILURE
        }
    }
}
