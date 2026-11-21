//! The test support of the `load` use case: the rules each scenario's input names, with what each binds its parameters to and
//! the statistics it is loaded with, and a load said as the scenarios state it. The plans are the lowering's fixtures.

use super::errors::LoadError;
use super::interface::LoadUseCase as _;
use super::types::{LoadSource, LoadedRules};
use super::use_case::LoadUseCaseImpl;
use crate::domain::code::types::Op;
use crate::domain::code::types::UnitKey;
use crate::domain::lowered::service::fill_code;
use crate::domain::lowered::types::MadeFill;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::fixtures::{digests_of, named};
use crate::domain::lowering::types::{LoadGiven, SelectedRule, SelectedRules};
use crate::domain::programs::service::accepted;
use crate::domain::programs::types::Programs;
use crate::domain::statistics::service::learned;
use crate::domain::statistics::types::{
    EncodedStatistics, MatchedStatistics, Measured, PointCounts, StatisticsBytes,
};
use crate::inbound::use_cases::scan::fixtures::scanned;
use ryo_language::domain::fillers::fixtures::Walking;
use ryo_language::domain::fillers::types::Fillers as _;
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::descriptions::types::{
    FieldWord, FixedName, KindVocabulary, KindWord, LanguageName,
};
use ryo_values::domain::plan_parts::types::{LiteralText, RepeatedRows};
use ryo_values::domain::query_bindings::types::{ParameterBinding, RuleBindings};
use ryo_values::domain::query_plans::types::{NodeDigests, Plan};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::collections::BTreeSet;

/// The input that names a rule set of no rule.
const NO_RULES: &str = "no rules";
/// The input of one rule the lowering lowers and one it cannot evaluate.
const ONE_REFUSED: &str = "one tree rule and one over the host's rows";
/// The input of two rules that share a sub-plan: the right side of each one's antijoin.
const SHARED: &str = "two rules, one sub-plan";
/// The input of two rules whose plans differ only in what a parameter of their sub-plan, the right side of their antijoin, is
/// bound to.
const BOUND: &str = "two rules, one sub-plan, two bound values";
/// The input of one rule loaded with the statistics of a sample of other rules.
const OTHER_STATISTICS: &str = "one tree rule, statistics of other rules";
/// The input whose answer says how many of its tables hold a product stage.
const PRODUCT: &str = "a product within a file";
/// The input of no rule and a language that states one fixed name at two places.
const FIXED_TWICE: &str = "a vocabulary that states a fixed name twice";
/// The input of a rule that reads the links by key and one that reads them whole.
const KEYED_THEN_WHOLE: &str = "a keyed read, then the whole relation";
/// The input of one walk over the file's own tree and the same walk over the tree with the embedded segments.
const EACH_TREE: &str = "one walk over each tree";
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

/// The names a scenario's language states: the fixture language's, and one fixed name at two places where the input says so.
fn vocabulary(scene: &str) -> KindVocabulary {
    let twice = || vec![FixedName("next".into()), FixedName("next".into())];
    KindVocabulary {
        fixed: if scene == FIXED_TWICE {
            twice()
        } else {
            Vec::new()
        },
        ..kinds()
    }
}

/// The names of the fixture language, as its walk states them: its kinds, its fields and its fixed names, each by place.
#[must_use]
pub fn kinds() -> KindVocabulary {
    let stated = Walking::vocabulary();
    let named = |names: &[NameText<'static>]| -> Vec<Box<str>> {
        names.iter().map(|name| Box::from(name.0)).collect()
    };
    KindVocabulary {
        language: LanguageName("walking".into()),
        kinds: named(&stated.kinds.0).into_iter().map(KindWord).collect(),
        fields: named(&stated.fields.0).into_iter().map(FieldWord).collect(),
        fixed: named(&stated.fixed.0).into_iter().map(FixedName).collect(),
    }
}

/// The rules a scenario's input names, in rule order: no rule, several named plans, or the one plan of the input's own name.
fn scene_rules(scene: &str) -> Vec<SceneRule> {
    let unbound = |name: &str| SceneRule {
        plan: named(name),
        bindings: Vec::new(),
    };
    let bound = |word: &str| SceneRule {
        plan: named("the edges to no word of a parameter"),
        bindings: vec![ParameterBinding::Text(LiteralText(word.into()))],
    };
    match scene {
        NO_RULES => Vec::new(),
        ONE_REFUSED => vec![unbound("one tree rule"), unbound("over the host's rows")],
        SHARED => vec![
            unbound("the elements that are no word"),
            unbound("the edges to no word"),
        ],
        BOUND => vec![bound("a"), bound("b")],
        OTHER_STATISTICS => vec![unbound("one tree rule")],
        FIXED_TWICE => Vec::new(),
        KEYED_THEN_WHOLE => vec![unbound("the links of the words"), unbound("every link")],
        EACH_TREE => vec![
            unbound("the parents over the syntax tree"),
            unbound("the parents over the semantic tree"),
        ],
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
    let digests: Vec<NodeDigests> = rules
        .iter()
        .map(|rule| digests_of(&rule.plan, RuleBindings(&rule.bindings)))
        .collect();
    let selected: Vec<SelectedRule<'_>> = rules
        .iter()
        .zip(&digests)
        .zip(0..)
        .map(|((rule, digests), ordinal)| SelectedRule {
            rule: SelectedRuleOrdinal(ordinal),
            plan: &rule.plan,
            bindings: RuleBindings(&rule.bindings),
            digests,
        })
        .collect();
    LoadUseCaseImpl.load(
        LoadSource::Plans(SelectedRules(&selected), statistics, LoadGiven::default()),
        &vocabulary(scene),
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

/// How many definitions were lowered, by node: the nodes the programs' pattern units are named by and the nodes of the tables
/// that are a definition's own rows (a definition read with nothing bound has that table's unit and no pattern unit), each
/// once. A rule's own program and a table of any other fill are no definition.
fn definitions(programs: &Programs) -> usize {
    let own_rows = fill_code(&MadeFill::Rows(Box::new([]), RepeatedRows::Kept));
    let nodes: BTreeSet<_> = programs
        .0
        .code
        .units
        .iter()
        .filter_map(|unit| match &unit.key {
            UnitKey::Pattern(node, ..) => Some(node),
            UnitKey::Table(node, fill) => (*fill == own_rows).then_some(node),
            UnitKey::Rule(_) => None,
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
        OTHER_STATISTICS => said.push_str(&match loaded.matched {
            MatchedStatistics::Units(matched) => {
                format!("; statistics matched {} units", matched.0)
            }
            MatchedStatistics::Absent => "; no statistics read".to_owned(),
        }),
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
