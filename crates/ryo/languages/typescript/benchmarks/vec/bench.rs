//! Cold, single-threaded vector benchmark over the TypeScript reader.
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
    // Resolved names can be escaped or implicit. Only actual syntax text entails a written word.
    first::Language {
        facts: Vec::new(),
        spelled: Vec::new(),
        word_kinds: [
            "identifier-token",
            "jsx-identifier-token",
            "private-identifier-token",
            "keyword",
        ]
        .iter()
        .map(|s| (*s).to_owned())
        .collect(),
        is_identifier: oxc_syntax::identifier::is_identifier_name,
    }
}

fn main() {
    reader_cli::main(ryo_typescript::linked, declaration);
}
