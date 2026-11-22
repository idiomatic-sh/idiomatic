//! The `load` use case's own sequence over a lowering: lower, pass, encode, wrap, ask the executor whether it runs every
//! word and fill of each rule; and what happens when the encoding refuses a rule or the executor cannot run one whole:
//! that rule is refused with its own cause and the others are lowered again without it, until nothing is refused. The
//! lowering is a parameter so that the sequence is proved before the lowering lowers.

use super::errors::LoadError;
use super::types::{LoadedRules, NamedWords};
use crate::domain::code::types::{NeedSite, ProgramCode, UnitKey};
use crate::domain::needs::masks::service::node_asked;
use crate::domain::needs::masks::types::{NeedMasks, NodeAsk};
use crate::domain::scratch::types::NodeNeed;
use crate::domain::encoding::service::encoded;
use crate::domain::executing::running::service::unit_not_run;
use crate::domain::lowered::types::LoweredPack;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{
    LoadRefusal, LoweredRules, PlanNode, RefusedRule, SelectedRule, SelectedRules,
};
use crate::domain::passes::service::passed;
use crate::domain::programs::service::unit_step;
use crate::domain::programs::types::{Programs, ProgramsEngine, RuleSteps};
use crate::domain::statistics::service::matched;
use crate::domain::statistics::types::{MatchedStatistics, Statistics};
use ryo_language::domain::facts::service::{base_words, file_words, intern};
use ryo_language::domain::facts::types::BaseWords;
use ryo_language::domain::fixed_names::service::checked;
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::descriptions::types::KindVocabulary;
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

/// The run's base words: the empty text, the language's fixed names by place, then `texts`. This is the one place a
/// language's fixed names are checked, once for a load: a vocabulary that states one twice, or states the empty text, is
/// refused here.
pub(super) fn worded(
    fixed: &[NameText<'_>],
    texts: &[NameText<'_>],
) -> Result<BaseWords, LoadError> {
    let stated = checked(fixed).map_err(LoadError::Vocabulary)?;
    Ok(base_words(stated, texts))
}

/// The word of each kind and of each field the vocabulary names, by code, among the run's base words: every name is a base
/// word by now, so none is new.
pub(super) fn named_words(base: &BaseWords, names: &KindVocabulary) -> NamedWords {
    let mut held = file_words(base);
    let kinds = names.kinds.iter().map(|kind| &*kind.0);
    let kinds = kinds
        .map(|name| intern(&mut held, NameText(name)))
        .collect();
    let fields = names.fields.iter().map(|field| &*field.0);
    let fields = fields
        .map(|name| intern(&mut held, NameText(name)))
        .collect();
    NamedWords { kinds, fields }
}

/// The programs of an encoded pack.
fn programs(
    pack: LoweredPack,
    code: ProgramCode,
    steps: Vec<RuleSteps>,
    names: &KindVocabulary,
) -> Result<Programs, LoadError> {
    // The pack's constant texts keep their places after the fixed names, whatever the vocabulary: the names of the kinds and
    // of the fields follow them.
    let constant = pack.texts.iter().map(|text| &*text.0);
    let named = names.kinds.iter().map(|kind| &*kind.0);
    let named = named.chain(names.fields.iter().map(|field| &*field.0));
    let texts: Vec<NameText<'_>> = constant.chain(named).map(NameText).collect();
    let stated: Vec<NameText<'_>> = names.fixed.iter().map(|name| NameText(&name.0)).collect();
    let words = worded(&stated, &texts)?;
    let NamedWords { kinds, fields } = named_words(&words, names);
    let node_needs = node_needs(&code);
    Ok(Programs(ProgramsEngine {
        code,
        words,
        kinds,
        fields,
        regexes: pack.regexes,
        reads: pack.reads,
        steps,
        texts: pack.texts,
        node_needs,
    }))
}

/// Whether the formula of each need site of the rules' branches and the tables asks the node rows: a function of the code
/// alone, found once at load (or read from a pack) and handed to every worker, which would otherwise find it again at the
/// first ask of each site.
pub(crate) fn node_needs(code: &ProgramCode) -> Vec<Option<NodeNeed>> {
    let branches = code.rules.iter().flat_map(|rule| rule.branches.iter().map(|branch| branch.need));
    // A branch or a table that waits on no decision names no site (a place past the needs).
    let within = |site: &NeedSite| usize::try_from(site.0).is_ok_and(|place| place < code.needs.len());
    let sites: Vec<NeedSite> = branches
        .chain(code.tables.iter().map(|table| table.need))
        .filter(within)
        .collect();
    let last = sites.iter().map(|site| site.0).max().map_or(0, |last| last.saturating_add(1));
    let mut needs = vec![None; usize::try_from(last).unwrap_or(0)];
    // The answer for each site named on the way is kept, so a site many formulas name is walked once.
    let mut masks = NeedMasks::default();
    for site in sites {
        let Some(slot) = needs.get_mut(usize::try_from(site.0).unwrap_or(usize::MAX)) else {
            continue;
        };
        if slot.is_none() {
            *slot = Some(match node_asked(code, site, &mut masks) {
                NodeAsk::Asked => NodeNeed::Asked,
                NodeAsk::NotAsked | NodeAsk::Finding | NodeAsk::Unknown => NodeNeed::NotAsked,
            });
        }
    }
    needs
}

/// A pack read back, made runnable against `names`: its words, and the word of each kind and field, made as a load makes them
/// (the fixed names, then the pack's texts, then the kinds' and fields' names).
pub(super) fn worded_pack(mut programs: Programs, names: &KindVocabulary) -> Result<Programs, LoadError> {
    let constant = programs.0.texts.iter().map(|text| &*text.0);
    let named = names.kinds.iter().map(|kind| &*kind.0);
    let named = named.chain(names.fields.iter().map(|field| &*field.0));
    let texts: Vec<NameText<'_>> = constant.chain(named).map(NameText).collect();
    let stated: Vec<NameText<'_>> = names.fixed.iter().map(|name| NameText(&name.0)).collect();
    let words = worded(&stated, &texts)?;
    let NamedWords { kinds, fields } = named_words(&words, names);
    programs.0.words = words;
    programs.0.kinds = kinds;
    programs.0.fields = fields;
    Ok(programs)
}

/// The rules of loaded programs the executor cannot run whole: each can reach a word or a fill the executor does not run
/// yet (`executing::running::service::unit_not_run`, the executor's one answer over the code). Such a rule is refused like
/// an operation the lowering does not take, with the step of its own plan that unit makes the rows of, so it never
/// reaches a scan.
///
/// A rule that reaches an unrun unit only of another rule's plan (a sub-plan it reads of a producer whose own unit is not
/// run) is kept for the next round when some rule's unrun unit is its own: taking the owners out may leave it whole. When
/// every unrun unit is another rule's, all are taken out.
fn not_run(programs: &Programs) -> Vec<RefusedRule> {
    let (own, foreign): (Vec<_>, Vec<_>) = unrun_units(programs).into_iter().partition(|(_, own)| *own);
    let taken = if own.is_empty() { foreign } else { own };
    taken.into_iter().map(|(refused, _)| refused).collect()
}

/// Each rule the executor cannot run whole, with whether the unit not run is a step of the rule's own plan.
fn unrun_units(programs: &Programs) -> Vec<(RefusedRule, bool)> {
    let code = &programs.0.code;
    let refused = code.rules.iter().filter_map(|rule| {
        let unit = unit_not_run(code, rule)?;
        let key = &code
            .units
            .get(usize::try_from(unit.0).unwrap_or(usize::MAX))?
            .key;
        // Spike (ryo-fmx3.1): the unit not run, by its key, when asked.
        if std::env::var_os("RYO_L2_REFUSED").is_some() {
            eprintln!("ryo l2: rule {} reaches unit {} not run: {key:?}", rule.rule.0, unit.0);
        }
        let (UnitKey::Pattern(node, ..) | UnitKey::Rule(node) | UnitKey::Table(node, _)) = key;
        let own = programs
            .0
            .steps
            .iter()
            .find(|steps| steps.rule == rule.rule)
            .is_some_and(|steps| steps.nodes.0.contains(node));
        Some((
            RefusedRule {
                rule: rule.rule,
                refusal: LoadRefusal::Unlowerable(PlanNode(unit_step(programs, rule.rule, key).0)),
            },
            own,
        ))
    });
    refused.collect()
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

/// The rules an encoding's refusal takes out of a pack whose rules were written in `written` order. The encoding's own
/// fault takes the one rule it names. A count of the pack past its width takes the rule that was being written and every
/// rule after it, in one pass: taking one rule at a time would lower the pack once for each. None when the refusal names
/// no rule of the pack.
fn taken(
    written: &[SelectedRuleOrdinal],
    refused: SelectedRuleOrdinal,
    refusal: LoadRefusal,
) -> Vec<SelectedRuleOrdinal> {
    let after = written.iter().copied().skip_while(|rule| *rule != refused);
    match refusal {
        LoadRefusal::Budget(_) => after.collect(),
        LoadRefusal::Unlowerable(_) => after.take(1).collect(),
    }
}

/// Loads `rules` through `lower`: the lowered pack passed and encoded, and every rule that lowers, encodes and runs
/// whole loaded, whatever is refused beside it. Each refused rule is answered with its own cause, in rule order: the
/// lowering's refusal; the encoding's, held to the rule's own plan; or the step of its plan whose unit the executor
/// cannot run whole.
///
/// The rules are lowered until a lowering has nothing refused after it. Each lowering before that one is followed by at
/// least one rule taken out (when none can be, every rule left is refused and the load ends), so a load of `n` rules
/// lowers `n + 1` times at most. What it costs in practice: one lowering more for all the rules the executor cannot run
/// (they are found together), one more for each rule the encoding itself refuses, and one more for a count of the pack
/// past its width (the rule being written and every rule after it go together).
///
/// The rules loaded do not depend on the order they were selected in, save by a count past a width: the lowering, the
/// encoding's own fault and the executor's walk each refuse a rule for what that rule holds, the others are lowered
/// again without it, and the last lowering is of exactly the rules no cause was found for. A count past a width is the
/// pack's, not a rule's: which rules fit before it runs out is their order's.
///
/// # Errors
///
/// `Vocabulary` when the language's fixed names cannot be words of the run. With no statistics no unit is matched or unmatched: the
/// answer's `matched` is `Absent`.
pub(super) fn sequenced(
    rules: SelectedRules<'_>,
    statistics: Option<&Statistics>,
    names: &KindVocabulary,
    lower: &dyn Fn(SelectedRules<'_>) -> LoweredRules,
) -> Result<LoadedRules, LoadError> {
    let mut selected: Vec<SelectedRule<'_>> = rules.0.to_vec();
    let mut refused: Vec<RefusedRule> = Vec::new();
    let (mut loaded, mut named) = (Programs::default(), MatchedStatistics::Absent);
    let plan_of = |rule: SelectedRuleOrdinal| rules.0.iter().find(|each| each.rule == rule);
    loop {
        let LoweredRules {
            mut pack,
            refused: unlowered,
        } = lower(SelectedRules(&selected));
        refused.extend(unlowered);
        passed(&mut pack, statistics);
        // PROTOTYPE (ryo-fmx3.1, join order): the passed pack, one line a table, definition and rule, when asked.
        if let Some(path) = std::env::var_os("RYO_ENGINE_DUMP_PACK") {
            let mut text = String::new();
            for (at, table) in pack.tables.iter().enumerate() {
                text.push_str(&format!("T{at} {table:?}\n"));
            }
            for (at, definition) in pack.definitions.iter().enumerate() {
                text.push_str(&format!("D{at} {definition:?}\n"));
            }
            for rule in &pack.rules {
                text.push_str(&format!("R{} {rule:?}\n", rule.rule.0));
            }
            let _written = std::fs::write(path, text);
        }
        let written: Vec<SelectedRuleOrdinal> = pack.rules.iter().map(|rule| rule.rule).collect();
        // What this lowering's pack is refused for, and the rules that takes out.
        let (cause, gone) = match encoded(&pack) {
            Ok(code) => {
                // PROTOTYPE (ryo-fmx3.1, join order): each made table's unit, as the ledger numbers it, when asked.
                if let Some(path) = std::env::var_os("RYO_ENGINE_DUMP_PACK") {
                    let mut text = String::new();
                    for (at, table) in code.tables.iter().enumerate() {
                        text.push_str(&format!("U{} T{at}\n", table.unit.0));
                    }
                    let mut path = std::path::PathBuf::from(path);
                    path.set_extension("units");
                    let _written = std::fs::write(path, text);
                }
                let steps = rule_steps(&code, rules);
                let lowered = programs(pack, code, steps, names)?;
                let unrun = not_run(&lowered);
                // Spike (ryo-fmx3.1): the rules the executor cannot run whole, with the unit's step, when asked.
                if std::env::var_os("RYO_L2_REFUSED").is_some() {
                    for each in &unrun {
                        eprintln!("ryo l2: not run whole: rule {} at {}", each.rule.0, each.refusal);
                    }
                }
                let Some(first) = unrun.first() else {
                    named = statistics.map_or(MatchedStatistics::Absent, |measured| {
                        MatchedStatistics::Units(matched(measured, &lowered.0.code.units))
                    });
                    loaded = lowered;
                    break;
                };
                // Every rule the executor cannot run whole is taken out at once, each with its own unit's step.
                (first.refusal, unrun)
            }
            Err(LoweringError::Refused { rule, refusal }) => {
                // Spike (ryo-fmx3.1): the encoding's refusal as it names the rule, when asked.
                if std::env::var_os("RYO_L2_REFUSED").is_some() {
                    eprintln!("ryo l2: encoding refused rule {} at {refusal}", rule.0);
                }
                let gone = taken(&written, rule, refusal).into_iter();
                let gone = gone.map(|each| RefusedRule {
                    rule: each,
                    refusal: held(refusal, plan_of(each)),
                });
                (refusal, gone.collect())
            }
        };
        let before = selected.len();
        selected.retain(|rule| {
            written.contains(&rule.rule) && !gone.iter().any(|each| each.rule == rule.rule)
        });
        if selected.len() == before {
            // The refusal took no rule out (it names none of the pack): it is the pack's own, and no rule is run on it.
            // Every rule left is refused with it, held to its own plan, and nothing is lowered again.
            refused.extend(selected.iter().map(|rule| RefusedRule {
                rule: rule.rule,
                refusal: held(cause, Some(rule)),
            }));
            break;
        }
        refused.extend(gone);
    }
    refused.sort_by_key(|rule| rule.rule);
    Ok(LoadedRules {
        programs: loaded,
        refused,
        matched: named,
    })
}
