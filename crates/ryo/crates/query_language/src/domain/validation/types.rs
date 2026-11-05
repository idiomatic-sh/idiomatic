//! The types of the `validation` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::program::types::StepName;

/// What a program's checks need to know about its inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Known<'a> {
    /// Whether step references must already carry linker-assigned identities.
    pub linked: bool,
    /// The names of the steps defined so far.
    pub steps: Vec<&'a StepName>,
    /// The names of the relations the host declared.
    pub hosted: &'a [ryo_values::domain::hosted::types::HostedName],
    /// Exact typed external sources declared by this compilation request, not host tables.
    pub observations: &'a [ryo_values::domain::observations::types::BoundObservationInput],
}
