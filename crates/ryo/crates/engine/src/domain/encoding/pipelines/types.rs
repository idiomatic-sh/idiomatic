//! The types of the `pipelines` part: what a probe and a use are asked, as their stages state them.

use crate::domain::steps::stages::types::{ProbeRange, UseMode};
use crate::domain::steps::types::{
    ColumnMask, DefinitionId, ReadColumn, Register, RegisterList, StepOperand, StepOperands,
    StepRows,
};

/// What a probe is asked.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ProbeRead<'r> {
    /// Where the rows are read from.
    pub(crate) source: StepRows,
    /// The columns matched.
    pub(crate) columns: &'r [ReadColumn],
    /// Their values.
    pub(crate) values: &'r [StepOperand],
    /// The range the rows are held to, when there is one.
    pub(crate) range: Option<&'r ProbeRange>,
    /// What is done with what is found.
    pub(crate) mode: UseMode,
    /// The batch column each row's handle goes to.
    pub(crate) out: Register,
}

/// What a use of a definition from a pipeline is asked.
#[derive(Clone, Copy, Debug)]
pub(crate) struct StageUse<'u> {
    /// The definition.
    pub(crate) definition: DefinitionId,
    /// The columns that arrive bound.
    pub(crate) bound: &'u ColumnMask,
    /// The columns a row gives back.
    pub(crate) collected: &'u ColumnMask,
    /// The batch's cells for the bound columns.
    pub(crate) values: &'u StepOperands,
    /// What is done with what is found.
    pub(crate) mode: UseMode,
    /// The batch columns the collected cells go to.
    pub(crate) outputs: &'u RegisterList,
}

/// Where a stage stands in its pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PipelinePlace {
    /// First: the one place a `Scan` stands.
    First,
    /// After the first.
    Later,
}
