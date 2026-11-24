//! Cold, single-threaded vector benchmark over a linked SDK reader.
#![allow(
    dead_code,
    reason = "shared benchmark modules also expose separate diagnostic entry points"
)]
extern crate ryo_sdk as ryo;
#[path = "../../../crates/engine/benches/vec/alloc_count.rs"]
mod alloc_count;
#[path = "../../../crates/engine/benches/vec/file_skip.rs"]
mod file_skip;
#[path = "../../../crates/engine/benches/vec/finish.rs"]
mod finish;
#[path = "../../../crates/engine/benches/vec/finisher.rs"]
mod finisher;
#[path = "../../../crates/engine/benches/vec/first.rs"]
mod first;
use first::{Vocab, span_bytes};
#[path = "../../../crates/engine/benches/vec/glob_pattern.rs"]
mod glob_pattern;
#[path = "../../../crates/engine/benches/vec/host_columns.rs"]
mod host_columns;
#[path = "../../../crates/engine/benches/vec/pair_checks.rs"]
mod pair_checks;
use glob_pattern::glob_pattern;
#[path = "../../../crates/engine/benches/vec/lower.rs"]
mod lower;
#[path = "../../../crates/engine/benches/vec/pair_skip.rs"]
mod pair_skip;
#[path = "../../../crates/engine/benches/vec/pair_summary.rs"]
mod pair_summary;
#[path = "../../../crates/engine/benches/vec/reader.rs"]
mod reader;
#[path = "../../../crates/engine/benches/vec/reader_run.rs"]
mod reader_run;
#[path = "../../../crates/engine/benches/vec/check_session.rs"]
mod check_session;
#[path = "../../../crates/engine/benches/vec/spec.rs"]
mod spec;

#[path = "../../../crates/engine/benches/vec/reader_cli.rs"]
mod reader_cli;
use reader_cli::{alloc_counts, flag, thread_cpu};
mod json_input;
mod json_output;
use json_input::rule_context;

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

#[path = "../../../../tastemaker/benchmarks/project/bench.rs"]
mod project_bench;
mod mixed;
mod shipping;

fn main() {
    // Composition only. Each provider owns its parser, facts and registration name.
    let readers = [
        ryo_css::linked,
        ryo_go::linked,
        ryo_html::linked,
        ryo_json::linked,
        ryo_markdown::linked,
        ryo_toml::linked,
    ];
    let start = thread_cpu();
    let _allocator = alloc_count::start();
    let args: Vec<_> = std::env::args()
        .skip(1)
        .filter(|arg| arg != "--bench")
        .collect();
    if args.as_slice() == ["--project"] {
        project_bench::replay();
        return;
    }
    let languages = || {
        let mut langs: Vec<_> = readers.iter().map(|link| link()).collect();
        langs.extend([ryo_python::linked(), ryo_rust::linked(), ryo_typescript::linked()]);
        langs
    };
    if let [mode, manifest, output] = args.as_slice()
        && mode == "--tm-batch"
    {
        project_bench::batch(languages(), manifest, output);
        return;
    }
    if let [manifest, _, output] = args.as_slice() {
        if let Err(reason) = project_bench::packets(languages(), manifest, output) {
            eprintln!("{reason}");
            std::process::exit(1);
        }
        return;
    }
    if let [mode, pack, request, output] = args.as_slice()
        && (mode == "--shipping" || mode == "--mixed")
    {
        let langs = languages();
        if mode == "--mixed" {
            // Scenario inputs carry their numbered inventory and one standalone query.
            // A pack request keeps its original multi-rule and fragment semantics.
            if !std::path::Path::new(pack).join("fragments").exists()
                && let Some(input) = json_input::scenario(request)
            {
                if let Err(reason) = project_bench::one(langs, pack, input, output) {
                    eprintln!("{reason}");
                    std::process::exit(1);
                }
            } else if !mixed::run(langs, pack, request, output) {
                std::process::exit(1);
            }
            return;
        }
        shipping::run(
            ryo::linter::Languages(langs),
            pack.as_ref(),
            request.as_ref(),
            output.as_ref(),
        );
        return;
    }
    let [name, pack, request, output] = args.as_slice() else {
        panic!("usage: vec-readers READER PACK REQUEST OUTPUT");
    };
    let reader = readers
        .into_iter()
        .map(|link| link())
        .find(|r| &*r.name().0 == name)
        .expect("requested reader must be linked");
    if !reader_run::run_files::<true>(start, reader, declaration(), pack, request, reader_run::Output::Path(output), None) {
        std::process::exit(1);
    }
}
