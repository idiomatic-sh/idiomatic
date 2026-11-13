//! The existing value and its behaviour, owned together.
/// Number of cases checked against a reference tool without running a candidate.
///
/// This result is intentionally distinct from `CaseCount`: reference-only checking
/// validates fixture expectations but cannot certify candidate behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReferenceCaseCount(
    /// Number of independently authored cases checked against the reference.
    pub usize,
);

impl std::fmt::Display for ReferenceCaseCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
