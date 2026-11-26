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
use ryo_values::domain::finish_plans::types::FinishPlan;
use ryo_values::domain::program::types::ParsedModules;
use ryo_values::domain::query_tokens::types::{QuerySourceId, QueryView};
use ryo_values::domain::runs::types::RuleId;
use std::collections::BTreeMap;
use std::path::Path;
use super::rule_context;
use super::check_session::refusal::Refusal;
use std::sync::Arc;

pub fn peak_kb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|st| st.lines().find(|l| l.starts_with("VmHWM:")).and_then(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok()))
        .unwrap_or(0)
}

/// The pack's texts: each rule's query (rules in byte order, `None` without one) and each fragment by file stem.
pub struct PackTexts {
    pub rules: Vec<(String, Option<String>)>,
    pub fragments: Vec<(String, String)>,
}

pub fn read_texts(pack: &Path) -> PackTexts {
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

pub fn parsed(texts: &PackTexts, context: RuleContext) -> PackPrograms {
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

pub fn run_lower(args: &[&str]) {
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

pub fn run_stage(args: &[&str]) {
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
pub struct ReadPack {
    pub typed: std::cell::RefCell<Option<std::sync::Arc<FinishPlan>>>,
    pub programs: PackPrograms,
    /// Thread CPU of reading the texts (and the request's rule context) and of parsing them.
    pub read: f64,
    pub parse: f64,
}

impl ReadPack {
    pub fn lower_axes(&self) -> std::sync::Arc<FinishPlan> {
        std::sync::Arc::new(lowered(&self.programs, PlanningStage::Axes))
    }
}

pub fn read_pack(pack: &Path, context: RuleContext) -> ReadPack {
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
pub fn second_half_plan(pack: &ReadPack, keep_for_first: bool) -> super::finish::Plan {
    finish_plan(pack, keep_for_first, true, None, None, None).unwrap_or_else(|reason| panic!("{reason}"))
}

/// Prepare the caller's query directly from its text. Bind typed host tables at execution.
pub fn query_plan(rule: &str, text: &str, context: RuleContext) -> Result<super::finish::Plan, String> {
    query_parts(rule, text, context, false, None, Default::default(), &[], &mut Default::default()).map(|(plan, _)| plan).map_err(String::from)
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Producer {
    pub rule: Box<str>,
    pub text: Arc<str>,
    pub output: Box<str>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Observation {
    pub name: Box<str>,
    pub columns: Vec<ryo_values::domain::observations::types::ObservationColumn>,
    pub producers: Vec<Producer>,
}

#[derive(Default)]
pub struct SharedPlanData {
    pub rows: rustc_hash::FxHashSet<Arc<ryo_values::domain::finish_plans::arguments::literals::types::RowsArguments>>,
    pub conditions: rustc_hash::FxHashSet<Arc<ryo_values::domain::finish_plans::expressions::types::PlanExpression>>,
    pub projections: rustc_hash::FxHashSet<Arc<[ryo_values::domain::finish_plans::types::SelectItem]>>,
}

pub fn intern<T: ?Sized + Eq + std::hash::Hash>(set: &mut rustc_hash::FxHashSet<Arc<T>>, value: &mut Arc<T>) {
    if let Some(previous) = set.get(value) {
        value.clone_from(previous);
    } else {
        set.insert(value.clone());
    }
}

pub fn query_parts(rule: &str, text: &str, mut context: RuleContext, keep: bool, separator: Option<&str>, modules: ParsedModules, observations: &[Observation], literals: &mut SharedPlanData) -> Result<(super::finish::Plan, ReadPack), Refusal> {
    use ryo_engine::domain::plan_lowering::types::{ContextObservation, ObservationProducer};
    use ryo_values::domain::{descriptions::types::ColumnName, observations::types::ObservationName};
    let mut texts = PackTexts { rules: vec![(rule.into(), Some(text.into()))], fragments: Vec::new() };
    let mut producer_ids = BTreeMap::new();
    for input in observations {
        let mut producers = Vec::new();
        for source in &input.producers {
            let index = producer_ids.len();
            let name = producer_ids.entry(source.rule.as_ref()).or_insert_with(|| {
                let alias = format!("producer {index}");
                texts.rules.push((alias.clone(), Some(source.text.to_string())));
                alias
            });
            producers.push(ObservationProducer { rule: RuleId(name.as_str().into()), output: ObservationName(source.output.clone()) });
        }
        context.observations.push(ContextObservation {
            name: ObservationName(input.name.clone()), columns: input.columns.iter().map(|c| ColumnName(c.name.0.clone())).collect(), producers
        });
    }
    let mut programs = parsed(&texts, context);
    programs.fragments = modules;
    let pack = ReadPack { typed: Default::default(), programs, read: 0.0, parse: 0.0 };
    let plan = finish_plan(&pack, keep, false, separator, (!observations.is_empty()).then_some(rule), Some(literals))?;
    Ok((plan, pack))
}

/// The queries of one module catalog lowered together, once: their texts are parsed and expanded in one pack so
/// fragments and shared steps exist once, then each query takes the steps its emit reads (a per-query step subset,
/// renumbered), which is the plan it would have had alone. A query that does not lower answers with its own error.
pub fn query_batch(
    queries: &[(&str, &BTreeMap<Box<str>, ryo_values::domain::finish_plans::expressions::types::PlanConstant>)],
    separator: &str,
    modules: ParsedModules,
    literals: &mut SharedPlanData,
) -> Vec<Result<(super::finish::Plan, Arc<FinishPlan>), Refusal>> {
    use ryo_engine::domain::plan_lowering::shared::SharedSteps;
    use ryo_values::domain::finish_plans::{
        arguments::types::StepNumber,
        service::inputs,
        types::{PlanEmit, PlannedRule, RuleEmit},
    };
    let aliases: Vec<String> = (0..queries.len()).map(|i| format!("query {i}")).collect();
    let mut context = RuleContext::default();
    let mut texts = PackTexts { rules: Vec::new(), fragments: Vec::new() };
    for (alias, (text, parameters)) in aliases.iter().zip(queries) {
        texts.rules.push((alias.clone(), Some((*text).to_owned())));
        context.parameters.insert(alias.as_str().into(), (*parameters).clone());
    }
    let mut programs = parsed(&texts, context);
    programs.fragments = modules;
    let mut plan = lowered(&programs, PlanningStage::Expanded);
    drop(programs);
    mark_through(&mut plan, PlanningStage::Axes);
    {
        use ryo_values::domain::{finish_plans::types::{FinishStep, RowCondition, SelectedColumns}, plan_parts::types::LiteralText};
        for step in &mut plan.steps {
            match step {
                FinishStep::Capture(c) => { c.separator.get_or_insert_with(|| LiteralText(separator.into())); }
                FinishStep::Rows(rows) => intern(&mut literals.rows, rows),
                FinishStep::Select(select) => {
                    if let RowCondition::Tested(term) = &mut select.condition {
                        intern(&mut literals.conditions, term);
                    }
                    if let SelectedColumns::Items(items) = &mut select.columns {
                        intern(&mut literals.projections, items);
                    }
                }
                _ => {}
            }
        }
    }
    let mut emits: std::collections::HashMap<&str, &PlanEmit> = std::collections::HashMap::new();
    let mut unwritten = std::collections::HashSet::new();
    for planned in &plan.rules {
        match &planned.emit {
            RuleEmit::Written(emit) => { emits.insert(planned.rule.0.as_ref(), emit); }
            RuleEmit::Unwritten => { unwritten.insert(planned.rule.0.as_ref()); }
        }
    }
    let reasons: std::collections::HashMap<&str, (&str, Option<&str>)> =
        plan.unplanned.iter().map(|u| (u.rule.0.as_ref(), (u.reason.0.as_ref(), u.ungiven.as_deref()))).collect();
    let mut seen = vec![false; plan.steps.len()];
    let mut numbers = vec![usize::MAX; plan.steps.len()];
    let mut reached = Vec::new();
    aliases.iter().map(|alias| {
        if let Some((reason, ungiven)) = reasons.get(alias.as_str()) {
            return Err(match ungiven {
                Some(name) => Refusal::MissingParameter { name: (*name).to_owned() },
                None => format!("unplanned query {reason}").into(),
            });
        }
        let Some(emit) = emits.get(alias.as_str()) else {
            return Err(Refusal::Other(if unwritten.contains(alias.as_str()) { "missing query emit".into() } else { "unplanned query".to_owned() }));
        };
        seen.iter_mut().for_each(|s| *s = false);
        reached.clear();
        let mut stack = vec![emit.step.0];
        while let Some(at) = stack.pop() {
            if std::mem::replace(&mut seen[at], true) { continue; }
            reached.push(at);
            stack.extend(inputs(&plan.steps[at]).into_iter().map(|input| input.0));
        }
        reached.sort_unstable();
        let mut shared = SharedSteps::default();
        for &at in &reached {
            numbers[at] = shared.append(plan.steps[at].clone(), &numbers);
        }
        let planned = PlannedRule {
            rule: RuleId("query".into()),
            emit: RuleEmit::Written(PlanEmit { step: StepNumber(numbers[emit.step.0]), subject: emit.subject.clone(), key: emit.key.clone() }),
        };
        let typed = Arc::new(FinishPlan { steps: shared.finish(), rules: vec![planned], unplanned: Vec::new() });
        Ok((super::finish::from_typed(typed.clone()), typed))
    }).collect()
}

pub fn finish_plan(pack: &ReadPack, keep_for_first: bool, log: bool, separator: Option<&str>, emit: Option<&str>, literals: Option<&mut SharedPlanData>) -> Result<super::finish::Plan, Refusal> {
    let t0 = super::thread_cpu();
    let mut plan = lowered(&pack.programs, PlanningStage::Expanded);
    let t1 = super::thread_cpu();
    // no fallback: a rule the lowering cannot plan fails the run, by name and reason (never a run without it, never a
    // plan from elsewhere)
    if let [only] = plan.unplanned.as_slice() && let Some(name) = &only.ungiven {
        return Err(Refusal::MissingParameter { name: name.to_string() });
    }
    if !plan.unplanned.is_empty() {
        let reasons: Vec<_> = plan.unplanned.iter().map(|u| format!("unplanned {} {}", u.rule.0, u.reason.0)).collect();
        return Err(reasons.join("\n").into());
    }
    if let Some(rule) = emit { plan.rules.retain(|r| r.rule.0.as_ref() == rule); }
    mark_through(&mut plan, PlanningStage::Axes);
    let t2 = super::thread_cpu();
    let steps = plan.steps.len();
    if let Some(separator)=separator {
        use ryo_values::domain::{finish_plans::types::FinishStep, plan_parts::types::LiteralText};
        for step in &mut plan.steps {
            if let FinishStep::Capture(c)=step {c.separator.get_or_insert_with(||LiteralText(separator.into()));}
        }
    }
    if let Some(tables) = literals {
        for step in &mut plan.steps {
            use ryo_values::domain::finish_plans::types::{FinishStep, RowCondition, SelectedColumns};
            match step {
                FinishStep::Rows(rows) => intern(&mut tables.rows, rows),
                FinishStep::Select(select) => {
                    if let RowCondition::Tested(term) = &mut select.condition {
                        intern(&mut tables.conditions, term);
                    }
                    if let SelectedColumns::Items(items) = &mut select.columns {
                        intern(&mut tables.projections, items);
                    }
                }
                _ => {}
            }
        }
    }
    let plan = std::sync::Arc::new(plan);
    let executor_plan = super::finish::from_typed(plan.clone());
    if keep_for_first {
        pack.typed.replace(Some(plan));
    }
    let t5 = super::thread_cpu();
    if log {
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
