//! The test support of the `project` use case: a project with no relation, the frontier rows of one file and a counter of the rows
//! the pass hands over.

use super::errors::ProjectPassError;
use super::interface::ProjectUseCase as _;
use super::use_case::ProjectUseCaseImpl;
use crate::domain::findings::types::{
    FoundCells, FoundRows, FoundTarget, FrontierCells, FrontierSource, ScanCounters,
};
use crate::domain::programs::service::scratch_for;
use crate::domain::programs::types::Programs;
use ryo_language::domain::facts::types::{
    FactAnswer, FactSource, FileWords, FilledColumns, ProjectFacts, Wanted,
};
use ryo_language::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::types::{Count, Relation};
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// A project with no relation: every fill refused.
#[derive(Clone, Copy, Debug)]
pub struct NoProject;

impl FactSource for NoProject {
    fn fill(
        &mut self,
        _relation: Relation,
        _wanted: Wanted,
        _words: &mut FileWords,
        _out: &mut FilledColumns,
    ) -> FactAnswer {
        FactAnswer::Refused(AbsenceCause::NoManifests)
    }

    fn render(&mut self, _request: RenderRequest) -> RenderOutcome {
        RenderOutcome::NotApplicable
    }
}

/// The frontier rows of one file: one row of rule 0's first frontier entry; none when `empty`.
#[derive(Clone, Copy, Debug)]
pub struct OneFile {
    /// Whether the file handed over nothing.
    pub empty: bool,
}

impl FrontierSource for OneFile {
    fn rows(
        &self,
        rule: SelectedRuleOrdinal,
        input: InputNumber,
        each: &mut dyn FnMut(FrontierCells<'_>),
    ) {
        if !self.empty && rule == SelectedRuleOrdinal(0) && input == InputNumber(0) {
            each(FrontierCells(&[]));
        }
    }
}

/// Counts the rows the pass hands over.
#[derive(Clone, Copy, Debug, Default)]
pub struct Found(pub Count);

impl FoundRows for Found {
    fn found(&mut self, _target: FoundTarget, _row: FoundCells<'_>, _words: &FileWords) {
        self.0 = Count(self.0.0.saturating_add(1));
    }
}

/// Runs the project pass of `programs` over one file's frontier rows (none when `empty`) and a project with no relation.
///
/// # Errors
///
/// The pass's refusal.
pub fn projected(
    programs: &Programs,
    empty: bool,
) -> Result<(ScanCounters, Count), ProjectPassError> {
    let mut found = Found::default();
    let facts = ProjectFacts {
        facts: &mut NoProject,
    };
    let scratch = &mut scratch_for(programs);
    let counters =
        ProjectUseCaseImpl.project(programs, &OneFile { empty }, facts, scratch, &mut found)?;
    Ok((counters, found.0))
}
