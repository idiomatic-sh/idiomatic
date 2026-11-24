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
#[path = "../../../../ryo/cli/benches/readers/bindings.rs"]
mod bindings;
use bindings::{context_from, rule_context};

/// These readers declare no necessary identifier spelling and no extra walk columns.
/// Their kind names, field names and filled relations come from the linked parser.
fn declaration() -> first::Language {
    first::Language {
        fact_family: |_| pair_summary::cost::Family::Fact,
        walk_facts: &[],
        host_len: finish::host::host_len,
        facts: Vec::new(),
        spelled: Vec::new(),
        text_kinds: Vec::new(),
        file_sets: Vec::new(),
        is_identifier: |_| false,
    }
}

#[path = "../bench.rs"]
mod project_bench;

pub use project_bench::{Live, LiveReport, capture, capture_then, capture_one, print_measurement};
