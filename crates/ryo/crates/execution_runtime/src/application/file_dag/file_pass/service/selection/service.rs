//! A proof over the open file's complete, ordered provider columns.

use crate::application::file_dag::fact_lookup::service::FactLookup;
use crate::application::file_dag::file_pass::types::SourceCondition;
use ryo_execution_domain::domain::executable_program::types::SourceAbsence;
use ryo_language::domain::symbols::service::text_of;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::files::types::SourceView;
use std::collections::HashSet;

/// Determine the eligible program through the file's already-committed columns and symbols.
pub(in crate::application::file_dag::file_pass) fn supplied(
    absence: &SourceAbsence,
    source: SourceView<'_>,
    names: &Symbols,
    lookup: &FactLookup<'_, '_>,
) -> SourceCondition {
    if lookup
        .partials()
        .iter()
        .any(|p| p.relation == absence.relation)
    {
        return SourceCondition::Unproven;
    }
    let Ok(view) = lookup.columns(absence.relation) else {
        return SourceCondition::Unproven;
    };
    let at = usize::from(absence.column);
    if view.kinds.get(at) != Some(&CellKind::Symbol) {
        return SourceCondition::Unproven;
    }
    let Some(column) = view.columns.get(at) else {
        return SourceCondition::Unproven;
    };
    let mut visited = HashSet::new();
    for &word in &column.0 {
        if !visited.insert(word) {
            continue;
        }
        let Ok(id) = u32::try_from(word) else {
            return SourceCondition::Unproven;
        };
        let Some(value) = text_of(names, ByteSlice(source.0), Symbol(id)) else {
            return SourceCondition::Unproven;
        };
        if value.0 == absence.text.0.as_ref() {
            return SourceCondition::Unproven;
        }
    }
    SourceCondition::Proven
}

use crate::application::file_dag::file_pass::types::{PassFile, PassProgram};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::routes::types::RuleRoute;
use ryo_execution_domain::domain::executable_program::service::{engine_uses, physical, rule};
use ryo_execution_domain::domain::executable_program::types::RuleNodes;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_operators::domain::physical_plan::types::PlanNodeId;
use ryo_values::domain::plan_parts::types::StepId;

/// The route whose roots this file selected for the rule.
fn route(pass: &PassProgram, condition: SourceCondition, index: RuleIndex) -> Option<&RuleRoute> {
    if condition != SourceCondition::Proven {
        return None;
    }
    pass.conditional
        .as_ref()?
        .routes
        .rules
        .get(index.0)?
        .as_ref()
}

/// The authored step identities on this file's selected route.
pub(in crate::application::file_dag::file_pass) fn file_rule(
    pass: &PassProgram,
    condition: SourceCondition,
    index: RuleIndex,
) -> Result<&RuleNodes, OperatingError> {
    match route(pass, condition, index) {
        Some(route) => Ok(&route.nodes),
        None => rule(&pass.program, index),
    }
}

/// The engine consumers released by this rule's selected public ports.
pub(in crate::application::file_dag::file_pass) fn file_uses(
    pass: &PassProgram,
    condition: SourceCondition,
    index: RuleIndex,
) -> Result<&[PlanNodeId], OperatingError> {
    match route(pass, condition, index) {
        Some(route) => Ok(&route.uses),
        None => engine_uses(&pass.program, index),
    }
}

/// Rebind a physical request made from an earlier-prepared authored schedule.
pub(in crate::application::file_dag::file_pass) fn selected_step(
    pass: &PassProgram,
    condition: SourceCondition,
    index: RuleIndex,
    step: StepId,
    ordinary: PlanNodeId,
) -> Result<PlanNodeId, OperatingError> {
    match route(pass, condition, index) {
        Some(selected) => selected
            .nodes
            .steps
            .get(step.0 as usize)
            .cloned()
            .ok_or(OperatingError::NotRun)?,
        None => Ok(ordinary),
    }
}

/// Release exactly the unused public holds, before any file reader acquires its rows.
pub(in crate::application::file_dag::file_pass) fn retire_unchosen(
    at: &mut PassFile<'_, '_, '_, '_>,
) {
    let Some(selection) = &at.pass.conditional else {
        return;
    };
    let unused = match at.condition {
        SourceCondition::Proven => &selection.routes.unused_when_proven,
        SourceCondition::Unproven => &selection.routes.unused_when_ordinary,
    };
    let plan = physical(&at.pass.program);
    for node in unused {
        at.buffers.used(plan, *node);
    }
}

/// Release the public ports selected for this rule.
pub(in crate::application::file_dag::file_pass) fn finish_rule(
    at: &mut PassFile<'_, '_, '_, '_>,
    index: RuleIndex,
) {
    let pass = at.pass;
    let Ok(uses) = file_uses(pass, at.condition, index) else {
        return;
    };
    for &node in uses {
        at.buffers.used(physical(&pass.program), node);
    }
}
