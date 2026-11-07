//! The ryo command line.

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
    // The edge reads where it runs once, here: the directory the command line was started in.
    // A name that is not text cannot be a root, so the commands read the directory they run in.
    let here = std::env::current_dir().map_or_else(
        |_| ".".to_owned(),
        |directory| {
            directory
                .into_os_string()
                .into_string()
                .unwrap_or_else(|_| ".".to_owned())
        },
    );
    ryo_cli::resources::run(here)
}
