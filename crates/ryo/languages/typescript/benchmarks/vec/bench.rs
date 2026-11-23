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
use first::{Vocab, span_bytes};
#[path = "../../../../crates/engine/benches/vec/pair_checks.rs"]
mod pair_checks;
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

fn thread_cpu() -> f64 {
    thread_local! { static START: cpu_time::ThreadTime = cpu_time::ThreadTime::now(); }
    START.with(|start| start.elapsed().as_secs_f64())
}

fn flag(args: &[&str], name: &str) -> Option<String> {
    args.iter()
        .position(|&a| a == name)
        .map(|at| args[at + 1].to_owned())
}

fn alloc_counts() -> [usize; 4] {
    let [blocks, bytes, peak] = alloc_count::counts();
    [blocks as usize, bytes as usize, 0, peak as usize]
}

fn rule_context(request: Option<&str>) -> ryo_engine::domain::plan_lowering::types::RuleContext {
    if let Some(path) = request {
        let context: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).expect("request")).expect("JSON request");
        assert!(
            context["rule_context"].is_null() && context["rule_observations"].is_null(),
            "this host provides no named context tables"
        );
    }
    Default::default()
}

fn main() {
    let start = thread_cpu();
    let _allocator = alloc_count::start();
    let args: Vec<_> = std::env::args()
        .skip(1)
        .filter(|arg| arg != "--bench")
        .collect();
    let [pack, request, findings] = args.as_slice() else {
        panic!("usage: vec-ts PACK REQUEST FINDINGS");
    };
    // Resolved names can be escaped or implicit. Only actual syntax text entails a written word.
    let language = first::Language {
        fact_family: |_| pair_summary::cost::Family::Fact,
        walk_facts: &[],
        host_len: finish::host::host_len,
        facts: Vec::new(),
        spelled: Vec::new(),
        text_kinds: Vec::new(),
        file_sets: Vec::new(),
        is_identifier: oxc_syntax::identifier::is_identifier_name,
    };
    if !reader_run::run(
        start,
        ryo_typescript::linked(),
        language,
        pack,
        request,
        findings,
    ) {
        std::process::exit(1);
    }
}
