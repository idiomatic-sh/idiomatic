//! Borrowed access to compact stored inputs, with no provider ownership escaping the generation.

use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::types::StepPlace;
use crate::domain::pipeline::types::Stepper;
use crate::domain::table_model::types::{Rows, SelectedRows, Table};
use crate::domain::trust::service::blocked;
use crate::domain::trust::types::Blocking;

/// One read-only view of a stepper's compact metadata; no mutable cache borrow is retained.
#[derive(Clone, Copy)]
pub(crate) struct CompactInput<'a, 'p, 'f> {
    /// Current plan inputs and their generation-bound provider reader.
    stepper: &'a Stepper<'p, 'f>,
}

impl<'a, 'p, 'f: 'p> CompactInput<'a, 'p, 'f> {
    /// Borrow only the duration of a hold/stream acquisition.
    pub(crate) fn new(stepper: &'a Stepper<'p, 'f>) -> Self {
        Self { stepper }
    }

    /// Reacquire a kept selection's provider before replay; otherwise retain the computed table.
    pub(crate) fn kept(&self, index: usize, table: &'p Table) -> Result<Rows<'p>, OperatingError> {
        if let Some(selection) = self.stepper.selected.get(index).and_then(Option::as_ref) {
            let Rows::Stored(source) = self.stepper.read.rows(selection.relation)? else {
                return Err(OperatingError::WrongOperation);
            };
            return Ok(Rows::Selected(SelectedRows {
                source,
                selection: std::sync::Arc::clone(selection),
            }));
        }
        Ok(Rows::Computed(table))
    }

    /// Only admitted, complete stable populations may use demand-driven compact execution.
    pub(crate) fn selected(
        &self,
        id: StepPlace,
        context: &mut OperatorContext<'_>,
    ) -> Result<Option<Rows<'f>>, OperatingError> {
        let Some(program) = self.stepper.selectors.get(id.0).and_then(Option::as_ref) else {
            return Ok(None);
        };
        if self
            .stepper
            .layout
            .needs
            .get(id.0)
            .is_some_and(|needs| blocked(needs, self.stepper.partial) == Blocking::Blocked)
            || context.stored_indexes.as_ref().is_none_or(|cache| {
                !cache
                    .selectors
                    .iter()
                    .any(|candidate| candidate.relation == program.relation)
            })
        {
            return Ok(None);
        }
        let Rows::Stored(source) = self.stepper.read.rows(program.relation)? else {
            return Ok(None);
        };
        if source.kinds != ryo_language::domain::relations::service::kinds(program.relation) {
            return Ok(None);
        }
        crate::domain::stored_selection::service::select(program, source, context)
            .map(|selected| selected.map(Rows::Selected))
    }
}
