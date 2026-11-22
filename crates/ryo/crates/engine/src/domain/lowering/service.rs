//! The `lowering` service: the selected rules' plans into one lowered pack (scope-engine3 2.6, track L1). The one lowering:
//! every rule is admitted or refused step by step; the accepted rules' steps are flattened once each, a sub-plan several
//! read being one definition for the pack; each rule's branches and each pattern asked of a definition are planned and
//! written once; and the pack is proved before it is answered. No rule is refused for what it costs or for how large it is.

use super::costing::constants::GROUP_ROWS;
use super::costing::service::definition_rows;
use super::costing::types::SampleSizes;
use super::definitions::service::{defined, use_branch};
use super::definitions::types::{DefinitionDraft, Drafts, Flattened, WaitingPattern};
use super::emptiness::branches::service::{branch_need, with_need};
use super::emptiness::needs::service::needs_added;
use super::emptiness::service::copied;
use super::flattening::types::FlatBranch;
use super::nodes::service::{digest_of, stored};
use super::nodes::types::{AcceptedPlace, AdmittedRule, NodeHome, NodeStore};
use super::observing::service::{reads_observations, resolved};
use super::placement::patterns::service::{
    home_refused, pattern_placed, pattern_plans, unlowerable,
};
use super::placement::service::sampled;
use super::placement::types::SampleStage;
use super::placement::types::{LoadSample, RefusedPlace};
use super::planning::service::planned;
use super::planning::types::{PlannedBranch, UnitRole};
use super::planning::writing::service::branch_program;
use super::proving::service::proved;
use super::proving::types::{ProductStated, ProofFault, ProvedUnit, UnitHomes, Unproved};
use super::sharing::service::prefixes_shared;
use super::tables::arms::service::rule_flattened;
use super::tables::inputs::service::step_defined;
use super::tables::service::admitted_in;
use super::tables::types::RuleFlattening;
use super::types::{LoadGiven, LoadRefusal, LoweredRules, RefusedRule, SelectedRules};
use crate::domain::findings::types::{ColumnKinds, RuleColumns};
use crate::domain::lowered::types::{
    Branch, DefinitionPattern, Emptiness, LoweredPack, LoweredRule, PackDefinition, PatternForm,
};
use crate::domain::statistics::types::Statistics;
use crate::domain::steps::types::{DefinitionId, EmitTarget, RegisterList, StepProgram};
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
use ryo_values::domain::descriptions::types::KindVocabulary;
use ryo_values::domain::plan_parts::types::StepId;
use std::collections::{HashMap, HashSet};

/// The fewest patterns waiting together whose plans are made on the pool.
const POOLED_PATTERNS: usize = 16;

/// The plan of each branch of a rule's own program, nothing given. A plan reads the drafts and writes nothing.
fn branch_plans(drafts: &Drafts, branches: &[FlatBranch]) -> Vec<Option<PlannedBranch>> {
    let plans = branches
        .iter()
        .map(|branch| planned(drafts, branch, &HashSet::new(), UnitRole::Branch));
    plans.collect()
}

/// Places every pattern that waits, in the order the drafts hand them (the last asked first; placing one may ask more).
/// A pattern's plans read the drafts and nothing a placement writes, so the plans of every pattern that waits are made
/// together on the pool, and each is placed with its own when its turn comes: the pack is the one a pattern at a time
/// makes.
fn patterns_placed(drafts: &mut Drafts, sample: LoadSample<'_>) -> Result<(), RefusedPlace> {
    let mut plans: HashMap<WaitingPattern, Option<Vec<PlannedBranch>>> = HashMap::new();
    // How many of the patterns that wait, from the first asked, have their plans: the ones asked since stand after them.
    let mut planned = 0;
    while let Some(waiting) = drafts.waiting.last().copied() {
        if planned < drafts.waiting.len() {
            let shared: &Drafts = drafts;
            let asked = shared.waiting.get(planned..).unwrap_or_default();
            // The patterns the rules' own programs ask wait together, and are planned on the pool; the few a placement
            // then asks are planned where they are: handing two or three plans to the pool costs more than making them.
            let made: Vec<Option<Vec<PlannedBranch>>> = if asked.len() < POOLED_PATTERNS {
                asked
                    .iter()
                    .map(|each| pattern_plans(shared, *each))
                    .collect()
            } else {
                asked
                    .par_iter()
                    .map(|each| pattern_plans(shared, *each))
                    .collect()
            };
            plans.extend(asked.iter().copied().zip(made));
        }
        let _placing = drafts.waiting.pop();
        planned = drafts.waiting.len();
        // A pattern whose plans were handed out already (it waited twice) is planned again where it is.
        let own = match plans.remove(&waiting) {
            Some(own) => own,
            None => pattern_plans(drafts, waiting),
        };
        pattern_placed(drafts, sample, waiting, own)?;
    }
    if crate::domain::walk::on("report") {
        let patterns = drafts.definitions.iter().flat_map(|draft| &draft.patterns);
        let (mut calls, mut tables, mut fill_calls) = (0_usize, 0_usize, 0_usize);
        for pattern in patterns {
            match pattern.form {
                Some(crate::domain::lowered::types::PatternForm::Call(_)) => {
                    calls = calls.saturating_add(1);
                    fill_calls = fill_calls.saturating_add(usize::from(pattern.fill_asked));
                }
                Some(crate::domain::lowered::types::PatternForm::Table(_)) => {
                    tables = tables.saturating_add(1);
                }
                None => {}
            }
        }
        eprintln!(
            "ryo walk: patterns call={calls} table={tables} call-asked-by-a-fill={fill_calls} late-fill-asks={}",
            drafts.late_fill_asks
        );
    }
    Ok(())
}

/// One rule's own program: a branch of its emitted rows each. When a branch has no program (an access of it has no key
/// after its anchor), the emitted step is a definition and the rule walks its table.
fn rule_lowered(
    drafts: &mut Drafts,
    sample: LoadSample<'_>,
    rule: &AdmittedRule<'_>,
    place: AcceptedPlace,
    mut flattened: Flattened,
    mut plans: Vec<Option<PlannedBranch>>,
) -> Result<LoweredRule, RefusedPlace> {
    let emitted = rule.rule.plan.emit.step;
    let refused = RefusedPlace {
        rule: place,
        refusal: unlowerable(emitted),
    };
    let node = digest_of(rule, emitted);
    let shape = rule.shapes.get(usize::from(emitted)).cloned().flatten();
    let shape = shape.ok_or(refused)?;
    if plans.iter().any(Option::is_none) {
        let home = NodeHome {
            rule: place,
            step: emitted,
        };
        let definition = defined(drafts, node, home, shape.clone(), flattened);
        flattened = Flattened {
            branches: vec![use_branch(drafts, definition)],
            emptiness: Emptiness::Definition(definition),
            keyless: Vec::new(),
        };
        plans = branch_plans(drafts, &flattened.branches);
    }
    let target = EmitTarget::Rule(rule.rule.rule);
    let mut branches = Vec::with_capacity(flattened.branches.len());
    for (branch, plan) in flattened.branches.iter().zip(plans) {
        let program =
            plan.and_then(|plan| branch_program(drafts, sample.kinds, branch, &plan, target));
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
    // A rule that reads a bound observation is flattened after the others: its producers' observed steps are definitions
    // by then. Each producer's observed steps are defined as it is flattened.
    let mut order: Vec<usize> = (0..rules.len()).collect();
    order.sort_by_key(|place| rules.get(*place).is_some_and(reads_observations));
    let mut flattened: Vec<Option<Flattened>> = (0..rules.len()).map(|_| None).collect();
    for place in order.into_iter().map(AcceptedPlace) {
        let mut state = RuleFlattening {
            rules,
            store,
            kinds: sample.kinds,
            place,
            flat: Vec::new(),
        };
        let refused = |step| RefusedPlace {
            rule: place,
            refusal: unlowerable(step),
        };
        let rows = rule_flattened(&mut state, &mut drafts).map_err(refused)?;
        for root in rules.get(place.0).map_or(&[][..], |rule| &rule.observed) {
            let _defined = step_defined(&mut state, &mut drafts, *root).ok_or(refused(*root))?;
        }
        if let Some(slot) = flattened.get_mut(place.0) {
            *slot = Some(rows);
        }
    }
    let flattened: Vec<Flattened> = flattened
        .into_iter()
        .map(Option::unwrap_or_default)
        .collect();
    // Every rule's branches are planned on the pool: a plan reads the drafts as the flattening left them. The programs
    // are then written a rule at a time, in rule order.
    let planned: Vec<Vec<Option<PlannedBranch>>> = {
        let shared: &Drafts = &drafts;
        flattened
            .par_iter()
            .map(|rows: &Flattened| branch_plans(shared, &rows.branches))
            .collect()
    };
    let mut lowered = Vec::with_capacity(rules.len());
    for (place, ((rule, rows), plans)) in rules.iter().zip(flattened).zip(planned).enumerate() {
        let place = AcceptedPlace(place);
        lowered.push(rule_lowered(&mut drafts, sample, rule, place, rows, plans)?);
    }
    // Each pattern asked is placed once; placing one may ask patterns of the definitions it reads.
    patterns_placed(&mut drafts, sample)?;
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
    let mut homes = UnitHomes {
        definitions: drafts.definitions.iter().map(|draft| draft.home).collect(),
        tables: drafts.table_homes.clone(),
    };
    // What each table is taken to hold: a definition's own estimate for the table of its rows, a group's for any other.
    let mut table_rows = vec![GROUP_ROWS; drafts.tables.len()];
    for (number, definition) in drafts.definitions.iter().enumerate() {
        let slot = definition
            .table
            .and_then(|table| table_rows.get_mut(usize::try_from(table.0).unwrap_or(usize::MAX)));
        if let Some(slot) = slot {
            let definition = DefinitionId(u32::try_from(number).unwrap_or(u32::MAX));
            *slot = definition_rows(&drafts, definition);
        }
    }
    let mut pack = assembled(drafts, lowered);
    needs_added(&mut pack, &table_rows);
    // SPIKE: the run of stages several pipelines of a table begin with is run once, as a table of its own; such a table
    // is its table's as far as a product's statement and a fault's home go.
    for shared in prefixes_shared(&mut pack) {
        let of = usize::try_from(shared.of.0).unwrap_or(usize::MAX);
        products.push(products.get(of).copied().unwrap_or(ProductStated::Keyed));
        if let Some(home) = homes.tables.get(of).copied() {
            homes.tables.push(home);
        }
    }
    proved(&pack, &products).map_err(|unproved| fault_refused(&homes, rules, unproved))?;
    Ok(pack)
}

/// The pack of the rules the lowering accepts, and each rule it refuses with the step it cannot evaluate or the count it cannot
/// write. No rule is refused for what it costs or for how large it is. `statistics` is none for a load given none: every flip
/// candidate then takes its table form, as given those of a first sample; a second sample's say each unit's form.
pub(crate) fn lowered(
    rules: SelectedRules<'_>,
    given: LoadGiven<'_>,
    statistics: Option<&Statistics>,
    kinds: &KindVocabulary,
) -> LoweredRules {
    lowered_at(rules, given, sampled(statistics), statistics, kinds)
}

/// The lowering at a stated sample stage: `lowered` reads the stage off the statistics.
pub(crate) fn lowered_at<'a>(
    rules: SelectedRules<'a>,
    given: LoadGiven<'a>,
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
        match admitted_in(*rule, given) {
            Ok(nodes) => accepted.push(nodes),
            Err(refusal) => refused.push(RefusedRule {
                rule: rule.rule,
                refusal,
            }),
        }
    }
    // Each bound observation's producers among the accepted rules, their observed steps shaped.
    resolved(&mut accepted);
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
