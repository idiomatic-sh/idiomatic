//! The replay suite: every test that runs rules with the real engine over a tree of its own is a module of one test binary, so the support
//! they share is compiled once, every part of it is used, and the binary is linked once.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a replay test that cannot run its rules has nothing to compare: the panic is the failure of the test"
)]
#![expect(
    clippy::print_stderr,
    reason = "the replay tells the person running it each rule's verdict and what it could not feed, on stderr, which the test harness shows beside the failure"
)]

/// The allocator the command ships, so a replay allocates as a user's run does: on
/// Windows mimalloc, as ruff and uv ship it.
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

#[path = "../fixtures/replay/base.rs"]
mod base;
#[path = "../fixtures/replay/cargo.rs"]
mod cargo;
#[path = "../fixtures/replay/copy.rs"]
mod copy;
#[cfg(any(feature = "go", feature = "typescript", feature = "python"))]
#[path = "../fixtures/replay/ecosystems.rs"]
mod ecosystems;
#[path = "../fixtures/replay/fetch.rs"]
mod fetch;
#[path = "../fixtures/replay/spans.rs"]
mod spans;
#[path = "../fixtures/replay/tools.rs"]
mod tools;
#[path = "../fixtures/replay/unbounded.rs"]
mod unbounded;

mod cache;
mod coverage;
mod damage;
mod filesystem;
mod finish;
mod governing;
mod languages;
mod linking;
mod lowering;
mod manifests;
#[cfg(feature = "typescript")]
mod manifests_bun;
mod manifests_cargo;
#[cfg(feature = "go")]
mod manifests_go;
#[cfg(feature = "typescript")]
mod manifests_npm;
#[cfg(feature = "typescript")]
mod manifests_pnpm;
#[cfg(feature = "python")]
mod manifests_uv;
mod rule_fixtures;
mod scenarios;
mod sdk;
mod snapshot;
mod walked;

use std::sync::{PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Who is running: the CPU of a process is the CPU of everything it runs, so the tests that time the process run alone and every
/// other test beside the others.
static RUNNING: RwLock<()> = RwLock::new(());

/// Taken by a test that may run beside the others.
pub fn beside() -> RwLockReadGuard<'static, ()> {
    RUNNING.read().unwrap_or_else(PoisonError::into_inner)
}

/// Taken by a test that measures what the process costs: nothing else runs while it is held.
pub fn alone() -> RwLockWriteGuard<'static, ()> {
    RUNNING.write().unwrap_or_else(PoisonError::into_inner)
}
