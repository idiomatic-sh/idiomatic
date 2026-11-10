//! One value: its declaration and its hand-written impls, trait impls included.
#![expect(
    clippy::field_scoped_visibility_modifiers,
    reason = "crate-scoped fields let checked workflows issue evidence while preventing external callers from constructing certified values"
)]
#[derive(Clone, Debug, PartialEq, Eq)]
/// Default coverage produced by auditing a suite, never fabricated by a caller.
///
/// ```compile_fail,E0451
/// use ryo_benchmark::domain::suite::types::audit::value::Audit;
/// let forged = Audit { implemented: Vec::new(), missing: Vec::new() };
/// ```
pub struct Audit {
    /// Default rules with an authored query and valid case envelope.
    pub(crate) implemented: Vec<ryo::linter::RuleId>,
    /// Default rules without an authored implementation in this inventory.
    pub(crate) missing: Vec<ryo::linter::RuleId>,
}
impl Audit {
    /// Number of default rules with an authored query and valid case envelope.
    #[must_use]
    pub fn authored_count(
        &self,
    ) -> ryo_benchmark_contracts::domain::workloads::types::rule_count::value::RuleCount {
        ryo_benchmark_contracts::domain::workloads::types::rule_count::value::RuleCount(
            self.implemented.len(),
        )
    }
    /// Authored subset; these rules still require executable correctness cases.
    #[must_use]
    pub fn authored(&self) -> &[ryo::linter::RuleId] {
        &self.implemented
    }
    /// Freeze an explicit nonempty subset; unselected authored rules remain outside coverage.
    ///
    /// # Errors
    /// Refuses an empty selection, duplicate identifiers or a rule absent from this audit.
    pub fn select(
        &self,
        rules: &[ryo::linter::RuleId],
    ) -> Result<Self, ryo_benchmark_contracts::domain::reports::errors::ReportsError> {
        let selected: std::collections::BTreeSet<_> = rules.iter().cloned().collect();
        if selected.is_empty()
            || selected.len() != rules.len()
            || selected.iter().any(|rule| !self.implemented.contains(rule))
        {
            return Err(
                ryo_benchmark_contracts::domain::reports::errors::ReportsError::Invalid(
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
