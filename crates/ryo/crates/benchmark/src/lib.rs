#![cfg_attr(doc, doc = include_str!("../README.md"))]
#![cfg_attr(not(doc), doc = env!("CARGO_PKG_DESCRIPTION"))]

/// The allocator on Windows: mimalloc, as the command line, ruff and uv ship it.
#[cfg(all(target_os = "windows", not(feature = "heap-profile")))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// The allocator wherever jemalloc builds: jemalloc, as the command line, ruff and uv ship it.
#[cfg(all(
    not(feature = "heap-profile"),
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

pub mod application;
pub mod domain;
pub mod inbound;
pub mod outbound;


