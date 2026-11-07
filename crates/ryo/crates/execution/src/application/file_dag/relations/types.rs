//! The counts and answers the relation owner keeps.

/// Resident source views an owner pins.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::application) struct PinnedViewCount(pub usize);

/// Identity checks or inspection reuses an owner has counted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::application::file_dag) struct CheckCount(pub u64);

/// Whether two borrows are the same admitted allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Sameness {
    Same,
    Different,
}

/// Whether a publication's normalized table is already charged to its leases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) enum NormalizationCharge {
    Charged,
    Uncharged,
}

/// Whether an admitted backing is a source relation the owner holds, or any other backing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) enum BackingForm {
    Canonical,
    Foreign,
}

/// Whether a generation's admissions may still be consumed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) enum Liveness {
    Live,
    Released,
}

/// Whether the resident source of a file may serve a later request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReuseEligibility {
    Eligible,
    Ineligible,
}
