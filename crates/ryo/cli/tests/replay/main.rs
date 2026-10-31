//! The replay suite: every test that runs the recorded rules over ruff is a module of one test binary, so the support they share is
//! compiled once, every part of it is used, and the binary is linked once.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a replay test that cannot run its recording has nothing to compare: the panic is the failure of the test"
)]
#![expect(
    clippy::print_stderr,
    reason = "the replay tells the person running it each rule's verdict and what it could not feed, on stderr, which the test harness shows beside the failure"
)]

/// The allocator the command ships, so what a replay measures of memory (the warm peak) is what a user's run allocates: on
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

#[path = "../fixtures/replay/answers.rs"]
mod answers;
#[path = "../fixtures/replay/base.rs"]
mod base;
#[path = "../fixtures/replay/counting.rs"]
mod counting;
#[cfg(feature = "python")]
#[path = "../fixtures/replay/feature_contracts.rs"]
mod feature_contracts;
#[path = "../fixtures/replay/ruff.rs"]
mod ruff;
#[path = "../fixtures/replay/stores.rs"]
mod stores;
#[path = "../fixtures/replay/tables.rs"]
mod tables;
#[path = "../fixtures/replay/tools.rs"]
mod tools;

mod behavior;
mod cache;
mod coverage;
mod damage;
mod filesystem;
mod finish;
mod frontier;
mod governing;
mod interface;
mod linear;
mod linking;
mod lowering;
mod manifests;
#[cfg(any(feature = "javascript", feature = "typescript"))]
mod manifests_bun;
mod manifests_cargo;
#[cfg(feature = "go")]
mod manifests_go;
#[cfg(any(feature = "javascript", feature = "typescript"))]
mod manifests_npm;
#[cfg(any(feature = "javascript", feature = "typescript"))]
mod manifests_pnpm;
#[cfg(feature = "python")]
mod manifests_uv;
mod peak;
mod rule_fixtures;
mod sdk;
mod snapshot;

use std::sync::{PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

use ryo_core::inbound::use_cases::lint::types::Rule;
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::runs::types::RuleId;

/// A rule written as Tastemaker writes it, as ryo is asked to run it.
#[must_use]
pub fn rule_from_query(id: &str, query: &str, include: &[String]) -> Rule {
    Rule {
        id: RuleId(id.into()),
        query: QueryText(query.into()),
        parameters: Vec::new(),
        include: include
            .iter()
            .map(|glob| Glob(glob.as_str().into()))
            .collect(),
        exclude: Vec::new(),
    }
}

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
