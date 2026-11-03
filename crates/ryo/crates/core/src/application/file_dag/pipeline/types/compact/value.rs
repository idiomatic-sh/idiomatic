//! Borrowed access to compact stored inputs, with no provider ownership escaping the generation.

use crate::application::file_dag::pipeline::types::Stepper;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::types::StepPlace;
use crate::domain::table_model::types::Rows;
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

    /// A selected result must be read through its owner publication before this
    /// fallback; only ordinary materialized cuts borrow their table here.
    pub(crate) fn kept(&self, index: usize, table: &'p Table) -> Result<Rows<'p>, OperatingError> {
        if self
            .stepper
            .selected
            .get(index)
            .is_some_and(Option::is_some)
        {
            return Err(OperatingError::NotRun);
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
        {
            return Ok(None);
        }
        let owner = &self.stepper.relation_owner;
        let Some(produced) = owner.produce_selection(
            program,
            &|relation| self.stepper.read.rows(relation),
            context,
        )?
        else {
            return Ok(None);
        };
        let (publication, rows) = owner.deliver_selection(produced)?;
        let slot = self
            .stepper
            .dag_shared
            .get(id.0)
            .ok_or(OperatingError::NotRun)?;
        if slot.get().is_none() {
            slot.set(publication).map_err(|_| OperatingError::NotRun)?;
        }
        Ok(Some(rows))
    }
}

use ryo_values::domain::rows::types::Table;
