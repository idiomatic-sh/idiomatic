//! Canonical findings use relative paths and UTF-8 byte offsets.

/// One normalized finding. Duplicate values remain significant.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diagnostic {
    /// Rule identifier within the pinned default profile.
    pub rule: String,
    /// Relative path of the source containing the finding.
    pub file: String,
    /// Inclusive start offset in UTF-8 bytes.
    pub start: usize,
    /// Exclusive end offset in UTF-8 bytes.
    pub end: usize,
}

/// Decoded findings and the exact files submitted to the tool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunReport {
    /// Finding multiset, preserving duplicate occurrences.
    pub diagnostics: Vec<Diagnostic>,
    /// Exact relative source paths submitted to the tool.
    pub files: Vec<String>,
}

/// Sorted finding multisets retained when comparison does not agree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Difference {
    /// Sorted findings returned by the reference.
    pub reference: Vec<Diagnostic>,
    /// Sorted findings returned by the candidate.
    pub candidate: Vec<Diagnostic>,
}
