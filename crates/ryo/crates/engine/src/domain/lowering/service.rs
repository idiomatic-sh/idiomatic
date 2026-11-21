//! The `lowering` service: the selected rules' plans into one lowered pack (scope-engine3 2.6, track L1). The one lowering:
//! every rule is admitted or refused step by step; the accepted rules' steps are flattened once each, a sub-plan several
//! read being one definition for the pack; each rule's branches and each pattern asked of a definition are planned and
//! written once; and the pack is proved before it is answered. No rule is refused for what it costs or for how large it is.

use super::costing::constants::GROUP_ROWS;
use super::costing::service::branches_rows;
use super::costing::types::SampleSizes;
use super::definitions::service::{defined, use_branch};
use super::definitions::types::{DefinitionDraft, Drafts, Flattened};
use super::emptiness::branches::service::{branch_need, with_need};
use super::emptiness::needs::service::needs_added;
use super::emptiness::service::copied;
use super::flattening::types::FlatBranch;
use super::nodes::service::{digest_of, stored};
use super::nodes::types::{AcceptedPlace, AdmittedRule, NodeHome, NodeStore};
use super::placement::patterns::service::{home_refused, pattern_placed, unlowerable};
use super::placement::service::sampled;
use super::placement::types::SampleStage;
use super::placement::types::{LoadSample, RefusedPlace};
use super::planning::service::planned;
use super::planning::types::UnitRole;
use super::planning::writing::service::branch_program;
use super::proving::service::proved;
use super::proving::types::{ProductStated, ProofFault, ProvedUnit, UnitHomes, Unproved};
use super::tables::arms::service::rule_flattened;
use super::tables::service::admitted;
use super::tables::types::RuleFlattening;
use super::types::{LoadRefusal, LoweredRules, RefusedRule, SelectedRules};
use crate::domain::findings::types::{ColumnKinds, RuleColumns};
use crate::domain::lowered::types::{
    Branch, DefinitionPattern, Emptiness, LoweredPack, LoweredRule, PackDefinition, PatternForm,
};
use crate::domain::statistics::types::Statistics;
use crate::domain::steps::types::{EmitTarget, RegisterList, StepProgram};
use ryo_values::domain::descriptions::types::KindVocabulary;
use ryo_values::domain::plan_parts::types::StepId;
use std::collections::HashSet;

/// One rule's own program: a branch of its emitted rows each. When a branch has no program (an access of it has no key
/// after its anchor), the emitted step is a definition and the rule walks its table.
fn rule_lowered(
    drafts: &mut Drafts,
    sample: LoadSample<'_>,
    rule: &AdmittedRule<'_>,
    place: AcceptedPlace,
    mut flattened: Flattened,
) -> Result<LoweredRule, RefusedPlace> {
    let emitted = rule.rule.plan.emit.step;
    let refused = RefusedPlace {
        rule: place,
        refusal: unlowerable(emitted),
    };
    let node = digest_of(rule, emitted);
    let shape = rule.shapes.get(usize::from(emitted)).cloned().flatten();
    let shape = shape.ok_or(refused)?;
    let unplanned = |drafts: &Drafts, branches: &[FlatBranch]| {
        let mut plans = branches
            .iter()
            .map(|branch| planned(drafts, branch, &HashSet::new(), UnitRole::Branch));
        plans.any(|plan| plan.is_none())
    };
    if unplanned(drafts, &flattened.branches) {
        let home = NodeHome {
            rule: place,
            step: emitted,
        };
        let rows = branches_rows(drafts, &flattened.branches);
        let definition = defined(drafts, node, home, shape.clone(), flattened, rows);
        flattened = Flattened {
            branches: vec![use_branch(drafts, definition)],
            emptiness: Emptiness::Definition(definition),
            keyless: Vec::new(),
        };
    }
    let target = EmitTarget::Rule(rule.rule.rule);
    let mut branches = Vec::with_capacity(flattened.branches.len());
    for branch in &flattened.branches {
        let program = planned(drafts, branch, &HashSet::new(), UnitRole::Branch)
            .and_then(|plan| branch_program(drafts, sample.kinds, branch, &plan, target));
        // The rule's formula, and what this branch itself needs.
        let need = branch_need(drafts, branch);
        branches.push(Branch {
            emptiness: with_need(copied(&flattened.emptiness), need),
            program: program.ok_or(refused)?,
        });
    }
    Ok(LoweredRule {
        rule: rule.rule.rule,
        node,
        branches,
        columns: RuleColumns {
            emit: ColumnKinds(shape.0),
            frontier: Vec::new(),
        },
    })
}

/// The drafts as a lowered pack: every pattern has its form by now.
fn assembled(drafts: Drafts, rules: Vec<LoweredRule>) -> LoweredPack {
    let definitions = drafts.definitions.into_iter().map(|draft| PackDefinition {
        node: draft.node,
        formals: RegisterList(draft.formals.into()),
        emptiness: draft.emptiness,
        patterns: draft
            .patterns
            .into_iter()
            .map(|pattern| DefinitionPattern {
                shape: pattern.shape,
                bound: pattern.asked.bound,
                collected: pattern.asked.collected,
                form: pattern
                    .form
                    .unwrap_or_else(|| PatternForm::Call(StepProgram::default())),
            })
            .collect(),
    });
    LoweredPack {
        definitions: definitions.collect(),
        tables: drafts.tables,
        rules,
        registers: drafts.registers,
        regexes: drafts.regexes,
        texts: drafts.texts,
        reads: drafts.reads.into_iter().collect(),
    }
}

/// The rule a fault of the proof belongs to, with its refusal: the rule itself, or the rule its unit was lowered from.
fn fault_refused(
    homes: &UnitHomes,
    rules: &[AdmittedRule<'_>],
    unproved: Unproved,
) -> RefusedPlace {
    let home = match unproved.unit {
        ProvedUnit::Rule(place) => rules.get(place.0).map(|rule| NodeHome {
            rule: place,
            step: rule.rule.plan.emit.step,
        }),
        ProvedUnit::Definition(definition) => {
            let place = usize::try_from(definition.0).unwrap_or(usize::MAX);
            homes.definitions.get(place).copied()
        }
        ProvedUnit::Table(table) => {
            let place = usize::try_from(table.0).unwrap_or(usize::MAX);
            homes.tables.get(place).copied()
        }
    };
    let home = home.unwrap_or(NodeHome {
        rule: AcceptedPlace(0),
        step: StepId(0),
    });
    match unproved.fault {
        ProofFault::Misplaced => home_refused(home),
        ProofFault::Past(width) => RefusedPlace {
            rule: home.rule,
            refusal: LoadRefusal::Budget(width),
        },
    }
}

/// The pack of the accepted rules, or the first rule the build refuses.
fn built(
    rules: &[AdmittedRule<'_>],
    store: &NodeStore,
    sample: LoadSample<'_>,
) -> Result<LoweredPack, RefusedPlace> {
    let mut drafts = Drafts {
        sized: sample.statistics.map(|statistics| SampleSizes {
            statistics: statistics.clone(),
            names: sample.kinds.clone(),
        }),
        ..Drafts::default()
    };
    let mut flattened = Vec::with_capacity(rules.len());
    for place in (0..rules.len()).map(AcceptedPlace) {
        let mut state = RuleFlattening {
            rules,
            store,
            kinds: sample.kinds,
            place,
            flat: Vec::new(),
        };
        let rows = rule_flattened(&mut state, &mut drafts);
        flattened.push(rows.map_err(|step| RefusedPlace {
            rule: place,
            refusal: unlowerable(step),
        })?);
    }
    let mut lowered = Vec::with_capacity(rules.len());
    for (place, (rule, rows)) in rules.iter().zip(flattened).enumerate() {
        let place = AcceptedPlace(place);
        lowered.push(rule_lowered(&mut drafts, sample, rule, place, rows)?);
    }
    // Each pattern asked is placed once; placing one may ask patterns of the definitions it reads.
    while let Some(waiting) = drafts.waiting.pop() {
        pattern_placed(&mut drafts, sample, waiting)?;
    }
    let stated = |definition: &DefinitionDraft| {
        if definition.keyless.is_empty() {
            ProductStated::Keyed
        } else {
            ProductStated::Stated
        }
    };
    let mut products = vec![ProductStated::Keyed; drafts.tables.len()];
    for definition in &drafts.definitions {
        let slot = definition
            .table
            .and_then(|table| products.get_mut(usize::try_from(table.0).unwrap_or(usize::MAX)));
        if let Some(slot) = slot {
            *slot = stated(definition);
        }
    }
    let homes = UnitHomes {
        definitions: drafts.definitions.iter().map(|draft| draft.home).collect(),
        tables: drafts.table_homes.clone(),
    };
    // What each table is taken to hold: a definition's own estimate for the table of its rows, a group's for any other.
    let mut table_rows = vec![GROUP_ROWS; drafts.tables.len()];
    for definition in &drafts.definitions {
        let slot = definition
            .table
            .and_then(|table| table_rows.get_mut(usize::try_from(table.0).unwrap_or(usize::MAX)));
        if let Some(slot) = slot {
            *slot = definition.rows;
        }
    }
    let mut pack = assembled(drafts, lowered);
    needs_added(&mut pack, &table_rows);
    proved(&pack, &products).map_err(|unproved| fault_refused(&homes, rules, unproved))?;
    Ok(pack)
}

/// The pack of the rules the lowering accepts, and each rule it refuses with the step it cannot evaluate or the count it cannot
/// write. No rule is refused for what it costs or for how large it is. `statistics` is none for a load given none: every flip
/// candidate then takes its call form; given those of a first sample, each takes its table form.
pub(crate) fn lowered(
    rules: SelectedRules<'_>,
    statistics: Option<&Statistics>,
    kinds: &KindVocabulary,
) -> LoweredRules {
    lowered_at(rules, sampled(statistics), statistics, kinds)
}

/// The lowering at a stated sample stage: `lowered` reads the stage off the statistics.
pub(crate) fn lowered_at(
    rules: SelectedRules<'_>,
    stage: SampleStage,
    statistics: Option<&Statistics>,
    kinds: &KindVocabulary,
) -> LoweredRules {
    let sample = LoadSample {
        stage,
        statistics,
        kinds,
    };
    let mut refused = Vec::new();
    let mut accepted = Vec::with_capacity(rules.0.len());
    for rule in rules.0 {
        match admitted(*rule) {
            Ok(nodes) => accepted.push(nodes),
            Err(refusal) => refused.push(RefusedRule {
                rule: rule.rule,
                refusal,
            }),
        }
    }
    // A rule the build refuses is taken out and the others are lowered again: a definition is never left half made.
    let pack = loop {
        let store = stored(&accepted);
        match built(&accepted, &store, sample) {
            Ok(pack) => break pack,
            Err(RefusedPlace { rule, refusal }) if rule.0 < accepted.len() => {
                let rule = accepted.remove(rule.0);
                refused.push(RefusedRule {
                    rule: rule.rule.rule,
                    refusal,
                });
            }
            // A refusal that names no accepted rule is the build's own fault: no rule is run on it.
            Err(RefusedPlace { refusal, .. }) => {
                refused.extend(accepted.iter().map(|rule| RefusedRule {
                    rule: rule.rule.rule,
                    refusal,
                }));
                break LoweredPack::default();
            }
        }
    };
    refused.sort_by_key(|rule| rule.rule);
    LoweredRules { pack, refused }
}
