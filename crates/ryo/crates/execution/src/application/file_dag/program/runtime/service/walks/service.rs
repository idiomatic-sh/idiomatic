//! Walk-source admission and native join eligibility for a live kernel permit.
use super::super::KernelPermit;
use crate::application::file_dag::program::admission::AdmittedInput;
use crate::application::file_dag::program::admission::AdmittedWalkSource;
use crate::application::file_dag::program::admission::FileGeneration;
use crate::application::file_dag::program::admission::Partition;
use crate::application::file_dag::relations::RelationOwner;
use crate::domain::executable_program::types::InputId;
use crate::domain::operating::errors::OperatingError;
use std::rc::Rc;
use std::sync::Arc;

impl KernelPermit<'_> {
    pub(in crate::application::file_dag::program) fn require_file_walk(
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
    pub(in crate::application::file_dag::program) fn require_project_walk(
        &self,
    ) -> Result<(), OperatingError> {
        self.validate_live()?;
        match self.partition {
            Partition::Project(_) => Ok(()),
            _ => Err(OperatingError::NotRead),
        }
    }
    pub(in crate::application::file_dag::program) fn admit_walk_source<'call>(
        &'call self,
        relation: ryo_relations::domain::relations::types::Relation,
        rows: &'call crate::domain::table_model::types::Rows<'call>,
    ) -> Result<Option<AdmittedWalkSource<'call>>, OperatingError> {
        self.validate_live()?;
        let source = self.bound_input(InputId(0))?;
        use ryo_values::domain::{
            plan_parts::types::{Input, RepeatedRows},
            plans::types::Operation,
        };
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

    pub(in crate::application::file_dag::program) fn validate_walk_source(
        &self,
        source: &AdmittedWalkSource<'_>,
    ) -> Result<(), OperatingError> {
        self.validate_live()?;
        source.require_input(self.bound_input(InputId(0))?)
    }
    pub(in crate::application::file_dag::program) fn selection_input<'a>(
        &'a self,
        shape: &crate::domain::executable_program::types::SelectionAlternative,
    ) -> Result<&'a AdmittedInput, OperatingError> {
        self.validate_live()?;
        let region = &self.program.regions()[self.region.0];
        let declared = crate::domain::executable_program::service::selection(&region.route())
            .ok_or(OperatingError::WrongOperation)?;
        if shape.target() != self.region
            || shape.logical() != region.instructions()[self.instruction.0].logical()
            || !Arc::ptr_eq(shape.selector(), declared.selector())
        {
            return Err(OperatingError::NotRead);
        }
        let source = self.bound_input(InputId(0))?;
        source.validate_publication()?;
        Ok(source)
    }
    pub(in crate::application::file_dag::program) fn validate_native_join(
        &self,
        native: &crate::application::file_dag::invocation::native::service::NativeJoinLease<'_>,
        owner: &Rc<RelationOwner>,
        left: &crate::domain::table_model::types::Rows<'_>,
    ) -> Result<(), OperatingError> {
        self.validate_registered(self.operation())?;
        if !matches!(
            self.native_join_eligibility(),
            crate::application::file_dag::program::runtime::types::NativeJoinEligibility::Native
        ) {
            return Err(OperatingError::WrongOperation);
        }
        let region = &self.program.regions()[self.region.0];
        if region.instructions().len() != 1 || region.inputs().len() != 2 {
            return Err(OperatingError::WrongOperation);
        }
        let left_input = self.bound_input(InputId(0))?;
        let right_input = self.bound_input(InputId(1))?;
        let expected = region.instructions()[self.instruction.0]
            .contract()
            .derived
            .slots[ryo_derived::domain::contract::types::DerivedKind::NativeLookup.ordinal()]
        .as_ref()
        .ok_or(OperatingError::WrongOperation)?;
        if !std::ptr::eq(expected.primary.plan.as_ref(), native.contract()) {
            return Err(OperatingError::NotRead);
        }
        owner.admit(
            left_input.publication(owner)?.as_ref(),
            crate::domain::operator::contract::views::BorrowedRows::from(left),
        )?;
        owner.admit(
            right_input.publication(owner)?.as_ref(),
            crate::domain::operator::contract::views::BorrowedRows::Tree(native.tree()),
        )?;
        native.finding()?;
        Ok(())
    }
    pub(in crate::application::file_dag::program) fn native_join_eligibility(
        &self,
    ) -> crate::application::file_dag::program::runtime::types::NativeJoinEligibility {
        let region = &self.program.regions()[self.region.0];
        if self.program.routes().tree_join == crate::domain::executable_program::types::RouteWiring::Wired
            && region.instructions().len() == 1
            && matches!(
                self.operation(),
                ryo_values::domain::plans::types::Operation::Join { reach: None, filter, .. } if filter.is_empty()
            )
            && region.instructions()[self.instruction.0]
                .contract()
                .schema
                .is_ok()
            && region.instructions()[self.instruction.0]
                .contract()
                .row_error
                .is_none()
        {
            crate::application::file_dag::program::runtime::types::NativeJoinEligibility::Native
        } else {
            crate::application::file_dag::program::runtime::types::NativeJoinEligibility::Ordinary
        }
    }
}
