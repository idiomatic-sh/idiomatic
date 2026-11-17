//! What a measured run's observations count.

/// Per-rule finding multiplicities, written explicitly rather than as debug representations.
#[derive(Clone, Debug)]
pub(crate) struct FindingCounts<'a>(pub std::collections::BTreeMap<&'a ryo::linter::RuleId, usize>);

impl std::fmt::Display for FindingCounts<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, (rule, count)) in self.0.iter().enumerate() {
            if index != 0 {
                f.write_str(super::constants::FINDING_SEPARATOR)?;
            }
            write!(f, "{rule}={count}")?;
        }
        Ok(())
    }
}
