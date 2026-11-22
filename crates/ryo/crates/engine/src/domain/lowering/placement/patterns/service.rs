//! The services of the `patterns` part: one pattern asked of a definition placed and written in its form: its program, or its definition's table.

use crate::domain::code::types::UnitKey;
use crate::domain::lowered::types::{ConstantRows, Emptiness, MadeFill, MadeTable, PatternForm};
use crate::domain::lowering::definitions::service::text_word;
use ryo_language::domain::facts::types::CellWord;
use crate::domain::lowering::definitions::types::{DefinitionDraft, Drafts, WaitingPattern};
use crate::domain::lowering::flattening::service::own_walks;
use crate::domain::lowering::flattening::types::{FlatBranch, FlatTerm, FlatVariable, OwnWalks};
use crate::domain::lowering::nodes::types::AcceptedPlace;
use crate::domain::lowering::nodes::types::NodeHome;
use crate::domain::lowering::placement::service::{measured, placed};
use crate::domain::lowering::placement::types::{
    FormChoice, PatternFacts, PatternProgram, TableForm,
};
use crate::domain::lowering::placement::types::{LoadSample, RefusedPlace};
use crate::domain::lowering::planning::emitting::service::indexed;
use crate::domain::lowering::planning::service::planned;
use crate::domain::lowering::planning::types::UnitRole;
use crate::domain::lowering::planning::types::{PatternColumns, PlannedBranch};
use crate::domain::lowering::planning::writing::service::{
    pattern_program, pattern_whole, pipeline,
};
use crate::domain::lowering::types::{LoadRefusal, PlanNode};
use crate::domain::steps::types::{ColumnMask, DefinitionId, MadeTableId, ReadColumn};
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::plan_parts::types::{RepeatedRows, StepId};
use ryo_values::domain::table_words::types::ColumnCount;
use std::collections::HashSet;

/// The refusal of a step that cannot be evaluated.
pub(crate) fn unlowerable(step: StepId) -> LoadRefusal {
    LoadRefusal::Unlowerable(PlanNode(step.0))
}

/// The refusal of the rule a definition was lowered from, naming the definition's step.
pub(crate) fn home_refused(home: NodeHome) -> RefusedPlace {
    RefusedPlace {
        rule: home.rule,
        refusal: unlowerable(home.step),
    }
}

/// A definition's draft, by number.
pub(crate) fn draft_at(drafts: &Drafts, definition: DefinitionId) -> Option<&DefinitionDraft> {
    drafts
        .definitions
        .get(usize::try_from(definition.0).unwrap_or(usize::MAX))
}

/// The table of a definition's own rows, made the first time a pattern of it takes the table form: one pipeline a branch,
/// repeats dropped at its write.
pub(crate) fn definition_table(
    drafts: &mut Drafts,
    sample: LoadSample<'_>,
    definition: DefinitionId,
) -> Result<MadeTableId, RefusedPlace> {
    let place = usize::try_from(definition.0).unwrap_or(usize::MAX);
    let Some(draft) = drafts.definitions.get(place) else {
        return Err(RefusedPlace {
            rule: AcceptedPlace(0),
            refusal: unlowerable(StepId(0)),
        });
    };
    if let Some(table) = draft.table {
        return Ok(table);
    }
    let (home, node, width) = (draft.home, draft.node, draft.shape.0.len());
    let branches: Vec<FlatBranch> = draft.branches.clone();
    // A definition of constant rows (each branch reads nothing and its row is constants) is a table of those rows, known
    // at load: no pipeline would read a source to write them.
    let fill = if let Some(rows) = constant_rows(&branches) {
        MadeFill::Constant(rows)
    } else {
        let mut pipelines = Vec::with_capacity(branches.len());
        for branch in &branches {
            let written = planned(drafts, branch, &HashSet::new(), UnitRole::Pipeline)
                .and_then(|plan| pipeline(drafts, sample.kinds, branch, &plan, Some(definition)));
            pipelines.push(written.ok_or_else(|| home_refused(home))?);
        }
        MadeFill::Rows(pipelines.into(), RepeatedRows::Dropped)
    };
    let table = MadeTableId(u32::try_from(drafts.tables.len()).unwrap_or(u32::MAX));
    drafts.tables.push(MadeTable {
        node,
        fill,
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

/// The rows of a definition whose every branch reads nothing and holds constants alone, each once, as the cells a table
/// holds: a text's word, a number's bits. None for any other definition.
fn constant_rows(branches: &[FlatBranch]) -> Option<ConstantRows> {
    let mut rows: Vec<Box<[CellWord]>> = Vec::with_capacity(branches.len());
    for branch in branches {
        if !branch.atoms.is_empty() {
            return None;
        }
        let row: Option<Box<[CellWord]>> = branch
            .row
            .iter()
            .map(|cell| match cell {
                FlatTerm::Text(number) => Some(text_word(*number)),
                FlatTerm::Number(number) => Some(CellWord(number.0.cast_unsigned())),
                FlatTerm::Variable(_) | FlatTerm::File => None,
            })
            .collect();
        let row = row?;
        if !rows.contains(&row) {
            rows.push(row);
        }
    }
    (!branches.is_empty()).then_some(ConstantRows(rows))
}

/// The variables a pattern's bound columns give a branch.
pub(crate) fn given(branch: &FlatBranch, bound: &ColumnMask) -> HashSet<FlatVariable> {
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

/// SPIKE (the walk): whether a table's pipeline asked the pattern.
fn draft_fill_asked(drafts: &Drafts, waiting: WaitingPattern) -> bool {
    draft_at(drafts, waiting.definition)
        .and_then(|draft| draft.patterns.get(waiting.pattern.0))
        .is_some_and(|pattern| pattern.fill_asked)
}

/// The plan of each branch of a pattern's definition with the pattern's bound columns given: a pattern has a program when
/// every branch has one, every access keyed. None when a branch has none, or the pattern is not one of the drafts'. It
/// reads the drafts and writes nothing: the plans of patterns that wait may be made in any order, or at once.
#[must_use]
pub(crate) fn pattern_plans(
    drafts: &Drafts,
    waiting: WaitingPattern,
) -> Option<Vec<PlannedBranch>> {
    let draft = draft_at(drafts, waiting.definition)?;
    let bound = &draft.patterns.get(waiting.pattern.0)?.asked.bound;
    let plans = draft
        .branches
        .iter()
        .map(|branch| planned(drafts, branch, &given(branch, bound), UnitRole::Pattern));
    plans.collect()
}

/// Places one pattern asked of a definition and writes it in its form: its program, or its definition's table. `plans` are
/// the pattern's own (`pattern_plans`).
pub(crate) fn pattern_placed(
    drafts: &mut Drafts,
    sample: LoadSample<'_>,
    waiting: WaitingPattern,
    plans: Option<Vec<PlannedBranch>>,
) -> Result<(), RefusedPlace> {
    let missing = RefusedPlace {
        rule: AcceptedPlace(0),
        refusal: unlowerable(StepId(0)),
    };
    let Some(draft) = draft_at(drafts, waiting.definition) else {
        return Err(missing);
    };
    let Some(pattern) = draft.patterns.get(waiting.pattern.0) else {
        return Err(missing);
    };
    let (bound, collected) = (pattern.asked.bound.clone(), pattern.asked.collected.clone());
    let (home, node, formals) = (draft.home, draft.node, draft.formals.clone());
    let branches: Vec<FlatBranch> = draft.branches.clone();
    let calls = draft.patterns.iter();
    let call_patterns = Count(
        calls
            .filter(|pattern| matches!(pattern.form, Some(PatternForm::Call(_))))
            .count(),
    );
    let table = match (own_walks(&branches), draft.keyless.is_empty()) {
        (OwnWalks::Walked, _) => TableForm::Calls,
        (OwnWalks::Stageable, true) => TableForm::Keyed,
        (OwnWalks::Stageable, false) => TableForm::Product,
    };
    let facts = PatternFacts {
        binds: Count(bound.0.len()),
        call: match &plans {
            Some(_) => PatternProgram::Keyed,
            None => PatternProgram::Scans,
        },
        table,
        call_patterns,
        measured: measured(
            sample.statistics,
            &UnitKey::Pattern(node, bound.clone(), collected.clone()),
        ),
    };
    let mut placement = placed(facts, sample.stage);
    // SPIKE (the walk): `RYO_WALK=fill`: a pattern a table's pipeline asks is made whole: the pipeline enumerates its
    // keys from a whole input anyway, so its asks scale with the input, and a probe of the definition's table is
    // cheaper than a navigation program run for each.
    // A definition whose own pipelines use its call form (a nearest walk, which no stage runs) keeps it.
    if crate::domain::walk::on("fill")
        && facts.table != TableForm::Calls
        && draft_fill_asked(drafts, waiting)
    {
        placement.form = FormChoice::Table;
    }
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
            let listed: Vec<ReadColumn> = bound
                .0
                .iter()
                .map(|column| ReadColumn::Made(*column))
                .collect();
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
        .and_then(|draft| draft.patterns.get_mut(waiting.pattern.0));
    if let Some(slot) = slot {
        slot.form = Some(form);
        slot.shape = placement.shape;
    }
    Ok(())
}
