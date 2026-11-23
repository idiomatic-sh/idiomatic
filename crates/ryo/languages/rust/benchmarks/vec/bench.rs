//! Cold, single-threaded vector benchmark over the Rust reader.
#![allow(
    dead_code,
    reason = "shared benchmark modules also expose separate diagnostic entry points"
)]
#[path = "../../../../crates/engine/benches/vec/alloc_count.rs"]
mod alloc_count;
#[path = "../../../../crates/engine/benches/vec/file_skip.rs"]
mod file_skip;
#[path = "../../../../crates/engine/benches/vec/finish.rs"]
mod finish;
#[path = "../../../../crates/engine/benches/vec/finisher.rs"]
mod finisher;
#[path = "../../../../crates/engine/benches/vec/first.rs"]
mod first;
use first::Vocab;
#[path = "../../../../crates/engine/benches/vec/pair_summary.rs"]
mod pair_summary;
#[path = "../../../../crates/engine/benches/vec/pair_skip.rs"]
mod pair_skip;
#[path = "../../../../crates/engine/benches/vec/lower.rs"]
mod lower;
#[path = "../../../../crates/engine/benches/vec/reader.rs"]
mod reader;
#[path = "../../../../crates/engine/benches/vec/reader_run.rs"]
mod reader_run;
#[path = "../../../../crates/engine/benches/vec/spec.rs"]
mod spec;

#[path = "../../../../crates/engine/benches/vec/reader_cli.rs"]
mod reader_cli;
use reader_cli::{alloc_counts, flag, rule_context, thread_cpu};

fn declaration() -> first::Language {
    first::Language {
        facts: Vec::new(),
        spelled: Vec::new(),
        text_kinds: Vec::new(),
        file_sets: Vec::new(),
        is_identifier: |word| {
            ra_ap_syntax::utils::is_identifier(word, ra_ap_parser::Edition::CURRENT)
        },
    }
}

fn main() {
    reader_cli::main(ryo_rust::linked, declaration);
}
