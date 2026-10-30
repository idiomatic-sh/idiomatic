//! The Tastemaker command line.

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
    // The edge reads where it runs and what binary it is: the start of the search for a workspace, and the stamp a plan,
    // a verdict and a proof are kept under, so one build never serves another's. The stamp hashes the executable's
    // bytes; an executable that cannot be read is a build of its own for this process alone.
    let start = std::env::current_dir()
        .map(|directory| directory.display().to_string())
        .unwrap_or_default();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let version =
        tastemaker_core::domain::packages::types::VersionText(env!("CARGO_PKG_VERSION").into());
    let build = std::env::current_exe()
        .and_then(std::fs::File::open)
        .and_then(|mut file| {
            tastemaker_core::domain::settings::service::build(
                &version,
                &mut tastemaker_core::domain::settings::types::Executable(&mut file),
            )
        })
        .unwrap_or_else(|_| {
            tastemaker_core::domain::settings::service::unread(
                &version,
                tastemaker_core::domain::settings::types::ProcessStart {
                    id: tastemaker_core::domain::settings::types::ProcessId(std::process::id()),
                    started: tastemaker_core::domain::settings::types::StartedAt(now),
                },
            )
        });
    tastemaker_cli::resources::run(&start, &build.0)
}
