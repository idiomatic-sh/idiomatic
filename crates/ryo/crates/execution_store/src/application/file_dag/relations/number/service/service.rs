//! The `service` application operation.

use crate::application::file_dag::relations::handle::service::PublishedInput;

/// The borrowed publication pins its exact population/generation for the invocation.
/// Construction is private: a schema or a caller assertion cannot mint this witness.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::relations::number::service::service::AdmittedNumber;
/// fn named(_: Option<&AdmittedNumber<'_>>) {}
/// ```
#[derive(Debug)]
pub struct AdmittedNumber<'a> {
    pub(crate) publication: &'a PublishedInput,
    pub(crate) column: ryo_values::domain::plan_parts::types::ColumnIndex,
}

impl AdmittedNumber<'_> {
    /// The publication the numeric admission pinned.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::relations::number::service::service::AdmittedNumber;
    /// let _ = AdmittedNumber::publication;
    /// ```
    pub fn publication(&self) -> &PublishedInput {
        self.publication
    }
    /// The numeric column the population owner bound.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::relations::number::service::service::AdmittedNumber;
    /// let _ = AdmittedNumber::column;
    /// ```
    pub fn column(&self) -> ryo_values::domain::plan_parts::types::ColumnIndex {
        self.column
    }
}
