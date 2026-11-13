//! The counts and answers the relation owner keeps.

/// Resident source views an owner pins.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PinnedViewCount(pub usize);

/// Identity checks or inspection reuses an owner has counted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CheckCount(pub u64);

/// Whether two borrows are the same admitted allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Sameness {
    Same,
    Different,
}

/// Whether a publication's normalized table is already charged to its leases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NormalizationCharge {
    Charged,
    Uncharged,
}

/// Whether an admitted backing is a source relation the owner holds, or any other backing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BackingForm {
    Canonical,
    Foreign,
}

/// Whether a generation's admissions may still be consumed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Liveness {
    Live,
    Released,
}

/// Whether the resident source of a file may serve a later request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReuseEligibility {
    Eligible,
    Ineligible,
}

/// One producer of the runtime's program, as the store keys its population: the runtime names it, the store only compares it.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::relations::types::ProducerId;
/// assert_ne!(ProducerId(1), ProducerId(0));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProducerId(pub usize);

/// How many producers one program owner holds.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::relations::types::ProducerCapacity;
/// assert_eq!(ProducerCapacity(4).0, 4);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProducerCapacity(pub usize);

/// An answer a port may find or not: named at the boundary so no port signature carries a bare `Option`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sought<T> {
    /// The answer was found.
    Found(T),
    /// Nothing answers the request.
    Absent,
}

impl<T> From<Option<T>> for Sought<T> {
    fn from(found: Option<T>) -> Self {
        found.map_or(Self::Absent, Self::Found)
    }
}

impl<T> From<Sought<T>> for Option<T> {
    fn from(lookup: Sought<T>) -> Self {
        match lookup {
            Sought::Found(found) => Some(found),
            Sought::Absent => None,
        }
    }
}
