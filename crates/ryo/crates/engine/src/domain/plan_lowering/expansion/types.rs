//! The types of the `expansion` pass: the plan being made and what a program in lowering can name.

use crate::domain::finish_plans::arguments::types::StepNumber;
use crate::domain::finish_plans::types::FinishStep;
use crate::domain::plan_lowering::filters::types::Parameters;
use hashbrown::HashMap;
use ryo_values::domain::program::types::FragmentProgram;
use ryo_values::domain::query_tokens::types::QuerySourceId;
use std::collections::BTreeMap;

/// The steps made so far, each by its identity (equal steps are one), and the fragments a program may import.
#[derive(Debug)]
pub(crate) struct Expansion<'p> {
    /// The steps, in the order they were made.
    pub(crate) steps: Vec<FinishStep>,
    /// Each step's number, by its identity: the step with every map's entries in key order.
    pub(crate) numbers: HashMap<FinishStep, StepNumber>,
    /// The fragments of the pack, by name.
    pub(crate) fragments: &'p BTreeMap<QuerySourceId, FragmentProgram>,
}

/// What a program being lowered can name: its steps and inputs, its parameters, and the fragments it imports.
#[derive(Debug, Default)]
pub(crate) struct ProgramScope<'p> {
    /// Each name's step.
    pub(crate) steps: HashMap<&'p str, StepNumber>,
    /// Its parameters' values.
    pub(crate) parameters: Parameters,
    /// Each imported name's fragment.
    pub(crate) imports: HashMap<&'p str, &'p str>,
}
