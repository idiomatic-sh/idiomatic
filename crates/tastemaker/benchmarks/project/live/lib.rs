//! The pinned product calls the shared session in-process, through its typed SDK edge.
#![allow(
    dead_code,
    reason = "shared benchmark modules also expose separate diagnostic entry points"
)]
extern crate ryo_sdk as ryo;
#[path = "../../../../ryo/crates/engine/benches/vec/alloc_count.rs"]
mod alloc_count;
#[path = "../../../../ryo/crates/engine/benches/vec/file_skip.rs"]
mod file_skip;
#[path = "../../../../ryo/crates/engine/benches/vec/finish.rs"]
mod finish;
#[path = "../../../../ryo/crates/engine/benches/vec/finisher.rs"]
mod finisher;
#[path = "../../../../ryo/crates/engine/benches/vec/first.rs"]
mod first;
use first::{Vocab, span_bytes};
#[path = "../../../../ryo/crates/engine/benches/vec/glob_pattern.rs"]
mod glob_pattern;
#[path = "../../../../ryo/crates/engine/benches/vec/host_columns.rs"]
mod host_columns;
#[path = "../../../../ryo/crates/engine/benches/vec/pair_checks.rs"]
mod pair_checks;
use glob_pattern::glob_pattern;
#[path = "../../../../ryo/crates/engine/benches/vec/check_session.rs"]
mod check_session;
#[path = "../../../../ryo/crates/engine/benches/vec/lower.rs"]
mod lower;
#[path = "../../../../ryo/crates/engine/benches/vec/pair_skip.rs"]
mod pair_skip;
#[path = "../../../../ryo/crates/engine/benches/vec/pair_summary.rs"]
mod pair_summary;
#[path = "../../../../ryo/crates/engine/benches/vec/reader.rs"]
mod reader;
#[path = "../../../../ryo/crates/engine/benches/vec/reader_run.rs"]
mod reader_run;
#[path = "../../../../ryo/crates/engine/benches/vec/spec.rs"]
mod spec;

#[path = "../../../../ryo/crates/engine/benches/vec/reader_cli.rs"]
mod reader_cli;
use reader_cli::{alloc_counts, flag, thread_cpu};
#[path = "../../../../ryo/cli/benches/readers/json_input.rs"]
mod json_input;
#[path = "../../../../ryo/cli/benches/readers/json_output.rs"]
mod json_output;
use json_input::rule_context;



#[path = "../bench.rs"]
mod project_bench;

pub use project_bench::{link_executor, measure_dispatch, configure_bench_check, configured_workers, HostPhase, host_snapshot, host_wall, capture, capture_then, capture_retained, capture_one, print_measurement};

// A pair of private executable aliases selects the gate's receipt collection.
// The check and the SDK have no runtime switch for their execution path.
fn proof_alias() -> Option<&'static std::ffi::OsStr> {
    use std::sync::OnceLock;
    static NAME: OnceLock<Option<std::ffi::OsString>> = OnceLock::new();
    NAME.get_or_init(|| {
        let arg = std::env::args_os().next()?;
        std::path::Path::new(&arg).file_name().map(ToOwned::to_owned)
    }).as_deref()
}

fn proof_enabled() -> bool {
    proof_alias().is_some_and(|name| name == "tm-sdk-serial-proof" || name == "tm-sdk-rounds-proof")
}

pub fn serial_host() -> bool {
    proof_alias().is_some_and(|name| name == "tm-sdk-serial" || name == "tm-sdk-serial-proof")
}
