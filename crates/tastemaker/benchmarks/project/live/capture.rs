//! The bench entry points: replay a closed capture through the executor with the readers linked into this binary, write one fixture's rows,
//! and end the process the way every image does.
use crate::linked::Linked;
use ryo_executor::check_session::replay;
use ryo_sdk::linter::Languages;

/// The linked languages as the executor's sources.
fn readers(linked: Languages) -> Vec<Linked> {
    linked.0.into_iter().map(Linked::from).collect()
}

/// Replays a closed capture with exactly the parsers linked into this binary.
pub fn capture(linked: Languages, manifest: &str, output: &str, workers: Option<std::num::NonZeroUsize>) {
    capture_then(linked, manifest, output, workers, || {});
}

/// As `capture`, running `completed` once the outputs are written.
pub fn capture_then(linked: Languages, manifest: &str, output: &str, workers: Option<std::num::NonZeroUsize>, completed: impl FnOnce()) {
    replay::batch_mode_with_workers(&readers(linked), manifest, output, None, workers, completed);
}

/// As `capture`, keeping the retained store under `root`.
pub fn capture_retained(linked: Languages, manifest: &str, output: &str, root: &str) {
    replay::batch_mode(&readers(linked), manifest, output, Some(std::path::Path::new(root)));
}

/// Writes one fixture's complete rows, through the same reader and executor as the check.
///
/// # Errors
///
/// The request file cannot be read or decoded, or the replay refuses it.
pub fn capture_one(linked: Languages, pack: &str, input: &str, out: &str) -> Result<(), String> {
    let bytes = std::fs::read(input).map_err(|e| e.to_string())?;
    let req = ryo_executor::request_input::decode(&bytes).map_err(|e| e.to_string())?;
    replay::one_source(&readers(linked), pack, req, out).map_err(|refusal| refusal.to_string())
}

/// Every image's last act, once its outputs are written: flush, report the outcome as the product's main does, and exit without
/// dropping what the session still holds (no destructors: the product's own exit).
pub fn exit_process(outcome: Result<(), Box<dyn std::error::Error>>) -> ! {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let code = match outcome {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("Error: {error:?}");
            1
        }
    };
    let _ = std::io::stderr().flush();
    std::process::exit(code)
}
