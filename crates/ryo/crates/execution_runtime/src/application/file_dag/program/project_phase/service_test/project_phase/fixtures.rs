//! The support of the `project_phase` cases.
use super::super::super::fixtures::submission::Root;

pub(super) fn root(rule: usize) -> Root {
    Root {
        declared: rule,
        named: rule,
    }
}
