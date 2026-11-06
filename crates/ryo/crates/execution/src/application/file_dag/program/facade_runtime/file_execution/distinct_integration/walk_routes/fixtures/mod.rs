//! Immutable real request inputs and the production file-owner composition.
use self::plans::Authored;
use self::semantics::{Calls, Derived, Meanings};
use super::super::*;
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::domain::positions::types::ClassPlace;
use ryo_relations::domain::relations::{
    service::to_bits,
    types::{ColumnsView, FactColumn},
};
use std::sync::atomic::{AtomicUsize, Ordering};
pub(super) mod plans;
pub(super) mod semantics;

#[derive(Clone, Copy, Debug)]
pub(super) enum Residency {
    Stable,
    Unstable,
}
#[derive(Clone, Copy)]
pub(super) enum Population {
    Three,
    Duplicate,
    Empty,
}

pub(super) struct Program {
    pub(super) prepared: Arc<PreparedProgram>,
    services: Arc<ProgramServices>,
    intents: Vec<FileRuleIntent>,
    residency: Residency,
    columns: Vec<FactColumn>,
    reads: AtomicUsize,
}
impl Program {
    pub(super) fn new(
        authored: Authored,
        population: Population,
        residency: Residency,
        meanings: Arc<Meanings>,
        derived: Arc<Derived>,
    ) -> Result<Self, OperatingError> {
        let compiled = crate::domain::prepared_request::service::assemble(
            authored.plans,
            vec![vec![]; authored.rules.len()],
            &authored.rules,
        )?;
        let prepared = crate::application::file_dag::program::preparation::service::prepare(
            compiled,
            &authored.rules,
            &vec![Digest([0; 16]); authored.rules.len()],
            &[],
            derived.clone(),
            authored.observations,
        )?;
        let services = Arc::new(ProgramServices::new(
            Arc::new(CommonOperatorEngine::new(meanings)),
            derived,
            &[CorpusInput::Available(CorpusEntry {
                executable: ExecutablePermission::Unknown,
                path: FilePath("walk-routes.input".into()),
                size: FileSize(0),
                modified: Nanoseconds(0),
            })],
        ));
        let rows: Vec<[i64; 2]> = match population {
            Population::Three => vec![[1, 5], [2, 9], [3, 7]],
            Population::Duplicate => vec![[2, 9], [2, 7], [3, 5]],
            Population::Empty => vec![],
        };
        let columns = vec![
            FactColumn(vec![to_bits(Cell::File(FileId(0))).0; rows.len()]),
            FactColumn(
                rows.iter()
                    .map(|row| to_bits(Cell::Number(Integer(row[0]))).0)
                    .collect(),
            ),
            FactColumn(
                rows.iter()
                    .map(|row| to_bits(Cell::Number(Integer(row[1]))).0)
                    .collect(),
            ),
        ];
        Ok(Self {
            prepared,
            services,
            intents: authored.intents,
            residency,
            columns,
            reads: AtomicUsize::new(0),
        })
    }
    pub(super) fn open<'a>(&'a self, symbols: &mut Symbols) -> Result<FileRun<'a>, OperatingError> {
        open_file(
            &self.prepared,
            FileId(0),
            ScopePartials::File {
                evaluation: &[],
                publication: &[],
            },
            &self.intents,
            self.services.clone(),
            b"",
            matches!(self.residency, Residency::Stable),
            symbols,
            None,
            None,
        )
    }
    pub(super) fn read(&self, relation: Relation) -> Result<Rows<'_>, OperatingError> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        if relation != Relation::LanguageTarget {
            return Err(OperatingError::NotRead);
        }
        Ok(Rows::Stored(ColumnsView {
            kinds: kinds(relation),
            columns: &self.columns,
        }))
    }
    /// The source class of the rule's authored head, resolved from the prepared walk.
    pub(super) fn head_class(&self, rule: RuleIndex) -> Result<ClassPlace, OperatingError> {
        let program = self.prepared.program();
        let head = program.rules()[rule.0].original_steps[0]
            .as_ref()
            .map_err(Clone::clone)?
            .logical;
        let region = program.logical_regions()[head.0];
        let instruction = program.regions()[region.0]
            .instructions
            .first()
            .ok_or(OperatingError::NotRun)?;
        self.prepared
            .walk_class(instruction)
            .ok_or(OperatingError::NotRun)
    }
    pub(super) fn reads(&self) -> Calls {
        Calls(self.reads.load(Ordering::Relaxed))
    }
}
pub(super) fn demand() -> RuleDemand {
    RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::NotRequired,
    }
}
pub(super) fn cells(values: &[i64]) -> Vec<Cell> {
    values
        .iter()
        .map(|number| Cell::Number(Integer(*number)))
        .collect()
}
