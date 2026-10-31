//! One value: its declaration and its hand-written impls, trait impls included.
#[derive(Clone, Debug, PartialEq, Eq)]
/// Default coverage produced by auditing a suite, never fabricated by a caller.
///
/// ```compile_fail
/// use ryo_benchmark::domain::suite::types::audit::value::Audit;
/// let forged = Audit { implemented: Vec::new(), missing: Vec::new() };
/// ```
pub struct Audit {
    pub(crate) implemented: Vec<String>,
    pub(crate) missing: Vec<String>,
}
impl Audit {
    /// Number of default rules with an authored query and valid case envelope.
    pub fn authored_count(&self) -> usize {
        self.implemented.len()
    }
    /// Authored subset; these rules still require executable correctness cases.
    pub fn authored(&self) -> &[String] {
        &self.implemented
    }
    /// Freeze an explicit nonempty subset; unselected authored rules remain outside coverage.
    pub fn select(
        &self,
        rules: &[String],
    ) -> Result<Self, crate::domain::diagnostics::errors::Error> {
        let selected: std::collections::BTreeSet<_> = rules.iter().cloned().collect();
        if selected.is_empty()
            || selected.len() != rules.len()
            || selected.iter().any(|rule| !self.implemented.contains(rule))
        {
            return Err(crate::domain::diagnostics::errors::Error::Invalid(
                "selected subset must contain distinct authored default rules".into(),
            ));
        }
        let mut missing = self.missing.clone();
        missing.extend(
            self.implemented
                .iter()
                .filter(|rule| !selected.contains(*rule))
                .cloned(),
        );
        missing.sort();
        Ok(Self {
            implemented: selected.into_iter().collect(),
            missing,
        })
    }
}
