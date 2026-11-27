//! The bench `tm`: the product's main with the bench entry points in front of it, a wrapper and not a patch. `tm` ends the process after the command (flush, exit without
//! destructors: the product's exit).
//!
//! Bench entry points: `--mixed PACK REQUEST OUT`, `--tm-batch MANIFEST OUT [--workers N]`, `--tm-batch-retained ROOT MANIFEST OUT`.
//! Bench environment for the check (the product's command line has no such flag): `TM_BENCH_WORKERS` (the pool size, set before the
//! command builds its pool). The SDK lists a walked corpus itself now: there is no file-order hook to set.
use std::process::ExitCode;

fn start_and_build() -> (String, String) {
    let start = std::env::current_dir().map(|directory| directory.display().to_string()).unwrap_or_default();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_nanos());
    let version = tastemaker_values::domain::packages::types::VersionText(env!("CARGO_PKG_VERSION").into());
    let build = std::env::current_exe()
        .and_then(std::fs::File::open)
        .and_then(|mut file| {
            tastemaker_values::domain::settings::service::build(&version, &mut tastemaker_values::domain::settings::types::Executable(&mut file))
        })
        .unwrap_or_else(|_| {
            tastemaker_values::domain::settings::service::unread(
                &version,
                tastemaker_values::domain::settings::types::ProcessStart {
                    id: tastemaker_values::domain::settings::types::ProcessId(std::process::id()),
                    started: tastemaker_values::domain::settings::types::StartedAt(now),
                },
            )
        });
    (start, build.0.to_string())
}

pub fn run() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let mode = arguments.next();
    match mode.as_deref() {
        Some("--mixed") => {
            let (pack, input, output) = (arguments.next().expect("pack"), arguments.next().expect("input"), arguments.next().expect("output"));
            if let Err(reason) = tm_project_session::capture_one(tm_project_session::languages(), &pack, &input, &output) {
                eprintln!("{reason}");
                return ExitCode::FAILURE;
            }
            return ExitCode::SUCCESS;
        }
        Some("--tm-batch-retained") => {
            let (root, manifest, output) = (arguments.next().expect("store root"), arguments.next().expect("manifest"), arguments.next().expect("output"));
            tm_project_session::capture_retained(tm_project_session::languages(), &manifest, &output, &root);
            return ExitCode::SUCCESS;
        }
        Some("--tm-batch") => {
            let manifest = arguments.next().expect("manifest");
            let output = arguments.next().expect("output");
            let workers = arguments.next().map(|flag| {
                assert_eq!(flag, "--workers");
                arguments.next().expect("worker count").parse().expect("positive worker count")
            });
            assert!(arguments.next().is_none(), "unexpected batch argument");
            // The one-worker replay enters from main: let the linked jobs run there so the phase clock accounts for the whole run.
            if workers.or_else(|| std::thread::available_parallelism().ok()).is_none_or(|n: std::num::NonZeroUsize| n.get() == 1) {
                rayon::ThreadPoolBuilder::new().num_threads(1).use_current_thread().build_global().expect("initialize one-worker replay");
            }
            tm_project_session::capture_then(tm_project_session::languages(), &manifest, &output, workers, || {
                std::io::Write::flush(&mut std::io::stdout()).expect("flush capture log");
                std::process::exit(0);
            });
            return ExitCode::SUCCESS;
        }
        _ => {}
    }
    if let Some(count) = std::env::var("TM_BENCH_WORKERS").ok().and_then(|text| text.parse::<usize>().ok()) {
        tastemaker_core::outbound::scheduler::types::WorkerCount::configure(count);
    }
    let (start, build) = start_and_build();
    let code = tastemaker_cli::resources::run(&start, &build);
    ryo_sdk::linter::print_measurement();
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    std::process::exit(numeric(code))
}

/// The number an `ExitCode` carries: the standard library has no getter for it, but its debug form names it (`ExitCode(unix_exit_status(2))`).
fn numeric(code: ExitCode) -> i32 {
    format!("{code:?}").chars().filter(char::is_ascii_digit).collect::<String>().parse().unwrap_or(1)
}
