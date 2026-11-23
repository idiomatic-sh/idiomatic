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
use first::{Vocab, span_bytes};
#[path = "../../../../crates/engine/benches/vec/pair_checks.rs"]
mod pair_checks;
#[path = "../../../../crates/engine/benches/vec/summary_schedule.rs"]
mod summary_schedule;
#[path = "../../../../crates/engine/benches/vec/summary_values.rs"]
mod summary_values;
#[path = "../../../../crates/engine/benches/vec/summary_proofs.rs"]
mod summary_proofs;
#[path = "../../../../crates/engine/benches/vec/summary_rules.rs"]
mod summary_rules;
#[path = "../../../../crates/engine/benches/vec/summary_literals.rs"]
mod summary_literals;
#[path = "../../../../crates/engine/benches/vec/host_columns.rs"]
mod host_columns;
#[path = "../../../../crates/engine/benches/vec/glob_pattern.rs"]
mod glob_pattern;
use glob_pattern::glob_pattern;
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
        fact_family: reader_run::fact_family,
        walk_facts: &[],
        host_len: finish::host::host_len,
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
