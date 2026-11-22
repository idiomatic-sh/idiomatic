//! The types of the `splicing` concept: plain data, public fields and derives.

use crate::domain::lowering::types::{OriginCount, PlannedColumn};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::hosted::types::HostedRelation;
use ryo_values::domain::observations::types::BoundObservationInput;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::PlanStep;
use ryo_values::domain::program::types::{FragmentContractParameter, ParameterDeclaration};
use ryo_values::domain::query_tokens::types::{QuerySourceId, QuerySpan, QueryTraceSite};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// The steps the lowering of one expansion made, held to be spliced into a later plan whose expansion is the same: the
/// same fragment, the same values bound, over the same environment.
#[derive(Clone, Debug)]
pub(crate) struct LoweredExpansion {
    /// Where its first step stood in the plan it was lowered in: a step it reads at or after this place is its own, one
    /// before it a hosted relation's.
    pub start: StepId,
    /// How many comparison origins that plan held before it.
    pub origins_before: OriginCount,
    /// Its steps, in the order made.
    pub steps: Vec<SplicedStep>,
    /// The origins of its comparisons, in the order interned.
    pub origins: Vec<SplicedOrigin>,
    /// The call sites below the expansion's own that its origins name; the first is none.
    pub traces: Vec<Vec<QueryTraceSite>>,
}

/// One step an expansion's lowering made.
#[derive(Clone, Debug)]
pub(crate) struct SplicedStep {
    /// The step, reading its own steps at the places of the plan it was lowered in.
    pub step: PlanStep,
    /// Its columns.
    pub fields: Vec<PlannedColumn>,
    /// The definition of the expansion it is named by.
    pub label: SplicedLabel,
}

/// The name of a step an expansion's lowering made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SplicedLabel {
    /// No definition names it.
    Unnamed,
    /// The definition at this place among the expansion's names it.
    Definition(DefinitionPlace),
}

/// The place of a definition among the definitions of an expansion, from its first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DefinitionPlace(pub usize);

/// The place of a trace among the traces a lowered expansion holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TracePlace(pub usize);

/// The origin of one comparison of an expansion, below the call site of the expansion.
#[derive(Clone, Debug)]
pub(crate) struct SplicedOrigin {
    /// The document that wrote the comparison.
    pub source_id: QuerySourceId,
    /// Where it is written.
    pub span: QuerySpan,
    /// The call sites below the expansion's, as their place among the expansion's `traces`.
    pub trace: TracePlace,
}

/// What lowering reads besides a program's steps: a lowered expansion is spliced only over the environment it was
/// lowered over.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LoweringEnvironment {
    /// The relations the query may read.
    pub catalog: Vec<Relation>,
    /// The relations the host declared.
    pub hosted: Vec<HostedRelation>,
    /// The bound observation inputs.
    pub observations: Vec<BoundObservationInput>,
    /// The parameters the root declares.
    pub parameters: Vec<ParameterDeclaration>,
}

/// One expansion's lowering kept on a shelf, with the values its call bound.
#[derive(Debug)]
pub(crate) struct KeptExpansion {
    /// The values the call bound.
    pub parameters: Vec<FragmentContractParameter>,
    /// What its lowering made.
    pub lowered: Arc<LoweredExpansion>,
}

/// The expansions lowered over one environment, by fragment: at most `LOWERED_EXPANSIONS` of every fragment.
#[derive(Debug, Default)]
pub(crate) struct ExpansionShelf(pub Mutex<HashMap<QuerySourceId, Vec<KeptExpansion>>>);

/// The shelf of one environment.
#[derive(Debug)]
pub(crate) struct EnvironmentShelf {
    /// What lowering reads besides a program's steps.
    pub environment: LoweringEnvironment,
    /// The expansions lowered over it.
    pub shelf: Arc<ExpansionShelf>,
}

/// The shelves of expansions lowered, one per environment, for the fragments of one set of parsed modules: at most
/// `LOWERING_ENVIRONMENTS` shelves of at most `LOWERED_EXPANSIONS` expansions each.
#[derive(Debug, Default)]
pub(crate) struct LoweredExpansions(pub Mutex<Vec<EnvironmentShelf>>);
