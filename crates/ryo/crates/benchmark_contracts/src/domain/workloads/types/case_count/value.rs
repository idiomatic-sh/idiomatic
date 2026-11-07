//! The existing value and its behaviour, owned together.
/// Number of independently executed correctness cases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaseCount(
    /// Number of independently authored cases checked.
    pub usize,
);

impl std::fmt::Display for CaseCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
