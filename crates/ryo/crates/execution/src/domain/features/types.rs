//! The types of the `features` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::plans::types::Feature;
use ryo_values::domain::records::types::RowTotal;

/// How many rows one step was fed, over all its inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StepWork {
    /// What the step does.
    pub operation: ryo_values::domain::plans::types::OperationKind,
    /// The rows of its inputs.
    pub rows: RowTotal,
}

/// How many rows a feature was fed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FeatureRows {
    /// The feature.
    pub feature: Feature,
    /// The rows of the inputs of the steps that have it.
    pub rows: RowTotal,
}
