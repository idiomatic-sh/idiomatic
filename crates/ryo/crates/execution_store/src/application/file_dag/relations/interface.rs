//! What the runtime needs from the store to hold, read and free published relations: the one pub route to them.
use super::admission::service::BorrowedInput;
use super::handle::service::Publication;
use super::number::service::AdmittedNumber;
use super::publication::types::{Body, ProducerScope};
use super::types::{Generation, ProducerId};
use crate::application::file_dag::derived::population::types::PopulationId;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::self_join_recogniser::types::NumberRequirement;
use ryo_execution_domain::domain::table_model::types::{RepresentationInput, Rows};
use ryo_relations::domain::relations::types::{CellKind, Relation};

/// What the runtime needs from the store to hold, read and free published relations: the one pub route to them.
pub trait RelationStore {
    /// The resident population's generation.
    fn generation(&self) -> Generation;
    /// Publishes `body`, its backing accounted to the ledger.
    ///
    /// # Errors
    ///
    /// A table whose rows do not fit its contract, a ledger that refuses the lease, or a population count that overflows.
    fn publish(&self, body: Body<'_>) -> Result<Publication, OperatingError>;
    /// A population identity for the project scope.
    ///
    /// # Errors
    ///
    /// A population count that overflows.
    fn population(&self) -> Result<PopulationId, OperatingError>;
    /// Admits the rows of a publication as a borrowed input of the current generation: the outer refusal is an identity one,
    /// the inner one the rows' availability.
    ///
    /// # Errors
    ///
    /// `NotRead` for a publication of another generation or other rows.
    fn admit<'a>(
        &self,
        publication: &Publication,
        rows: RepresentationInput<'a>,
    ) -> Result<Result<BorrowedInput<'a>, OperatingError>, OperatingError>;
    /// Admits a numeric source for the producer that requires it, the one way to mint the witness; `None` when the producer or
    /// its sources do not qualify.
    ///
    /// # Errors
    ///
    /// The physical inspection's refusal of the rows.
    fn admit_number<'a>(
        &self,
        publication: &'a Publication,
        producer: ProducerId,
        required: &NumberRequirement,
        rows: RepresentationInput<'_>,
    ) -> Result<Option<AdmittedNumber<'a>>, OperatingError>;
    /// The rows of a publication, read through `read` when they are a source's.
    ///
    /// # Errors
    ///
    /// `NotRead` for a publication of another generation or a source whose rows changed; `read`'s refusal.
    fn rows<'r, 'f: 'r>(
        &self,
        publication: &'r Publication,
        read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
    ) -> Result<Rows<'r>, OperatingError>;
    /// The rows of a publication in their normalized form.
    ///
    /// # Errors
    ///
    /// `NotRead` for a publication of another generation or other rows; the inspection's or the ledger's refusal.
    fn normalize<'r, 'f: 'r>(
        &self,
        publication: &'r Publication,
        rows: Rows<'f>,
    ) -> Result<Rows<'r>, OperatingError>;
    /// The cell kinds of a publication.
    ///
    /// # Errors
    ///
    /// `NotRead` for a publication of another generation.
    fn kinds<'a>(&self, publication: &'a Publication) -> Result<&'a [CellKind], OperatingError>;
    /// Binds one producer to its publication: one population identity across rules and safe recomputation.
    ///
    /// # Errors
    ///
    /// `NotRead` for a publication of another generation; `WrongBinding` for a publication bound in another scope.
    fn bind_producer(
        &self,
        scope: ProducerScope,
        producer: ProducerId,
        publication: &Publication,
    ) -> Result<(), OperatingError>;
    /// Pins the derived views of a publication to it.
    fn pin(&self, publication: &Publication);
    /// Releases the derived views of one producer; a producer never bound releases nothing.
    fn retire_producer(&self, scope: ProducerScope, producer: ProducerId);
    /// Ends the resident population: every publication of its generation is refused from now on.
    fn release(&self);
}
