//! The audit of the registered case receipts: which cases were verified and which failed.
use crate::domain::artifacts::types::digest::value::ArtifactDigest;
use crate::domain::requests::types::RuleLabel;
use crate::domain::workloads::types::benchmark_case_name::value::BenchmarkCaseName;
use crate::domain::workloads::types::case_count::value::CaseCount;

/// The rule and name that identify one registered case.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CaseKey {
    /// The rule the case belongs to.
    pub rule: RuleLabel,
    /// The case's name within its rule.
    pub name: BenchmarkCaseName,
}

/// A case whose declared file metadata the verifier recomputed independently.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetadataVerifiedCase {
    /// The verified case.
    pub case: CaseKey,
    /// The recomputed metadata digest, never echoed from pass flags.
    pub digest: ArtifactDigest,
}

/// One strict verification failure of a case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditFailure(
    /// The failing case and its reason.
    pub String,
);

/// An offline audit keeps valid pairs available for safe shard resume without hiding failures.
#[derive(Clone, Debug, Default)]
pub struct ReceiptAudit {
    /// Every registered case, including missing or invalid observations.
    pub case_count: CaseCount,
    /// Exact identities that independently passed the shared validator.
    pub verified: Vec<CaseKey>,
    /// Metadata contracts independently recomputed by this verifier, never echoed from pass flags.
    pub metadata_verified: Vec<MetadataVerifiedCase>,
    /// Strict failures, never skipped successes.
    pub errors: Vec<AuditFailure>,
    /// Raw report equality, excluding process observations.
    pub raw_exact: CaseCount,
    /// Documented diagnostic or process discrepancies.
    pub documented_discrepancies: CaseCount,
    /// Named documented tool policy differences, a subset of documented discrepancies.
    pub tool_contract_differences: Vec<CaseKey>,
    /// Explicit diagnostics-contract-only equality.
    pub contract_only: CaseCount,
}
