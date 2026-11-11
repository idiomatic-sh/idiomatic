//! The `external` application operation.

use super::super::service::*;
#[cfg(test)]
use crate::application::file_dag::relations::admission::service::BorrowedInput;
#[cfg(test)]
use ryo_execution_domain::domain::operating::errors::OperatingError;
#[cfg(test)]
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
#[cfg(test)]
use ryo_relations::domain::relations::types::Relation;

impl RelationOwner {
    #[cfg(test)]
    pub(crate) fn admit_relation<'a>(
        &self,
        relation: Relation,
        rows: RepresentationInput<'a>,
    ) -> Result<BorrowedInput<'a>, OperatingError> {
        let record = self.source_borrowed(relation, rows)?;
        self.admit(record.as_ref(), rows)
    }
}
