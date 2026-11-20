//! The test support of the `load` use case: the rules each scenario's input names, with what each binds its parameters to and
//! the statistics it is loaded with, and a load said as the scenarios state it. The plans are the lowering's fixtures.

use super::errors::LoadError;
use super::interface::LoadUseCase as _;
use super::types::{LoadSource, LoadedRules};
use super::use_case::LoadUseCaseImpl;
use crate::domain::code::types::Op;
use crate::domain::code::types::UnitKey;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::fixtures::named;
use crate::domain::lowering::types::{SelectedRule, SelectedRules};
use crate::domain::programs::service::accepted;
use crate::domain::programs::types::Programs;
use crate::domain::statistics::service::learned;
use crate::domain::statistics::types::{EncodedStatistics, Measured, PointCounts, StatisticsBytes};
use crate::inbound::use_cases::scan::fixtures::scanned;
use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::query_bindings::types::{ParameterBinding, RuleBindings};
use ryo_values::domain::query_plans::types::Plan;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::collections::BTreeSet;

/// The input that names a rule set of no rule.
const NO_RULES: &str = "no rules";
/// The input of one rule the lowering lowers and one it cannot evaluate.
const ONE_REFUSED: &str = "one tree rule and one over the host's rows";
/// The input of two rules that share a sub-plan.
const SHARED: &str = "two rules, one sub-plan";
/// The input of two rules whose plans differ only in what a parameter of their sub-plan is bound to.
const BOUND: &str = "two rules, one sub-plan, two bound values";
/// The input of one rule loaded with the statistics of a sample of other rules.
const OTHER_STATISTICS: &str = "one tree rule, statistics of other rules";
/// The input whose answer says how many of its tables hold a product stage.
const PRODUCT: &str = "a product within a file";
/// The file a sample of a scene's rules is run over.
const SAMPLE: &str = "a b a";

/// One rule of a scenario's input: its plan, and what its parameters are bound to.
#[derive(Clone, Debug)]
struct SceneRule {
    /// The plan.
    plan: Plan,
    /// What each of the plan's parameters is bound to, in the plan's order.
    bindings: Vec<ParameterBinding>,
}

/// The kinds of the fixture language.
#[must_use]
pub fn kinds() -> KindVocabulary {
    KindVocabulary {
        language: LanguageName("walking".into()),
        kinds: Vec::new(),
    }
}

/// The rules a scenario's input names, in rule order: no rule, several named plans, or the one plan of the input's own name.
fn scene_rules(scene: &str) -> Vec<SceneRule> {
    let unbound = |name: &str| SceneRule {
        plan: named(name),
        bindings: Vec::new(),
    };
    let bound = |word: &str| SceneRule {
        plan: named("the words of a parameter"),
        bindings: vec![ParameterBinding::Text(LiteralText(word.into()))],
    };
    match scene {
        NO_RULES => Vec::new(),
        ONE_REFUSED => vec![unbound("one tree rule"), unbound("over the host's rows")],
        SHARED => vec![unbound("the words a"), unbound("the words b")],
        BOUND => vec![bound("a"), bound("b")],
        OTHER_STATISTICS => vec![unbound("one tree rule")],
        other => vec![unbound(other)],
    }
}

/// Loads a scenario's rules with `statistics`.
///
/// # Errors
///
/// The load's refusal: never for a rule, which is answered among the refused.
pub fn loaded_with(scene: &str, statistics: StatisticsBytes<'_>) -> Result<LoadedRules, LoadError> {
    let rules = scene_rules(scene);
    let selected: Vec<SelectedRule<'_>> = rules
        .iter()
        .zip(0..)
        .map(|(rule, ordinal)| SelectedRule {
            rule: SelectedRuleOrdinal(ordinal),
            plan: &rule.plan,
            bindings: RuleBindings(&rule.bindings),
        })
        .collect();
    LoadUseCaseImpl.load(
        LoadSource::Plans(SelectedRules(&selected), statistics),
        &kinds(),
    )
}

/// What a sample of a scenario's rules, loaded with no statistics and run over a small file, learns.
///
/// # Errors
///
/// The load's refusal.
pub fn sampled(scene: &str) -> Result<EncodedStatistics, LoadError> {
    let programs = loaded_with(scene, StatisticsBytes(&[]))?.programs;
    // A sample that scanned no file counted nothing.
    let counts = match scanned(&programs, SAMPLE) {
        Ok(Some(scan)) => scan.counts,
        Ok(None) | Err(_) => PointCounts::default(),
    };
    Ok(learned(
        Measured {
            programs: &programs,
            counts: &counts,
        },
        None,
    ))
}

/// The statistics a scenario's input is loaded with: a sample of other rules where its name says so, else none.
fn scene_statistics(scene: &str) -> Result<EncodedStatistics, LoadError> {
    if scene == OTHER_STATISTICS {
        sampled("as-of latest before")
    } else {
        Ok(EncodedStatistics::default())
    }
}

/// Loads a scenario's rules, with the statistics its input names.
///
/// # Errors
///
/// The load's refusal: never for a rule, which is answered among the refused.
pub fn loaded(scene: &str) -> Result<LoadedRules, LoadError> {
    let statistics = scene_statistics(scene)?;
    loaded_with(scene, StatisticsBytes(&statistics.0))
}

/// How many definitions the programs hold: the nodes their pattern units are named by, each once.
fn definitions(programs: &Programs) -> usize {
    let nodes: BTreeSet<_> = programs
        .0
        .code
        .units
        .iter()
        .filter_map(|unit| match &unit.key {
            UnitKey::Pattern(node, ..) => Some(node),
            UnitKey::Rule(_) | UnitKey::Table(..) => None,
        })
        .collect();
    nodes.len()
}

/// How many tables of the programs hold a product stage: a pipeline's words end in the write of its table.
fn product_tables(programs: &Programs) -> usize {
    let (mut tables, mut product) = (BTreeSet::new(), false);
    for word in &programs.0.code.words {
        if word.op == Op::StageProduct {
            product = true;
        }
        if word.op == Op::StageWrite {
            if product {
                let _new = tables.insert(word.a);
            }
            product = false;
        }
    }
    tables.len()
}

/// A load as the scenarios state it: how many rules it runs; for the inputs that ask, how many definitions, tables with a
/// product stage or units the statistics matched; then each rule it refused, with why.
#[must_use]
pub fn said(scene: &str, loaded: &LoadedRules) -> String {
    let programs = &loaded.programs;
    let mut said = format!("loaded {} rules", accepted(programs).len());
    match scene {
        SHARED | BOUND => said.push_str(&format!(", {} definitions", definitions(programs))),
        PRODUCT => said.push_str(&format!(
            ", {} tables with a product stage",
            product_tables(programs)
        )),
        OTHER_STATISTICS => {
            said.push_str(&format!("; statistics matched {} units", loaded.matched.0));
        }
        _ => {}
    }
    if !loaded.refused.is_empty() {
        let each: Vec<String> = loaded
            .refused
            .iter()
            .map(|refused| {
                LoweringError::Refused {
                    rule: refused.rule,
                    refusal: refused.refusal,
                }
                .to_string()
            })
            .collect();
        said.push_str(&format!("; refused {}: {}", each.len(), each.join("; ")));
    }
    said
}
