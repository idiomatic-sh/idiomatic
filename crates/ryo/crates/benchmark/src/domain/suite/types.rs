//! Audit, inventory and certification belong to the benchmark workflow.
pub mod audit;
pub mod certificate;
pub mod inventory;

/// Whether a profile's authored defaults own a rule's case evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaseOwnership {
    /// The rule is an explicit default and owns case evidence.
    Owns,
    /// The rule only supplies an executable query.
    Supports,
}

/// Whether a context declaration names a logical input as absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextAbsence {
    /// The context seals the input as absent.
    Sealed,
    /// The context does not name the input as absent.
    Unnamed,
}
