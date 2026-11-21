//! The services of the `patterns` part: one pattern asked of a definition placed and written in its form: its program, or its definition's table.

use crate::domain::code::types::UnitKey;
use crate::domain::lowered::types::{Emptiness, MadeFill, MadeTable, PatternForm};
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
use crate::domain::lowering::planning::types::PatternColumns;
use crate::domain::lowering::planning::types::UnitRole;
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

/// Places one pattern asked of a definition and writes it in its form: its program, or its definition's table.
pub(crate) fn pattern_placed(
    drafts: &mut Drafts,
    sample: LoadSample<'_>,
    waiting: WaitingPattern,
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
    // A pattern has a program when every branch has one with the bound columns given: every access keyed.
    let plans: Option<Vec<_>> = branches
        .iter()
        .map(|branch| planned(drafts, branch, &given(branch, &bound), UnitRole::Pattern))
        .collect();
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
