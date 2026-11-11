//! Worker-local lease capabilities wrap thread-safe immutable publication records.
use crate::application::file_dag::derived::ledger::service::ResourceLease;
use crate::application::file_dag::relations::publication::types::RelationResult;
use crate::application::file_dag::relations::types::NormalizationCharge;
use std::{cell::RefCell, sync::Arc};
#[derive(Debug)]
struct RequiredLeases {
    _initial: Arc<ResourceLease>,
    normalized: RefCell<Option<Arc<ResourceLease>>>,
    normalized_in_initial: NormalizationCharge,
}
/// A publication of a relation result the owner leased: its record, pinned for its holder.
#[derive(Clone, Debug)]
pub struct PublishedInput {
    record: Arc<RelationResult>,
    leases: Arc<RequiredLeases>,
}
impl PublishedInput {
    pub(crate) fn new(record: Arc<RelationResult>, initial: Arc<ResourceLease>) -> Self {
        let normalized_in_initial = if record.normalized.get().is_some() {
            NormalizationCharge::Charged
        } else {
            NormalizationCharge::Uncharged
        };
        Self {
            record,
            leases: Arc::new(RequiredLeases {
                _initial: initial,
                normalized: RefCell::new(None),
                normalized_in_initial,
            }),
        }
    }
    /// The published record this input leases, borrowed for reading.
    pub(crate) fn as_ref(&self) -> &RelationResult {
        &self.record
    }
    /// The published record.
    #[cfg(test)]
    pub(crate) fn record(&self) -> &Arc<RelationResult> {
        &self.record
    }
    pub(crate) fn normalization_charged(&self) -> NormalizationCharge {
        if self.leases.normalized_in_initial == NormalizationCharge::Charged
            || self.leases.normalized.borrow().is_some()
        {
            NormalizationCharge::Charged
        } else {
            NormalizationCharge::Uncharged
        }
    }
    pub(crate) fn hold_normalization(&self, lease: Arc<ResourceLease>) {
        *self.leases.normalized.borrow_mut() = Some(lease);
    }
    /// Whether `other` leases the same record.
    #[cfg(test)]
    pub(crate) fn same_record(
        &self,
        other: &Self,
    ) -> crate::application::file_dag::relations::types::Sameness {
        if Arc::ptr_eq(&self.record, &other.record) {
            crate::application::file_dag::relations::types::Sameness::Same
        } else {
            crate::application::file_dag::relations::types::Sameness::Different
        }
    }
}

/// What the store published for a holder: a relation's publication, or packed words it leases. The one handle the runtime holds.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::relations::publication::types::PublicationBody;
/// use ryo_execution_store::application::file_dag::fixtures::program_derived_port;
/// use ryo_execution_store::application::file_dag::relations::{interface::RelationsPort, service::RelationOwner};
/// use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
/// use ryo_relations::domain::relations::types::{CellKind, FileId, Value};
/// use ryo_values::domain::rows::types::{RowCount, Table};
/// let table = Table { kinds: vec![CellKind::File], cells: vec![Value::File(FileId(0))], count: RowCount(1) };
/// let owner = RelationOwner::fixture(ReuseEligibility::Eligible, program_derived_port());
/// let publication = owner.publish(PublicationBody::Computed(table))?;
/// assert_eq!(owner.kinds(&publication)?.len(), 1);
/// # Ok::<(), ryo_execution_domain::domain::operating::errors::OperatingError>(())
/// ```
#[derive(Clone, Debug)]
pub struct Publication(pub(crate) PublicationHold);

/// What a publication holds.
#[derive(Clone, Debug)]
pub(crate) enum PublicationHold {
    Relation(PublishedInput),
    /// Packed words; the lease is held, never read.
    Packed {
        _lease: crate::application::file_dag::relations::packed::service::PackedPublication,
    },
}
impl Publication {
    /// Whether `other` holds the same published record, for the engine's own proofs.
    #[cfg(any(test, feature = "fixtures"))]
    #[must_use]
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_domain::domain::operating::errors::OperatingError;
    /// use ryo_execution_store::application::file_dag::relations::publication::types::PublicationBody;
    /// use ryo_execution_store::application::file_dag::fixtures::program_derived_port;
    /// use ryo_execution_store::application::file_dag::relations::{interface::RelationsPort, service::RelationOwner};
    /// use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
    /// use ryo_relations::domain::relations::types::{CellKind, FileId, Value};
    /// use ryo_values::domain::rows::types::{RowCount, Table};
    /// let table = Table { kinds: vec![CellKind::File], cells: vec![Value::File(FileId(0))], count: RowCount(1) };
    /// let owner = RelationOwner::fixture(ReuseEligibility::Eligible, program_derived_port());
    /// let publication = owner.publish(PublicationBody::Computed(table.clone()))?;
    /// assert!(publication.same_record(&publication.clone()).is_ok());
    /// let other = owner.publish(PublicationBody::Computed(table))?;
    /// assert!(matches!(publication.same_record(&other), Err(OperatingError::NotRead)));
    /// # Ok::<(), OperatingError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// `NotRead` when it holds another record, or either holds packed words.
    pub fn same_record(
        &self,
        other: &Self,
    ) -> Result<(), ryo_execution_domain::domain::operating::errors::OperatingError> {
        match (&self.0, &other.0) {
            (PublicationHold::Relation(this), PublicationHold::Relation(that))
                if Arc::ptr_eq(&this.record, &that.record) =>
            {
                Ok(())
            }
            _ => Err(ryo_execution_domain::domain::operating::errors::OperatingError::NotRead),
        }
    }
    /// The publication of a relation: what the store's tests and fixtures hand a workspace directly.
    #[cfg(any(test, feature = "fixtures"))]
    pub(crate) fn of(input: PublishedInput) -> Self {
        Self(PublicationHold::Relation(input))
    }
    /// The relation's publication.
    pub(crate) fn input(
        &self,
    ) -> Result<&PublishedInput, ryo_execution_domain::domain::operating::errors::OperatingError>
    {
        match &self.0 {
            PublicationHold::Relation(input) => Ok(input),
            PublicationHold::Packed { .. } => {
                Err(ryo_execution_domain::domain::operating::errors::OperatingError::NotRead)
            }
        }
    }
}
