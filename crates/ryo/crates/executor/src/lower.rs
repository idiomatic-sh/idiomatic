//! The edge from RyoQL text to the plans the executor runs: the query language's one parser reads each query and fragment, and
//! its `plan_lowering` lowers them and runs its passes.

use ryo_query_language::domain::plan_lowering::service::{lowered, mark_through, passed};
use ryo_query_language::domain::plan_lowering::types::{
    PackPrograms, PackRule, PlanningStage, RuleContext, RuleProgram, UnreadQuery,
};
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

/// The pack's texts: each rule's query (rules in byte order, `None` without one) and each fragment by file stem.
pub struct PackTexts {
    pub rules: Vec<(String, Option<String>)>,
    pub fragments: Vec<(String, String)>,
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

/// The query's parsed programs, and the typed plan kept for the first half of the analysis.
pub struct ReadPack {
    pub typed: std::cell::RefCell<Option<std::sync::Arc<FinishPlan>>>,
    pub programs: PackPrograms,
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
    use ryo_query_language::domain::plan_lowering::types::{ContextObservation, ObservationProducer};
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
    let pack = ReadPack { typed: Default::default(), programs };
    let plan = finish_plan(&pack, keep, separator, (!observations.is_empty()).then_some(rule), Some(literals))?;
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
    use ryo_query_language::domain::plan_lowering::shared::SharedSteps;
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

pub fn finish_plan(pack: &ReadPack, keep_for_first: bool, separator: Option<&str>, emit: Option<&str>, literals: Option<&mut SharedPlanData>) -> Result<super::finish::Plan, Refusal> {
    let mut plan = lowered(&pack.programs, PlanningStage::Expanded);
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
    Ok(executor_plan)
}
