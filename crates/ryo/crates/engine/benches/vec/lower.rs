//! THROWAWAY edge (bead ryo-fmx3.1): a pack's rules from their RyoQL text to the finish plan, in Rust: the query language's one
//! parser reads every rule and fragment, the engine's `plan_lowering` lowers them and runs its passes, the JSON plan codec
//! writes each stage.
//!
//! `vec lower <pack> <out prefix> [--repeat K]`: writes `<out prefix>-u.json` (expanded), `-b1.json` (semijoins), `-b2.json`
//! (counts) and `.json` (axes), the names the Python chain (`scripts6h/plan.sh`) writes; prints each phase's thread CPU
//! (user + system) and the peak resident set after it. With `--repeat K` the phases run K times in one process, and each
//! repeat's CPU is printed. With `--measure` no snapshot is encoded or written.
//! `--request F`: the run's request, whose rule context (context tables, rule observations) the lowering reads.
//! `vec stage <in.json> <stage> <out.json>`: decodes a plan, runs the one pass `stage` (`semijoins`, `counts`, `axes`) adds,
//! writes it; `stage` `none` writes the decoded plan back (a round trip).

use ryo_engine::domain::plan_lowering::service::{lowered, mark_through, passed};
use ryo_engine::domain::plan_lowering::types::{
    PackPrograms, PackRule, PlanningStage, RuleContext, RuleProgram, UnreadQuery,
};
use ryo_engine::outbound::plan_codec::factory::json;
use ryo_engine::outbound::plan_codec::interface::PlanCodecPort as _;
use ryo_query_language::outbound::syntax::factory::winnow;
use ryo_query_language::outbound::syntax::interface::SyntaxPort as _;
use ryo_values::domain::finish_plans::types::{FinishPlan, FinishStep};
use ryo_values::domain::program::types::ParsedModules;
use ryo_values::domain::query_tokens::types::{QuerySourceId, QueryView};
use ryo_values::domain::runs::types::RuleId;
use std::collections::BTreeMap;
use std::path::Path;
use super::rule_context;
use std::sync::Arc;

fn peak_kb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|st| st.lines().find(|l| l.starts_with("VmHWM:")).and_then(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok()))
        .unwrap_or(0)
}

/// The pack's texts: each rule's query (rules in byte order, `None` without one) and each fragment by file stem.
struct PackTexts {
    rules: Vec<(String, Option<String>)>,
    fragments: Vec<(String, String)>,
}

fn read_texts(pack: &Path) -> PackTexts {
    let mut ids: Vec<String> = std::fs::read_dir(pack.join("rules"))
        .expect("rules dir")
        .map(|e| e.expect("entry").file_name().into_string().expect("utf-8 rule id"))
        .collect();
    ids.sort();
    let rules = ids
        .into_iter()
        .map(|id| {
            let text = std::fs::read_to_string(pack.join("rules").join(&id).join("query.ryoql")).ok();
            (id, text)
        })
        .collect();
    let mut fragments: Vec<(String, String)> = std::fs::read_dir(pack.join("fragments"))
        .map(|d| {
            d.filter_map(|e| {
                let path = e.expect("entry").path();
                let stem = path.file_stem()?.to_str()?.to_owned();
                (path.extension()? == "ryoql").then(|| (stem, std::fs::read_to_string(&path).expect("fragment")))
            })
            .collect()
        })
        .unwrap_or_default();
    fragments.sort();
    PackTexts { rules, fragments }
}

fn parsed(texts: &PackTexts, context: RuleContext) -> PackPrograms {
    let syntax = winnow();
    let mut fragments = BTreeMap::new();
    for (stem, text) in &texts.fragments {
        let id = QuerySourceId(stem.as_str().into());
        match syntax.read_fragment(&id, QueryView(text)) {
            Ok(fragment) => {
                let _previous = fragments.insert(id, fragment);
            }
            Err(e) => println!("unread fragment {stem}: {e}"),
        }
    }
    let rules = texts
        .rules
        .iter()
        .map(|(id, text)| PackRule {
            rule: RuleId(id.as_str().into()),
            program: match text {
                None => RuleProgram::Unread(UnreadQuery("no query".into())),
                Some(text) => match syntax.read(QueryView(text)) {
                    Ok(program) => RuleProgram::Parsed(Box::new(program)),
                    Err(error) => RuleProgram::Unread(UnreadQuery(error.to_string().into())),
                },
            },
        })
        .collect();
    PackPrograms { rules, fragments: ParsedModules(Arc::new(fragments)), context }
}

pub(crate) fn run_lower(args: &[&str]) {
    let pack = Path::new(args[0]);
    let out = args[1];
    let repeat: usize = args.iter().position(|a| *a == "--repeat").map_or(1, |i| args[i + 1].parse().expect("repeat"));
    // `--measure`: no stage is written, so the peak after the passes is preparation's own.
    let measure = args.contains(&"--measure");
    let request = args.iter().position(|a| *a == "--request").map(|i| args[i + 1]);
    for round in 0..repeat {
        let write = round == 0 && !measure;
        let t0 = super::thread_cpu();
        let texts = read_texts(pack);
        let t1 = super::thread_cpu();
        let pack_programs = parsed(&texts, rule_context(request));
        let t2 = super::thread_cpu();
        let hwm_parse = peak_kb();
        let mut plan = lowered(&pack_programs, PlanningStage::Expanded);
        let t3 = super::thread_cpu();
        let expanded_bytes = write.then(|| super::finish::json_output::snapshot(&plan));
        let t4 = super::thread_cpu();
        let mut stages = Vec::new();
        let mut pass_cpu = 0.0;
        let mut encode_cpu = t4 - t3;
        for (stage, suffix) in [(PlanningStage::Semijoins, "-b1"), (PlanningStage::Counts, "-b2"), (PlanningStage::Axes, "")] {
            let a = super::thread_cpu();
            let marked = passed(&mut plan, stage);
            pass_cpu += super::thread_cpu() - a;
            if write {
                let start = super::thread_cpu();
                stages.push((suffix, marked, super::finish::json_output::snapshot(&plan)));
                encode_cpu += super::thread_cpu() - start;
            }
        }
        let hwm_passes = peak_kb();
        let hwm = hwm_passes;
        let unread = pack_programs.rules.iter().filter(|r| matches!(r.program, RuleProgram::Unread(_))).count();
        println!(
            "round {round} rules {} unread {unread} fragments {} steps {} planned {} unplanned {}",
            texts.rules.len(),
            texts.fragments.len(),
            plan.steps.len(),
            plan.rules.len(),
            plan.unplanned.len()
        );
        println!(
            "round {round} cpu read {:.4} parse {:.4} expand {:.4} passes {:.4} encode {:.4} total-prep {:.4} peak-after-parse-kb {hwm_parse} peak-after-passes-kb {hwm_passes} peak-kb {hwm}",
            t1 - t0,
            t2 - t1,
            t3 - t2,
            pass_cpu,
            encode_cpu,
            (t3 - t1) + pass_cpu,
        );
        if let Some(bytes) = expanded_bytes {
            std::fs::write(format!("{out}-u.json"), bytes).expect("write");
            for (suffix, marked, bytes) in stages {
                println!("stage {suffix} marked {marked}");
                std::fs::write(format!("{out}{suffix}.json"), bytes).expect("write");
            }
            for u in &plan.unplanned {
                println!("unplanned {} {}", u.rule.0, u.reason.0);
            }
        }
        assert!(plan.unplanned.is_empty(), "{} unplanned rules", plan.unplanned.len());
    }
}

pub(crate) fn run_stage(args: &[&str]) {
    let mut plan = super::finish::json_input::snapshot(args[0], |bytes| json().decoded(bytes));
    let marked = match args[1] {
        "none" => 0,
        "semijoins" => passed(&mut plan, PlanningStage::Semijoins),
        "counts" => passed(&mut plan, PlanningStage::Counts),
        "axes" => passed(&mut plan, PlanningStage::Axes),
        other => panic!("stage {other}"),
    };
    println!("marked {marked}");
    std::fs::write(args[2], super::finish::json_output::snapshot(&plan)).expect("write");
}

/// The pack's rules and fragments read and parsed once at start, for both halves' plans (`vec funnel ... --finish <pack dir>`).
pub(crate) struct ReadPack {
    pub(crate) typed: std::cell::RefCell<Option<std::sync::Arc<FinishPlan>>>,
    pub(crate) programs: PackPrograms,
    /// Thread CPU of reading the texts (and the request's rule context) and of parsing them.
    pub(crate) read: f64,
    pub(crate) parse: f64,
}

impl ReadPack {
    pub(super) fn lower_axes(&self) -> std::sync::Arc<FinishPlan> {
        std::sync::Arc::new(lowered(&self.programs, PlanningStage::Axes))
    }
}

pub(crate) fn read_pack(pack: &Path, context: RuleContext) -> ReadPack {
    let t0 = super::thread_cpu();
    let texts = read_texts(pack);
    let t1 = super::thread_cpu();
    let programs = parsed(&texts, context);
    let t2 = super::thread_cpu();
    drop(texts);
    ReadPack { typed: Default::default(), programs, read: t1 - t0, parse: t2 - t1 }
}

/// The second half's plan, lowered in this process from the parsed pack: expansion, the three passes, then the typed plan
/// shared with the executor and the first-half analysis.
pub(crate) fn second_half_plan(pack: &ReadPack, keep_for_first: bool) -> super::finish::Plan {
    finish_plan(pack, keep_for_first, Input::RulePack).unwrap_or_else(|reason| panic!("{reason}"))
}

/// Prepare the caller's query directly from its text. Bind typed host tables at execution.
#[allow(dead_code, reason = "file-only benchmark binaries prepare rule packs")]
pub(crate) fn query_plan(rule: &str, text: &str, context: RuleContext) -> Result<super::finish::Plan, String> {
    query_parts(rule, text, context, false, None).map(|(plan, _)| plan)
}

pub(crate) fn query_parts(rule: &str, text: &str, context: RuleContext, keep: bool, separator: Option<&str>) -> Result<(super::finish::Plan, ReadPack), String> {
    let texts = PackTexts { rules: vec![(rule.into(), Some(text.into()))], fragments: Vec::new() };
    let pack = ReadPack { typed: Default::default(), programs: parsed(&texts, context), read: 0.0, parse: 0.0 };
    let plan = finish_plan(&pack, keep, Input::Query(separator))?;
    Ok((plan, pack))
}

enum Input<'a> {
    RulePack,
    Query(Option<&'a str>),
}

fn finish_plan(pack: &ReadPack, keep_for_first: bool, input: Input<'_>) -> Result<super::finish::Plan, String> {
    let t0 = super::thread_cpu();
    let mut plan = lowered(&pack.programs, PlanningStage::Expanded);
    let t1 = super::thread_cpu();
    // no fallback: a rule the lowering cannot plan fails the run, by name and reason (never a run without it, never a
    // plan from elsewhere)
    if !plan.unplanned.is_empty() {
        let reasons: Vec<_> = plan.unplanned.iter().map(|u| format!("unplanned {} {}", u.rule.0, u.reason.0)).collect();
        return Err(reasons.join("\n"));
    }
    mark_through(&mut plan, PlanningStage::Axes);
    let t2 = super::thread_cpu();
    if let Input::Query(Some(separator)) = input {
        for step in &mut plan.steps {
            if let FinishStep::Capture(args) = step {
                args.separator.get_or_insert_with(|| ryo_values::domain::plan_parts::types::LiteralText(separator.into()));
            }
        }
    }
    let steps = plan.steps.len();
    let plan = std::sync::Arc::new(plan);
    let rule_pack = matches!(input, Input::RulePack);
    let executor_plan = super::finish::from_typed(plan.clone(), rule_pack);
    if keep_for_first {
        pack.typed.replace(Some(plan));
    }
    let t5 = super::thread_cpu();
    if rule_pack {
    println!(
        "start-up second half: steps {steps} expand {:.4} passes {:.4} to-value 0.0000 decode 0.0000 executor-plan {:.4} total {:.4} s",
        t1 - t0,
        t2 - t1,
        t5 - t2,
        t5 - t0
    );
    }
    Ok(executor_plan)
}
