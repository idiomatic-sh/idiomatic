//! THROWAWAY edge (bead ryo-fmx3.1): a pack's rules from their RyoQL text to the finish plan, in Rust: the query language's one
//! parser reads every rule and fragment, the engine's `plan_lowering` lowers them and runs its passes, the JSON plan codec
//! writes each stage.
//!
//! `vec lower <pack> <out prefix> [--repeat K]`: writes `<out prefix>-u.json` (expanded), `-b1.json` (semijoins), `-b2.json`
//! (counts) and `.json` (axes), the names the Python chain (`scripts6h/plan.sh`) writes; prints each phase's thread CPU
//! (user + system) and the peak resident set after it. With `--repeat K` the phases run K times in one process, and each
//! repeat's CPU is printed. With `--measure` nothing is written but the final plan's encoding, after the peak is read.
//! `vec stage <in.json> <stage> <out.json>`: decodes a plan, runs the one pass `stage` (`semijoins`, `counts`, `axes`) adds,
//! writes it; `stage` `none` writes the decoded plan back (a round trip).

use ryo_engine::domain::plan_lowering::service::{lowered, passed};
use ryo_engine::domain::plan_lowering::types::{PackPrograms, PackRule, PlanningStage, RuleProgram, UnreadQuery};
use ryo_engine::outbound::plan_codec::factory::json;
use ryo_engine::outbound::plan_codec::interface::PlanCodecPort as _;
use ryo_query_language::outbound::syntax::factory::winnow;
use ryo_query_language::outbound::syntax::interface::SyntaxPort as _;
use ryo_values::domain::program::types::ParsedModules;
use ryo_values::domain::query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId, QueryView};
use ryo_values::domain::runs::types::RuleId;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

/// The root a fragment bundle is read under: the parser reads a bundle's root first, and fragments are read without one.
const BUNDLE_ROOT: &str = "let rows = select file from \"syntax-tree\";\nemit rows subject file key(file);\n";

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

fn parsed(texts: &PackTexts) -> PackPrograms {
    let syntax = winnow();
    // One bundle a fragment, so that a fragment the parser refuses refuses only the rules that import it.
    let mut fragments = BTreeMap::new();
    for (stem, text) in &texts.fragments {
        let id = QuerySourceId(stem.as_str().into());
        let modules = BTreeMap::from([(id.clone(), QueryDocumentSource { id: id.clone(), text: text.as_str().into() })]);
        let bundle = QueryBundle {
            root: QueryDocumentSource { id: QuerySourceId("root".into()), text: BUNDLE_ROOT.into() },
            modules: Arc::new(modules),
        };
        match syntax.read_bundle(&bundle) {
            Ok(mut b) => {
                if let Some(fragment) = b.fragments.remove(&id) {
                    let _earlier = fragments.insert(id, fragment);
                }
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
    PackPrograms { rules, fragments: ParsedModules(Arc::new(fragments)) }
}

pub(crate) fn run_lower(args: &[&str]) {
    let pack = Path::new(args[0]);
    let out = args[1];
    let repeat: usize = args.iter().position(|a| *a == "--repeat").map_or(1, |i| args[i + 1].parse().expect("repeat"));
    // `--measure`: no stage is written, so the peak after the passes is preparation's own.
    let measure = args.contains(&"--measure");
    let codec = json();
    for round in 0..repeat {
        let write = round == 0 && !measure;
        let t0 = super::thread_cpu();
        let texts = read_texts(pack);
        let t1 = super::thread_cpu();
        let pack_programs = parsed(&texts);
        let t2 = super::thread_cpu();
        let hwm_parse = peak_kb();
        let mut plan = lowered(&pack_programs, PlanningStage::Expanded);
        let t3 = super::thread_cpu();
        let expanded_bytes = write.then(|| codec.encoded(&plan));
        let t4 = super::thread_cpu();
        let mut stages = Vec::new();
        let mut pass_cpu = 0.0;
        for (stage, suffix) in [(PlanningStage::Semijoins, "-b1"), (PlanningStage::Counts, "-b2"), (PlanningStage::Axes, "")] {
            let a = super::thread_cpu();
            let marked = passed(&mut plan, stage);
            pass_cpu += super::thread_cpu() - a;
            if write {
                stages.push((suffix, marked, codec.encoded(&plan)));
            }
        }
        let t5 = super::thread_cpu();
        let hwm_passes = peak_kb();
        let final_bytes = codec.encoded(&plan);
        let t6 = super::thread_cpu();
        let hwm = peak_kb();
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
            t6 - t5,
            (t3 - t1) + pass_cpu,
        );
        let _encode_expanded = t4 - t3;
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
        drop(final_bytes);
    }
}

pub(crate) fn run_stage(args: &[&str]) {
    let codec = json();
    let bytes = std::fs::read(args[0]).expect("read plan");
    let mut plan = codec.decoded(&bytes).unwrap_or_else(|e| panic!("{e}"));
    let marked = match args[1] {
        "none" => 0,
        "semijoins" => passed(&mut plan, PlanningStage::Semijoins),
        "counts" => passed(&mut plan, PlanningStage::Counts),
        "axes" => passed(&mut plan, PlanningStage::Axes),
        other => panic!("stage {other}"),
    };
    println!("marked {marked}");
    std::fs::write(args[2], codec.encoded(&plan)).expect("write");
}

/// The pack's rules and fragments read and parsed once at start, for both halves' plans (`vec funnel ... --finish <pack dir>`).
pub(crate) struct ReadPack {
    pub(crate) programs: PackPrograms,
    /// Thread CPU of reading the texts and of parsing them.
    pub(crate) read: f64,
    pub(crate) parse: f64,
}

pub(crate) fn read_pack(pack: &Path) -> ReadPack {
    let t0 = super::thread_cpu();
    let texts = read_texts(pack);
    let t1 = super::thread_cpu();
    let programs = parsed(&texts);
    let t2 = super::thread_cpu();
    drop(texts);
    ReadPack { programs, read: t1 - t0, parse: t2 - t1 }
}

/// The second half's plan, lowered in this process from the parsed pack: expansion, the three passes, then the typed plan
/// handed to the executor as the JSON value it reads (the codec's bytes decoded in memory; no file). Prints its phases.
pub(crate) fn second_half_plan(pack: &ReadPack) -> super::finish::Plan {
    let t0 = super::thread_cpu();
    let mut plan = lowered(&pack.programs, PlanningStage::Expanded);
    let t1 = super::thread_cpu();
    for stage in [PlanningStage::Semijoins, PlanningStage::Counts, PlanningStage::Axes] {
        let _marked = passed(&mut plan, stage);
    }
    let t2 = super::thread_cpu();
    let bytes = json().encoded(&plan);
    let steps = plan.steps.len();
    drop(plan);
    let t3 = super::thread_cpu();
    let value: serde_json::Value = serde_json::from_slice(&bytes).expect("the codec's own plan");
    drop(bytes);
    let t4 = super::thread_cpu();
    let executor_plan = super::finish::from_value(&value);
    drop(value);
    let t5 = super::thread_cpu();
    println!(
        "start-up second half: steps {steps} expand {:.4} passes {:.4} encode {:.4} decode {:.4} executor-plan {:.4} total {:.4} s",
        t1 - t0,
        t2 - t1,
        t3 - t2,
        t4 - t3,
        t5 - t4,
        t5 - t0
    );
    executor_plan
}
