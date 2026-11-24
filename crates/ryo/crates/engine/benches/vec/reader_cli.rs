//! Process entry point shared by providers using the SDK file reader.
use super::{alloc_count, first, reader_run};

pub(super) fn thread_cpu() -> f64 {
    thread_local! { static START: cpu_time::ThreadTime = cpu_time::ThreadTime::now(); }
    START.with(|start| start.elapsed().as_secs_f64())
}

pub(super) fn flag(args: &[&str], name: &str) -> Option<String> {
    args.iter()
        .position(|&a| a == name)
        .map(|at| args[at + 1].to_owned())
}

pub(super) fn alloc_counts() -> [usize; 4] {
    let [blocks, bytes, peak] = alloc_count::counts();
    [blocks as usize, bytes as usize, 0, peak as usize]
}

pub(super) fn rule_context(
    request: Option<&str>,
) -> ryo_engine::domain::plan_lowering::types::RuleContext {
    if let Some(path) = request {
        let context: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).expect("request")).expect("JSON request");
        return context_from(&context);
    }
    Default::default()
}

pub(super) fn context_from(context: &serde_json::Value) -> ryo_engine::domain::plan_lowering::types::RuleContext {
        assert!(
            context["rule_context"].is_null() && context["rule_observations"].is_null(),
            "this host provides no named context tables"
        );
    Default::default()
}

pub(super) fn main(link: fn() -> ryo::languages::Language, declaration: fn() -> first::Language) {
    let start = thread_cpu();
    let _allocator = alloc_count::start();
    let args: Vec<_> = std::env::args()
        .skip(1)
        .filter(|arg| arg != "--bench")
        .collect();
    let [pack, request, findings] = args.as_slice() else {
        panic!("usage: vector benchmark: PACK REQUEST FINDINGS");
    };
    if !reader_run::run(start, link(), declaration(), pack, request, findings) {
        std::process::exit(1);
    }
}
