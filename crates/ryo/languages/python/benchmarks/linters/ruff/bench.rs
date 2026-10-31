//! One executable composes the shared benchmark and the Python SDK provider.
mod adapter;
mod composition;
mod constants;
mod schemas;
mod settings;
fn main() {
    if let Err(error) = composition::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
