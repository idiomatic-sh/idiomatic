//! The types of the `validation` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::program::types::StepName;

/// What a program's checks need to know about its inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Known<'a> {
    /// The names of the steps defined so far.
    pub steps: Vec<&'a StepName>,
    /// The names of the relations the host declared.
    pub hosted: &'a [ryo_values::domain::hosted::types::HostedName],
}
