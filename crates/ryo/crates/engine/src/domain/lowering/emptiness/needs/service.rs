//! The services of the `needs` part: the tables a rule's branch cannot give a row without, added to its formula in an order fixed at load.

use crate::domain::lowered::types::{Emptiness, LoweredPack, PatternForm};
use crate::domain::lowering::definitions::types::RowEstimate;
use crate::domain::steps::types::{
    DefinitionId, MadeTableId, Polarity, ProgramStep, StepBody, StepProgram, StepRows,
};

/// The tables a rule's branch cannot give a row without: each table it reads rows of, by a made table's rows, by a
/// definition's rows whole, or by a use in table form that asks for a row.
pub(crate) fn tables_needed(pack: &LoweredPack, program: &StepProgram) -> Vec<MadeTableId> {
    let of_definition = |definition: DefinitionId| {
        let place = usize::try_from(definition.0).unwrap_or(usize::MAX);
        let patterns = pack
            .definitions
            .get(place)
            .map(|definition| &definition.patterns);
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
pub(crate) fn needs_added(pack: &mut LoweredPack, rows: &[RowEstimate]) {
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
