//! Canonical findings use relative paths and UTF-8 byte offsets.

/// One normalized finding. Duplicate values remain significant.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diagnostic {
    pub rule: String,
    pub file: String,
    pub start: usize,
    pub end: usize,
}

/// Decoded findings and the exact files submitted to the tool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunReport {
    pub diagnostics: Vec<Diagnostic>,
    pub files: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Difference {
    pub reference: Vec<Diagnostic>,
    pub candidate: Vec<Diagnostic>,
}
