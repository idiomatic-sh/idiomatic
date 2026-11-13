//! The support of the `project_phase` cases.
use super::super::super::fixtures::submission::Root;
use super::super::super::fixtures::{Entry, corpus, covered, prepare_for, program};
use crate::application::file_dag::frontiers::errors::FrontiersError;
use crate::application::file_dag::program::project_phase::interface::ProgramProjectPhasePort as _;
use crate::application::file_dag::program::project_phase::types::ProjectPhase;
use crate::application::file_dag::program::request_execution::interface::ProgramRequestExecutionPort as _;
use crate::application::file_dag::program::request_execution::service::begin;
use ryo_execution_domain::domain::coverage::types::{InapplicableFiles, RefusedFiles, RuleIndex};
use ryo_execution_domain::domain::operating::errors::OperatingError;

pub(super) const fn root(rule: usize) -> Root {
    Root {
        declared: rule,
        named: rule,
    }
}

/// How a phase prepared a rule's frontiers: ready, refused by its metadata, or refused otherwise.
#[derive(Debug, PartialEq)]
pub(super) enum Prepared {
    Ready,
    Metadata(OperatingError),
    Other,
}

/// The refusals of a scope for declared rule 2 and for undeclared rule 3, and in a phase the
/// preparations of declared rule 0's and undeclared rule 3's frontiers.
#[derive(Debug, PartialEq)]
pub(super) struct Scopes {
    pub(super) declared: Option<OperatingError>,
    pub(super) undeclared: Option<OperatingError>,
    pub(super) frontiers: Option<(Prepared, Prepared)>,
}

/// Opens a phase or stage of a request over three rules and the row's scopes, and prepares the
/// declared and undeclared rules' frontiers.
pub(super) fn scopes(entry: Entry) -> Result<Scopes, OperatingError> {
    let prepared = program(entry, 3);
    let corpus = corpus();
    let request = begin(&prepared, &corpus, None);
    let mut arenas =
        crate::application::file_dag::program::request_execution::types::ObservationArenas::default(
        );
    let check = |phase: &ProjectPhase<'_>| -> Result<Scopes, OperatingError> {
        let scope = |rule| {
            phase.project_scope(
                rule,
                &corpus,
                &covered(3),
                &[],
                &InapplicableFiles::default(),
                &RefusedFiles::default(),
            )
        };
        let declared = scope(RuleIndex(2)).err();
        let undeclared = scope(RuleIndex(3)).err();
        let frontiers = if matches!(entry, Entry::Stage) {
            None
        } else {
            let prepare = |rule| {
                prepare_for(&request, &scope(RuleIndex(0))?, rule).map_or_else(
                    |error| match error {
                        FrontiersError::Metadata(error) => Ok(Prepared::Metadata(error)),
                        _ => Ok(Prepared::Other),
                    },
                    |_| Ok::<_, OperatingError>(Prepared::Ready),
                )
            };
            Some((prepare(RuleIndex(0))?, prepare(RuleIndex(3))?))
        };
        Ok(Scopes {
            declared,
            undeclared,
            frontiers,
        })
    };
    let answered = match entry {
        Entry::Phase => {
            let phase = request.project_phase()?;
            let answered = check(&phase)?;
            phase.finish()?;
            answered
        }
        Entry::Stage => {
            let stage = request.observations(&mut arenas)?;
            let answered = check(&stage.phase)?;
            stage.finish_stage()?;
            answered
        }
    };
    request.finish();
    Ok(answered)
}
