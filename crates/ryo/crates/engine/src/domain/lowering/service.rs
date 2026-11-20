//! The `lowering` service: the selected rules' plans into one lowered pack (scope-engine3 2.6, track L1). The one lowering:
//! every rule is admitted or refused step by step; the accepted rules' steps are flattened once each, a sub-plan several
//! read being one definition for the pack; each rule's branches and each pattern asked of a definition are planned and
//! written once; and the pack is proved before it is answered. No rule is refused for what it costs or for how large it is.

use super::costing::constants::GROUP_ROWS;
use super::costing::service::branches_rows;
use super::definitions::service::{defined, use_branch};
use super::definitions::types::{
    DefinitionDraft, Drafts, Flattened, RowEstimate, WaitingPattern,
};
use super::emptiness::service::copied;
use super::flattening::types::{FlatAtom, FlatBranch, FlatTerm, FlatVariable};
use super::nodes::service::{digest_of, stored};
use super::nodes::types::{NodeHome, NodeStore, AdmittedRule};
use super::placement::service::{measured, placed, sampled};
use super::placement::types::{CallForm, FormChoice, PatternFacts, SampleStage, TableForm};
use super::planning::emitting::service::indexed;
use super::planning::service::planned;
use super::planning::types::UnitRole;
use super::planning::writing::service::{
    PatternColumns, branch_program, pattern_program, pattern_whole, pipeline,
};
use super::proving::service::proved;
use super::proving::types::{ProductStated, ProofFault, ProvedUnit, Unproved};
use super::tables::arms::service::rule_flattened;
use super::tables::service::admitted;
use super::tables::types::RuleFlattening;
use super::types::{LoadRefusal, LoweredRules, PlanNode, RefusedRule, SelectedRules};
use crate::domain::code::types::UnitKey;
use crate::domain::findings::types::{ColumnKinds, RuleColumns};
use crate::domain::lowered::types::{
    Branch, DefinitionPattern, Emptiness, LoweredPack, LoweredRule, MadeFill, MadeTable,
    PackDefinition, PatternForm,
};
use crate::domain::statistics::types::Statistics;
use crate::domain::steps::types::{
    ColumnMask, DefinitionId, EmitTarget, MadeTableId, Polarity, ProgramStep, ReadColumn,
    RegisterCount, RegisterList, StepBody, StepProgram, StepRows,
};
use ryo_values::domain::descriptions::types::KindVocabulary;
use ryo_values::domain::plan_parts::types::{RepeatedRows, StepId};
use ryo_values::domain::table_words::types::ColumnCount;
use std::collections::HashSet;

/// A rule the build of a pack refuses: its place among the accepted rules, and why.
type Refused = (usize, LoadRefusal);

/// What a load reads of its statistics while it lowers.
#[derive(Clone, Copy)]
struct Sample<'a> {
    /// What they are of.
    stage: SampleStage,
    /// The statistics.
    statistics: &'a Statistics,
    /// The language's kinds.
    kinds: &'a KindVocabulary,
}

/// The refusal of a step that cannot be evaluated.
fn unlowerable(step: StepId) -> LoadRefusal {
    LoadRefusal::Unlowerable(PlanNode(step.0))
}

/// The refusal of the rule a definition was lowered from, naming the definition's step.
fn home_refused(home: NodeHome) -> Refused {
    (home.rule, unlowerable(home.step))
}

/// A definition's draft, by number.
fn draft_at(drafts: &Drafts, definition: DefinitionId) -> Option<&DefinitionDraft> {
    drafts
        .definitions
        .get(usize::try_from(definition.0).unwrap_or(usize::MAX))
}

/// The table of a definition's own rows, made the first time a pattern of it takes the table form: one pipeline a branch,
/// repeats dropped at its write.
fn definition_table(
    drafts: &mut Drafts,
    sample: Sample<'_>,
    definition: DefinitionId,
) -> Result<MadeTableId, Refused> {
    let place = usize::try_from(definition.0).unwrap_or(usize::MAX);
    let Some(draft) = drafts.definitions.get(place) else {
        return Err((0, unlowerable(StepId(0))));
    };
    if let Some(table) = draft.table {
        return Ok(table);
    }
    let (home, node, width) = (draft.home, draft.node, draft.shape.0.len());
    let branches: Vec<FlatBranch> = draft.branches.clone();
    let mut pipelines = Vec::with_capacity(branches.len());
    for branch in &branches {
        let written = planned(drafts, branch, &HashSet::new(), UnitRole::Pipeline)
            .and_then(|plan| pipeline(drafts, sample.kinds, branch, &plan, Some(definition)));
        pipelines.push(written.ok_or_else(|| home_refused(home))?);
    }
    let table = MadeTableId(u32::try_from(drafts.tables.len()).unwrap_or(u32::MAX));
    drafts.tables.push(MadeTable {
        node,
        fill: MadeFill::Rows(pipelines.into(), RepeatedRows::Dropped),
        width: ColumnCount(width),
        indexes: Vec::new(),
        emptiness: Emptiness::Definition(definition),
    });
    drafts.table_homes.push(home);
    if let Some(draft) = drafts.definitions.get_mut(place) {
        draft.table = Some(table);
    }
    Ok(table)
}

/// The variables a pattern's bound columns give a branch.
fn given(branch: &FlatBranch, bound: &ColumnMask) -> HashSet<FlatVariable> {
    let cells = bound
        .0
        .iter()
        .filter_map(|column| branch.row.get(usize::from(column.0)));
    cells
        .filter_map(|cell| match cell {
            FlatTerm::Variable(variable) => Some(*variable),
            FlatTerm::Text(_) | FlatTerm::Number(_) | FlatTerm::File => None,
        })
        .collect()
}

/// Places one pattern asked of a definition and writes it in its form: its program, or its definition's table.
fn pattern_placed(
    drafts: &mut Drafts,
    sample: Sample<'_>,
    waiting: WaitingPattern,
) -> Result<(), Refused> {
    let missing = (0, unlowerable(StepId(0)));
    let Some(draft) = draft_at(drafts, waiting.definition) else {
        return Err(missing);
    };
    let Some(pattern) = draft.patterns.get(waiting.pattern) else {
        return Err(missing);
    };
    let (bound, collected) = (pattern.bound.clone(), pattern.collected.clone());
    let (home, node, formals) = (draft.home, draft.node, draft.formals.clone());
    let branches: Vec<FlatBranch> = draft.branches.clone();
    let calls = draft.patterns.iter();
    let call_patterns = calls
        .filter(|pattern| matches!(pattern.form, Some(PatternForm::Call(_))))
        .count();
    let walks = branches.iter().flat_map(|branch| &branch.atoms);
    let table = if walks.clone().any(|atom| matches!(atom, FlatAtom::Nearest { .. })) {
        TableForm::Calls
    } else if draft.keyless.is_empty() {
        TableForm::Keyed
    } else {
        TableForm::Product
    };
    // A pattern has a program when every branch has one with the bound columns given: every access keyed.
    let plans: Option<Vec<_>> = branches
        .iter()
        .map(|branch| planned(drafts, branch, &given(branch, &bound), UnitRole::Pattern))
        .collect();
    let facts = PatternFacts {
        binds: bound.0.len(),
        call: match &plans {
            Some(_) => CallForm::Keyed,
            None => CallForm::Scans,
        },
        table,
        call_patterns,
        measured: measured(
            sample.statistics,
            &UnitKey::Pattern(node, bound.clone(), collected.clone()),
        ),
    };
    let placement = placed(facts, sample.stage);
    let form = match (placement.form, plans) {
        (FormChoice::Call, Some(plans)) => {
            let columns = PatternColumns {
                definition: waiting.definition,
                formals: &formals,
                bound: &bound,
                collected: &collected,
            };
            let mut programs = Vec::with_capacity(branches.len());
            for (branch, plan) in branches.iter().zip(&plans) {
                let written = pattern_program(drafts, sample.kinds, branch, plan, columns);
                programs.push(written.ok_or_else(|| home_refused(home))?);
            }
            PatternForm::Call(pattern_whole(programs))
        }
        (FormChoice::Table, _) | (FormChoice::Call, None) => {
            let table = definition_table(drafts, sample, waiting.definition)?;
            let listed: Vec<ReadColumn> = bound.0.iter().map(|column| ReadColumn::Made(*column)).collect();
            if !listed.is_empty() {
                indexed(drafts, table, &listed);
            }
            PatternForm::Table(table)
        }
    };
    let place = usize::try_from(waiting.definition.0).unwrap_or(usize::MAX);
    let slot = drafts
        .definitions
        .get_mut(place)
        .and_then(|draft| draft.patterns.get_mut(waiting.pattern));
    if let Some(slot) = slot {
        slot.form = Some(form);
        slot.shape = placement.shape;
    }
    Ok(())
}

/// One rule's own program: a branch of its emitted rows each. When a branch has no program (an access of it has no key
/// after its anchor), the emitted step is a definition and the rule walks its table.
fn rule_lowered(
    drafts: &mut Drafts,
    sample: Sample<'_>,
    rule: &AdmittedRule<'_>,
    place: usize,
    mut flattened: Flattened,
) -> Result<LoweredRule, Refused> {
    let emitted = rule.rule.plan.emit.step;
    let refused = (place, unlowerable(emitted));
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
        branches.push(Branch {
            emptiness: copied(&flattened.emptiness),
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
                bound: pattern.bound,
                collected: pattern.collected,
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
        registers: RegisterCount(drafts.registers),
        regexes: drafts.regexes,
        texts: drafts.texts,
        reads: drafts.reads.into_iter().collect(),
    }
}

/// The tables a rule's branch cannot give a row without: each table it reads rows of, by a made table's rows, by a
/// definition's rows whole, or by a use in table form that asks for a row.
fn tables_needed(pack: &LoweredPack, program: &StepProgram) -> Vec<MadeTableId> {
    let of_definition = |definition: DefinitionId| {
        let place = usize::try_from(definition.0).unwrap_or(usize::MAX);
        let patterns = pack.definitions.get(place).map(|definition| &definition.patterns);
        patterns.into_iter().flatten()
    };
    let used = |body: &StepBody| match body {
        StepBody::Definition {
            definition,
            bound,
            collected,
            ..
        } => of_definition(*definition)
            .find(|pattern| pattern.bound == *bound && pattern.collected == *collected)
            .and_then(|pattern| match pattern.form {
                PatternForm::Table(table) => Some(table),
                PatternForm::Call(_) => None,
            }),
        StepBody::Inline(_) => None,
    };
    let read = |source: &StepRows| match source {
        StepRows::Made(table) => Some(*table),
        StepRows::Definition(definition) => {
            of_definition(*definition).find_map(|pattern| match pattern.form {
                PatternForm::Table(table) => Some(table),
                PatternForm::Call(_) => None,
            })
        }
        StepRows::Node(_) | StepRows::Filled(_) | StepRows::Written(_) => None,
    };
    let mut needed = Vec::new();
    for placed in &program.0 {
        let table = match &placed.step {
            ProgramStep::EachRow { source, .. } | ProgramStep::EachMatch { source, .. } => {
                read(source)
            }
            ProgramStep::Latest { table, .. } => Some(*table),
            ProgramStep::Or { bodies, .. } => bodies.iter().find_map(used),
            ProgramStep::Exists {
                polarity: Polarity::Present,
                body,
                ..
            } => used(body),
            ProgramStep::Exists { .. }
            | ProgramStep::EachKind { .. }
            | ProgramStep::EachRange { .. }
            | ProgramStep::Children { .. }
            | ProgramStep::Axis { .. }
            | ProgramStep::Within { .. }
            | ProgramStep::Nearest { .. }
            | ProgramStep::Closure { .. }
            | ProgramStep::Read { .. }
            | ProgramStep::Test { .. }
            | ProgramStep::Call { .. }
            | ProgramStep::Render { .. }
            | ProgramStep::Let { .. }
            | ProgramStep::Fork { .. }
            | ProgramStep::Yield { .. }
            | ProgramStep::Emit { .. } => None,
        };
        needed.extend(table.filter(|table| !needed.contains(table)));
    }
    needed
}

/// Gives each rule's branch its whole need: the formula of its rows, then each table it cannot give a row without, the
/// smallest estimate first. The order is fixed at load: a part that needs no table is decided before any table is made, and
/// no decision depends on what another rule built.
fn needs_added(pack: &mut LoweredPack, rows: &[RowEstimate]) {
    let estimate = |table: &MadeTableId| {
        let place = usize::try_from(table.0).unwrap_or(usize::MAX);
        (rows.get(place).copied().unwrap_or_default(), *table)
    };
    let mut needs: Vec<Vec<MadeTableId>> = Vec::new();
    for branch in pack.rules.iter().flat_map(|rule| &rule.branches) {
        let mut tables = tables_needed(pack, &branch.program);
        tables.sort_by_key(estimate);
        needs.push(tables);
    }
    let branches = pack.rules.iter_mut().flat_map(|rule| &mut rule.branches);
    for (branch, tables) in branches.zip(needs) {
        if tables.is_empty() {
            continue;
        }
        let formula = std::mem::take(&mut branch.emptiness);
        let parts = std::iter::once(formula).chain(tables.into_iter().map(Emptiness::Table));
        branch.emptiness = Emptiness::All(parts.collect());
    }
}

/// The rule a fault of the proof belongs to, with its refusal: the rule itself, or the rule its unit was lowered from.
fn fault_refused(drafts_homes: &(Vec<NodeHome>, Vec<NodeHome>), rules: &[AdmittedRule<'_>], unproved: Unproved) -> Refused {
    let at = |homes: &[NodeHome], number: u32| {
        homes
            .get(usize::try_from(number).unwrap_or(usize::MAX))
            .copied()
    };
    let home = match unproved.unit {
        ProvedUnit::Rule(place) => rules.get(place).map(|rule| NodeHome {
            rule: place,
            step: rule.rule.plan.emit.step,
        }),
        ProvedUnit::Definition(definition) => at(&drafts_homes.0, definition.0),
        ProvedUnit::Table(table) => at(&drafts_homes.1, table.0),
    };
    let home = home.unwrap_or(NodeHome {
        rule: 0,
        step: StepId(0),
    });
    match unproved.fault {
        ProofFault::Misplaced => home_refused(home),
        ProofFault::Past(width) => (home.rule, LoadRefusal::Budget(width)),
    }
}

/// The pack of the accepted rules, or the first rule the build refuses.
fn built(
    rules: &[AdmittedRule<'_>],
    store: &NodeStore,
    sample: Sample<'_>,
) -> Result<LoweredPack, Refused> {
    let mut drafts = Drafts::default();
    let mut flattened = Vec::with_capacity(rules.len());
    for place in 0..rules.len() {
        let mut state = RuleFlattening {
            rules,
            store,
            kinds: sample.kinds,
            place,
            flat: Vec::new(),
        };
        let rows = rule_flattened(&mut state, &mut drafts);
        flattened.push(rows.map_err(|step| (place, unlowerable(step)))?);
    }
    let mut lowered = Vec::with_capacity(rules.len());
    for (place, (rule, rows)) in rules.iter().zip(flattened).enumerate() {
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
    let homes = (
        drafts.definitions.iter().map(|draft| draft.home).collect(),
        drafts.table_homes.clone(),
    );
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
/// write. No rule is refused for what it costs or for how large it is.
pub(crate) fn lowered(
    rules: SelectedRules<'_>,
    statistics: &Statistics,
    kinds: &KindVocabulary,
) -> LoweredRules {
    lowered_at(rules, sampled(statistics), statistics, kinds)
}

/// The lowering at a stated sample stage: `lowered` reads the stage off the statistics.
pub(crate) fn lowered_at(
    rules: SelectedRules<'_>,
    stage: SampleStage,
    statistics: &Statistics,
    kinds: &KindVocabulary,
) -> LoweredRules {
    let sample = Sample {
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
            Err((place, refusal)) if place < accepted.len() => {
                let rule = accepted.remove(place);
                refused.push(RefusedRule {
                    rule: rule.rule.rule,
                    refusal,
                });
            }
            // A refusal that names no accepted rule is the build's own fault: no rule is run on it.
            Err((_, refusal)) => {
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
