//! One value: its declaration and its hand-written impls, trait impls included.
#[derive(Clone, Debug, PartialEq, Eq)]
/// Default coverage produced by auditing a suite, never fabricated by a caller.
///
/// ```compile_fail
/// use ryo_benchmark::domain::suite::types::audit::value::Audit;
/// let forged = Audit { implemented: Vec::new(), missing: Vec::new() };
/// ```
pub struct Audit {
    pub(crate) implemented: Vec<ryo::linter::RuleId>,
    pub(crate) missing: Vec<ryo::linter::RuleId>,
}
impl Audit {
    /// Number of default rules with an authored query and valid case envelope.
    pub fn authored_count(&self) -> crate::domain::suite::types::RuleCount {
        crate::domain::suite::types::RuleCount(self.implemented.len())
    }
    /// Authored subset; these rules still require executable correctness cases.
    pub fn authored(&self) -> &[ryo::linter::RuleId] {
        &self.implemented
    }
    /// Freeze an explicit nonempty subset; unselected authored rules remain outside coverage.
    pub fn select(
        &self,
        rules: &[ryo::linter::RuleId],
    ) -> Result<Self, crate::domain::diagnostics::errors::DiagnosticsError> {
        let selected: std::collections::BTreeSet<_> = rules.iter().cloned().collect();
        if selected.is_empty()
            || selected.len() != rules.len()
            || selected.iter().any(|rule| !self.implemented.contains(rule))
        {
            return Err(
                crate::domain::diagnostics::errors::DiagnosticsError::Invalid(
                    crate::domain::suite::constants::INVALID_SELECTION.into(),
                ),
            );
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
