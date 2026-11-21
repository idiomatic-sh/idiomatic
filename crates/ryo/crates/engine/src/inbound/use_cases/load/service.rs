//! The `load` use case's own sequence over a lowering: lower, pass, encode, wrap; and what happens when the encoding refuses a
//! rule. The lowering is a parameter so that the sequence is proved before the lowering lowers.

use super::errors::LoadError;
use super::types::{LoadPass, LoadedRules};
use crate::domain::code::types::ProgramCode;
use crate::domain::encoding::service::encoded;
use crate::domain::lowered::types::LoweredPack;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{
    LoadRefusal, LoweredRules, PlanNode, RefusedRule, SelectedRule, SelectedRules,
};
use crate::domain::passes::service::passed;
use crate::domain::programs::types::{Programs, ProgramsEngine, RuleSteps};
use crate::domain::statistics::service::matched;
use crate::domain::statistics::types::{MatchedStatistics, Statistics};
use ryo_language::domain::facts::service::base_words;
use ryo_language::domain::facts::types::{BaseWords, FixedNames};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// Each loaded rule's plan as a work notice needs it, in the code's rule order: the step it emits and its steps' digests.
fn rule_steps(code: &ProgramCode, rules: SelectedRules<'_>) -> Vec<RuleSteps> {
    let of = |rule: SelectedRuleOrdinal| rules.0.iter().find(|selected| selected.rule == rule);
    let loaded = code.rules.iter().filter_map(|rule| of(rule.rule));
    loaded
        .map(|selected| RuleSteps {
            rule: selected.rule,
            emitted: selected.plan.emit.step,
            nodes: selected.digests.clone(),
        })
        .collect()
}

/// The run's base words: the language's fixed names, then the pack's constant texts. This is where a vocabulary whose fixed
/// names cannot be words is refused, once for a load.
pub(super) fn worded(fixed: &FixedNames, texts: &[NameText<'_>]) -> Result<BaseWords, LoadError> {
    base_words(fixed, texts).map_err(LoadError::Vocabulary)
}

/// The programs of an encoded pack.
fn programs(
    pack: LoweredPack,
    code: ProgramCode,
    steps: Vec<RuleSteps>,
) -> Result<Programs, LoadError> {
    let texts: Vec<NameText<'_>> = pack.texts.iter().map(|text| NameText(&text.0)).collect();
    // JOIN(fixed names): the vocabulary's fixed names (`KindVocabulary.fixed`, owned texts) go here, first. At this head
    // `base_words` takes them as `&FixedNames`, whose texts are static, so the two cannot meet: the run's words hold no fixed
    // name and no vocabulary is refused. The integration's checked fixed names, built from borrowed texts, replace this line.
    let fixed = FixedNames::default();
    let words = worded(&fixed, &texts)?;
    Ok(Programs(ProgramsEngine {
        code,
        words,
        regexes: pack.regexes,
        reads: pack.reads,
        steps,
    }))
}

/// The refusal a rule takes from an encoding's: a step the encoding names is held to the rule's own plan, whose last step is the
/// greatest the encoding can mean (it holds no plan).
fn held(refusal: LoadRefusal, rule: Option<&SelectedRule<'_>>) -> LoadRefusal {
    match refusal {
        LoadRefusal::Unlowerable(step) => {
            let last = rule.map_or(0, |rule| rule.plan.steps.len().saturating_sub(1));
            LoadRefusal::Unlowerable(PlanNode(
                step.0.min(u32::try_from(last).unwrap_or(u32::MAX)),
            ))
        }
        LoadRefusal::Budget(width) => LoadRefusal::Budget(width),
    }
}

/// The rules an encoding's refusal takes out of a pack whose rules were written in `written` order. A count of the pack past
/// its width takes the rule that was being written and every rule after it, in one pass: lowering again without one rule at a
/// time would lower the pack once for each. The encoding's own fault takes the one rule. On the last lowering either takes
/// every rule, since nothing is lowered a third time.
fn taken(
    written: &[SelectedRuleOrdinal],
    refused: SelectedRuleOrdinal,
    refusal: LoadRefusal,
    lowering: LoadPass,
) -> Vec<SelectedRuleOrdinal> {
    let after = written.iter().copied().skip_while(|rule| *rule != refused);
    match (lowering, refusal) {
        (LoadPass::Last, LoadRefusal::Budget(_) | LoadRefusal::Unlowerable(_)) => written.to_vec(),
        (LoadPass::First, LoadRefusal::Budget(_)) => after.collect(),
        (LoadPass::First, LoadRefusal::Unlowerable(_)) => after.take(1).collect(),
    }
}

/// Loads `rules` through `lower`: the lowered pack passed and encoded, each rule the lowering or the encoding refuses answered
/// with why, in rule order. The rules are lowered twice at most.
///
/// # Errors
///
/// `Vocabulary` when the language's fixed names cannot be words of the run. With no statistics no unit is matched or unmatched: the
/// answer's `matched` is `Absent`.
pub(super) fn sequenced(
    rules: SelectedRules<'_>,
    statistics: Option<&Statistics>,
    lower: &dyn Fn(SelectedRules<'_>) -> LoweredRules,
) -> Result<LoadedRules, LoadError> {
    let mut selected: Vec<SelectedRule<'_>> = rules.0.to_vec();
    let mut refused: Vec<RefusedRule> = Vec::new();
    let (mut loaded, mut named) = (Programs::default(), MatchedStatistics::Absent);
    // Once, and once more without the rules an encoding refused: never a third time.
    for lowering in [LoadPass::First, LoadPass::Last] {
        let LoweredRules {
            mut pack,
            refused: unlowered,
        } = lower(SelectedRules(&selected));
        refused.extend(unlowered);
        passed(&mut pack, statistics);
        let written: Vec<SelectedRuleOrdinal> = pack.rules.iter().map(|rule| rule.rule).collect();
        match encoded(&pack) {
            Ok(code) => {
                named = statistics.map_or(MatchedStatistics::Absent, |measured| {
                    MatchedStatistics::Units(matched(measured, &code.units))
                });
                let steps = rule_steps(&code, rules);
                loaded = programs(pack, code, steps)?;
                break;
            }
            Err(LoweringError::Refused { rule, refusal }) => {
                let gone = taken(&written, rule, refusal, lowering);
                for each in &gone {
                    let plan = rules.0.iter().find(|selected| selected.rule == *each);
                    refused.push(RefusedRule {
                        rule: *each,
                        refusal: held(refusal, plan),
                    });
                }
                selected.retain(|rule| written.contains(&rule.rule) && !gone.contains(&rule.rule));
            }
        }
    }
    refused.sort_by_key(|rule| rule.rule);
    Ok(LoadedRules {
        programs: loaded,
        refused,
        matched: named,
    })
}
