//! The asymptote command line.

/// The allocator on Windows: mimalloc, as ruff and uv ship it.
#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// The allocator wherever jemalloc builds: jemalloc, as ruff and uv ship it.
#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "openbsd"),
    not(target_os = "aix"),
    not(target_os = "android"),
    any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "powerpc64",
        target_arch = "riscv64"
    )
))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() -> std::process::ExitCode {
    stop_on_signals();
    // The edge reads where it runs and what binary it is: the start of the search for a product, and the stamp the
    // cache is kept under, so one build never reuses another's runs.
    let start = std::env::current_dir()
        .map(|directory| directory.display().to_string())
        .unwrap_or_default();
    let build = std::env::current_exe()
        .and_then(std::fs::metadata)
        .map_or_else(
            |_| env!("CARGO_PKG_VERSION").to_owned(),
            |meta| {
                // A program the system cannot date has no time: its stamp reads 0 and its version and size still part
                // it from another build.
                let modified = meta.modified().map_or(0, |time| {
                    time.duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |since| since.as_nanos())
                });
                // ignore: idiomatic.arch/digest-not-stamp-alone - the key is of the running program, not of a file it reads: its version, size and time part one build from another, and digesting the binary would read it whole on every command
                format!("{}:{}:{modified}", env!("CARGO_PKG_VERSION"), meta.len())
            },
        );
    asymptote_cli::resources::run(&start, &build, on_signal)
}

/// What a SIGINT or SIGTERM runs before the command ends: stopping the tools each composition's connection notes.
static ON_SIGNAL: std::sync::Mutex<Vec<Box<dyn Fn() + Send>>> = std::sync::Mutex::new(Vec::new());

/// Runs `stop` when a SIGINT or SIGTERM ends the command, before it ends (`signalled`): handed to the command tree,
/// which hands it to the command that runs tools.
fn on_signal(stop: Box<dyn Fn() + Send>) {
    ON_SIGNAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(stop);
}

/// Runs what `on_signal` was given: called when a SIGINT or SIGTERM ends the command, before it ends.
#[cfg(unix)]
fn signalled() {
    for stop in ON_SIGNAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .iter()
    {
        stop();
    }
}

/// Makes SIGINT and SIGTERM end the command at once with every tool it runs: both are blocked before any other
/// thread starts, so every thread inherits the mask, and taken on a thread of their own, which stops the running
/// tools (`signalled`) and raises the signal again, unblocked, so the command ends as the signal ends it.
/// No handler is installed; a tool starts with no signal blocked.
fn stop_on_signals() {
    #[cfg(unix)]
    {
        use nix::sys::signal::{SigSet, Signal};
        let mut set = SigSet::empty();
        set.add(Signal::SIGINT);
        set.add(Signal::SIGTERM);
        if set.thread_block().is_err() {
            return;
        }
        drop(std::thread::spawn(move || {
            let Ok(signal) = set.wait() else {
                return;
            };
            signalled();
            set.thread_unblock().unwrap_or_default();
            nix::sys::signal::raise(signal).unwrap_or_default();
            // Still here: the signal was ignored when the command started (a background job of a script ignores
            // SIGINT), so the command ends with the code a shell gives a process the signal ended.
            ended_by(signal);
        }));
    }
}

/// Ends the command with the code a shell gives a process `signal` ended: 128 and the signal's number, 130 for
/// SIGINT and 143 for SIGTERM, the two it takes.
#[cfg(unix)]
#[expect(
    clippy::exit,
    reason = "a signal that ends the command ends it from the thread that took it, as the signal itself would have"
)]
fn ended_by(signal: nix::sys::signal::Signal) {
    std::process::exit(if signal == nix::sys::signal::Signal::SIGINT {
        130
    } else {
        143
    });
}
