//! Compact rule facts for coverage, frontier retention and work accounting.
use crate::domain::plans::types::{Feature, OperationKind, StepScope};

/// The rows counted for a feature come from every input or one source position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeatureSource {
    AllInputs,
    Input(usize),
}

/// One source-bound feature of an original logical step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FeatureSlot {
    pub feature: Feature,
    pub source: FeatureSource,
}

/// Only identity and accounting facts remain; the executable operation lives in the program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepMetadata {
    pub operation: OperationKind,
    pub scope: StepScope,
    pub features: Box<[FeatureSlot]>,
}

/// The small facts needed beyond the executable program, without a retained Plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleRunMetadata {
    pub reads_project: bool,
    pub has_cross_steps: bool,
    /// A false slot is a cross-file observation input, never a per-file captured frontier.
    pub frontier_capture: Box<[bool]>,
    pub steps: Box<[StepMetadata]>,
}
