//! Walk-source admission for a live kernel permit.
use super::super::KernelPermit;
use crate::application::file_dag::program::admission::AdmittedWalkSource;
use crate::application::file_dag::program::admission::FileGeneration;
use crate::application::file_dag::program::admission::Partition;
use ryo_execution_domain::domain::executable_program::types::InputId;
use ryo_execution_domain::domain::operating::errors::OperatingError;

impl KernelPermit<'_> {
    pub(crate) fn require_file_walk(
        &self,
        expected: &FileGeneration,
    ) -> Result<(), OperatingError> {
        self.validate_live()?;
        match self.partition {
            Partition::File(actual)
                if actual.matches(expected, self.generation)
                    == crate::application::file_dag::program::admission::Agreement::Same =>
            {
                Ok(())
            }
            _ => Err(OperatingError::NotRead),
        }
    }
    pub(crate) fn require_project_walk(
        &self,
    ) -> Result<(), OperatingError> {
        self.validate_live()?;
        match self.partition {
            Partition::Project(_) => Ok(()),
            _ => Err(OperatingError::NotRead),
        }
    }
    pub(crate) fn admit_walk_source<'call>(
        &'call self,
        relation: ryo_relations::domain::relations::types::Relation,
        rows: &'call ryo_execution_domain::domain::table_model::types::Rows<'call>,
    ) -> Result<Option<AdmittedWalkSource<'call>>, OperatingError> {
        self.validate_live()?;
        let source = self.bound_input(InputId(0))?;
        use ryo_values::domain::{
            plan_parts::types::{Input, RepeatedRows},
            plans::types::Operation};
        let authored = match &self.descriptor().operation() {
            Operation::Select {
                from: Input::Relation(authored),
                ..
            }
            | Operation::Project {
                from: Input::Relation(authored),
                distinct: RepeatedRows::Kept,
                ..
            } => authored,
            _ => return Err(OperatingError::WrongOperation),
        };
        if *authored != relation {
            return Err(OperatingError::WrongOperation);
        }
        source.walk_source(relation, rows, self.generation)
    }

    pub(crate) fn validate_walk_source(
        &self,
        source: &AdmittedWalkSource<'_>,
    ) -> Result<(), OperatingError> {
        self.validate_live()?;
        source.require_input(self.bound_input(InputId(0))?)
    }
}
