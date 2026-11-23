//! The types of the `expansion` pass: the plan being made and what a program in lowering can name.

use ryo_values::domain::finish_plans::arguments::types::StepNumber;
use ryo_values::domain::finish_plans::expressions::types::PlanConstant;
use ryo_values::domain::finish_plans::types::FinishStep;
use crate::domain::plan_lowering::filters::types::Parameters;
use crate::domain::plan_lowering::types::RuleContext;
use hashbrown::HashMap;
use ryo_values::domain::program::types::FragmentProgram;
use ryo_values::domain::query_tokens::types::QuerySourceId;
use std::collections::BTreeMap;

/// The steps made so far, each by its identity (equal steps are one), and the fragments a program may import.
#[derive(Debug)]
pub(crate) struct Expansion<'p> {
    /// The steps, in the order they were made.
    pub(crate) steps: Vec<FinishStep>,
    /// Each step's number, by the hash of its identity (the step with every map's entries in key order): the steps
    /// are held once, in `steps`.
    pub(crate) numbers: hashbrown::HashTable<(u64, StepNumber)>,
    /// The hasher of the identities.
    pub(crate) hasher: hashbrown::DefaultHashBuilder,
    /// The fragments of the pack, by name.
    pub(crate) fragments: &'p BTreeMap<QuerySourceId, FragmentProgram>,
    /// The step each fragment instance returned: a fragment given the same inputs and parameters makes the same
    /// steps, each already numbered, so it is lowered once.
    pub(crate) instances: HashMap<FragmentInstance<'p>, StepNumber>,
    /// What the run gives the rules beside the catalog.
    pub(crate) context: &'p RuleContext,
    /// The step each rule's observed output is, by (rule, output name), for the rules lowered so far.
    pub(crate) observed: HashMap<(&'p str, &'p str), StepNumber>,
    /// Whether rule observations can be read: only once every rule that reads none is lowered, so that each reader sees
    /// all of its producers.
    pub(crate) observations_open: bool,
}

/// A fragment as one call gives it: the fragment, the step each input names (in declaration order) and each
/// parameter's value (in declaration order, absent when not given).
#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) struct FragmentInstance<'p> {
    /// The fragment, by the name its imports give it.
    pub(crate) fragment: &'p str,
    /// Each input's step.
    pub(crate) inputs: Vec<StepNumber>,
    /// Each parameter's value.
    pub(crate) values: Vec<PlanConstant>,
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
