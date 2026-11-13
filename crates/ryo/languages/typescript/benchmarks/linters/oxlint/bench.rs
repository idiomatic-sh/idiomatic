//! One executable composes the shared benchmark and the TypeScript SDK provider.
mod adapter;
mod composition;
mod constants;
mod frontend_projection;
mod schemas;
mod settings;

/// The allocator on Windows: mimalloc, as the command line, ruff and uv ship it.
#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// The allocator wherever jemalloc builds: jemalloc, as the command line, ruff and uv ship it.
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
    let mut output = std::io::stdout().lock();
    let mut observations = std::io::stderr().lock();
    match composition::run(&mut output, &mut observations) {
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
