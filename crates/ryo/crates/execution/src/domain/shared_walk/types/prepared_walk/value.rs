//! Opaque request preparation and independent results for shared relation walks.

use crate::domain::{
    coverage::types::RuleIndex,
    operating::errors::OperatingError,
    positions::types::ClassPlace,
    step_classes::{
        service::merge,
        types::{Merge, StepClass},
    },
};
use ryo_values::domain::plans::types::Plan;

/// Number of equivalence classes in one prepared request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ClassCount(pub usize);

/// Candidate source rows offered to a reached class.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RowsVisited(pub u64);

/// A request's exact comparison and binding equivalence, with no retained Plan.
#[derive(Debug)]
pub(crate) struct PreparedWalk {
    classes: Merge,
    physical: Vec<WalkRoute>,
    prefixes: Vec<Vec<PredicatePlace>>,
    predicates: Vec<PreparedPredicate>,
    needed: Vec<crate::domain::table_cells::types::ColumnPlaces>,
}

impl PreparedWalk {
    pub(crate) fn compile(plans: &[Plan], bindings: &[Vec<ParameterBinding>]) -> Self {
        let mut prepared = Self {
            classes: merge(plans, bindings),
            physical: Vec::new(),
            prefixes: Vec::new(),
            predicates: Vec::new(),
            needed: Vec::new(),
        };
        prepared.prepare_predicates(bindings);
        prepared
    }

    /// Ordered prefixes share only equal predicates over the same source and
    /// referenced typed bindings. A failing prefix never opens a later predicate.
    fn prepare_predicates(&mut self, bindings: &[Vec<ParameterBinding>]) {
        self.prefixes.clear();
        self.predicates.clear();
        self.needed.clear();
        for (at, class) in self.classes.classes.iter().enumerate() {
            let mut prefixes = Vec::new();
            let mut parent = None;
            for (number, comparison) in class.comparisons.iter().enumerate() {
                let existing = self.predicates.iter().position(|candidate| {
                    let other = &self.classes.classes[candidate.class.0];
                    candidate.parent == parent
                        && other.source == class.source
                        && other.comparisons[candidate.number] == *comparison
                        && crate::domain::step_classes::service::predicate_bindings_match(
                            std::slice::from_ref(comparison),
                            bindings.get(other.first.rule.0).map(Vec::as_slice),
                            bindings.get(class.first.rule.0).map(Vec::as_slice),
                        )
                });
                let place = match existing {
                    Some(place) => PredicatePlace(place),
                    None => {
                        let place = PredicatePlace(self.predicates.len());
                        self.predicates.push(PreparedPredicate {
                            class: ClassPlace(at),
                            number,
                            parent,
                        });
                        place
                    }
                };
                prefixes.push(place);
                parent = Some(place);
            }
            let mut needed = crate::domain::table_cells::types::ColumnPlaces::default();
            crate::domain::text_gate::service::needed_of_rest(
                &class.comparisons,
                &crate::domain::table_scratch::types::ComparisonNumbers(
                    (0..class.comparisons.len()).collect(),
                ),
                &mut needed,
            );
            self.prefixes.push(prefixes);
            self.needed.push(needed);
        }
    }
    pub(crate) fn prefixes(&self, class: ClassPlace) -> &[PredicatePlace] {
        &self.prefixes[class.0]
    }
    pub(crate) fn comparison(
        &self,
        predicate: PredicatePlace,
    ) -> &ryo_values::domain::plan_parts::types::Comparison {
        let prepared = &self.predicates[predicate.0];
        &self.classes.classes[prepared.class.0].comparisons[prepared.number]
    }
    pub(crate) fn needed(
        &self,
        class: ClassPlace,
    ) -> &crate::domain::table_cells::types::ColumnPlaces {
        &self.needed[class.0]
    }

    /// Direct lookup in the immutable physical program, prepared once per request.
    pub(crate) fn class_for(
        &self,
        instruction: &crate::domain::executable_program::types::Instruction,
    ) -> Option<ClassPlace> {
        match self.physical.get(instruction.logical.0)? {
            WalkRoute::Shared(class) => Some(*class),
            WalkRoute::Barrier(_barrier) => None,
        }
    }

    /// Resolve only a physical selection head whose rows this walk can deliver.
    fn resolve_class(
        &self,
        program: &crate::domain::executable_program::types::ExecutableProgram,
        instruction: &crate::domain::executable_program::types::Instruction,
    ) -> Option<ClassPlace> {
        let region = program.logical_regions().get(instruction.logical.0)?;
        if program
            .regions()
            .get(region.0)?
            .instructions
            .first()?
            .logical
            != instruction.logical
        {
            return None;
        }
        use ryo_values::domain::{plan_parts::types::RepeatedRows, plans::types::Operation};
        let (relation, comparisons, kept) = match &instruction.operation {
            Operation::Select {
                from: Input::Relation(relation),
                comparisons,
                kept,
            } => (relation, comparisons.as_slice(), kept),
            Operation::Project {
                from: Input::Relation(relation),
                columns,
                distinct: RepeatedRows::Kept,
            } => (relation, &[][..], columns),
            _ => return None,
        };
        if kept.iter().any(|column| {
            column.0 as usize >= ryo_relations::domain::relations::service::columns(*relation).len()
        }) {
            return None;
        }
        let original = program
            .rules()
            .get(instruction.producer.0)?
            .original_steps
            .iter()
            .filter_map(|origin| origin.as_ref().ok())
            .find(|origin| origin.logical == instruction.logical)?;
        let class = self
            .class_of(instruction.producer)?
            .get(original.step.0 as usize)
            .copied()
            .flatten()?;
        (self.classes().get(class.0)?.source == *relation
            && self.classes().get(class.0)?.comparisons == comparisons)
            .then_some(class)
    }

    /// Exclude logical classes with no physical consumer of the shared result.
    pub(crate) fn retain_program_consumers(
        &mut self,
        program: &crate::domain::executable_program::types::ExecutableProgram,
        bindings: &[Vec<ParameterBinding>],
    ) -> Result<(), OperatingError> {
        let retained = self
            .classes
            .classes
            .iter()
            .enumerate()
            .map(|(at, class)| {
                class
                    .members
                    .iter()
                    .copied()
                    .filter(|member| {
                        let Some(origin) = program
                            .rules()
                            .get(member.rule.0)
                            .and_then(|rule| rule.original_steps.get(member.step.0))
                            .and_then(|origin| origin.as_ref().ok())
                        else {
                            return false;
                        };
                        let Some(region) = program
                            .logical_regions()
                            .get(origin.logical.0)
                            .and_then(|region| program.regions().get(region.0))
                        else {
                            return false;
                        };
                        region
                            .instructions
                            .iter()
                            .find(|instruction| instruction.logical == origin.logical)
                            .and_then(|instruction| self.resolve_class(program, instruction))
                            == Some(ClassPlace(at))
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let mut remap = vec![None; self.classes.classes.len()];
        let mut compact = Vec::new();
        for (old, (mut class, members)) in std::mem::take(&mut self.classes.classes)
            .into_iter()
            .zip(retained)
            .enumerate()
        {
            if members.is_empty() {
                continue;
            }
            class.members = members;
            class.first = class.members[0];
            remap[old] = Some(ClassPlace(compact.len()));
            compact.push(class);
        }
        self.classes.classes = compact;
        for rule in &mut self.classes.class_of {
            for class in rule {
                *class = class.and_then(|old| remap[old.0]);
            }
        }
        self.prepare_predicates(bindings);
        self.physical =
            vec![WalkRoute::Barrier(WalkBarrier::Representation); program.logical_regions().len()];
        for region in program.regions() {
            for (at, instruction) in region.instructions.iter().enumerate() {
                // Synthetic observation union heads have their own namespace and
                // remain publication barriers, never ordinary source-walk classes.
                let Some(route) = self.physical.get_mut(instruction.logical.0) else {
                    continue;
                };
                *route = WalkRoute::Barrier(if at == 0 {
                    source_barrier(&instruction.operation)
                } else {
                    WalkBarrier::Continuation
                });
            }
            let Some(head) = region.instructions.first() else {
                continue;
            };
            if let Some(class) = self.resolve_class(program, head) {
                *self
                    .physical
                    .get_mut(head.logical.0)
                    .ok_or(OperatingError::NotRun)? = WalkRoute::Shared(class);
            }
        }
        Ok(())
    }

    pub(crate) fn classes(&self) -> &[StepClass] {
        &self.classes.classes
    }

    pub(crate) fn class_of(&self, rule: RuleIndex) -> Option<&[Option<ClassPlace>]> {
        self.classes.class_of.get(rule.0).map(Vec::as_slice)
    }
}

/// Explicit source boundaries remain in request metadata. Adding an operation
/// requires choosing its source-walk barrier in this exhaustive classifier.
#[derive(Clone, Copy, Debug)]
enum WalkRoute {
    Shared(ClassPlace),
    Barrier(WalkBarrier),
}
#[derive(Clone, Copy, Debug)]
enum WalkBarrier {
    Representation,
    DependentInput,
    Observation,
    RowSemantics,
    DerivedOrOrdered,
    RequestOwnedSource,
    Continuation,
}
fn source_barrier(operation: &ryo_values::domain::plans::types::Operation) -> WalkBarrier {
    use ryo_values::domain::{plan_parts::types::Input, plans::types::Operation};
    match operation {
        Operation::Select { from, .. } | Operation::Project { from, .. } => match from {
            Input::Relation(_) => WalkBarrier::Representation,
            Input::Step(_) => WalkBarrier::DependentInput,
            Input::BoundObservation(_) => WalkBarrier::Observation,
        },
        Operation::Extend { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Replace { .. }
        | Operation::SpanBetween { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::SpanBounds { .. } => WalkBarrier::RowSemantics,
        Operation::Join { .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::Antijoin { .. }
        | Operation::GroupCount { .. }
        | Operation::GroupDigest { .. }
        | Operation::OrderedMatch { .. }
        | Operation::Words { .. }
        | Operation::Axis { .. }
        | Operation::Closure { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Cycles { .. } => WalkBarrier::DerivedOrOrdered,
        Operation::Rows { .. } | Operation::Hosted { .. } => WalkBarrier::RequestOwnedSource,
    }
}

/// Immutable identity of an exact ordered predicate prefix in this request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PredicatePlace(pub usize);
#[derive(Debug)]
struct PreparedPredicate {
    class: ClassPlace,
    number: usize,
    parent: Option<PredicatePlace>,
}

/// Sparse results for only classes reached in this admitted population.
#[derive(Debug)]
pub(crate) struct NativeBatch {
    class_count: ClassCount,
    classes: std::collections::BTreeMap<
        ClassPlace,
        (
            std::sync::Arc<crate::domain::shared_walk::types::ClassResult>,
            RowsVisited,
        ),
    >,
    counts: crate::domain::shared_walk::types::WalkCounts,
}

impl NativeBatch {
    pub(crate) fn for_program(program: &PreparedWalk) -> Self {
        Self::empty(ClassCount(program.classes().len()))
    }
    /// Preflight the whole transfer before committing any class or counter.
    pub(crate) fn merge(&mut self, next: Self) -> Result<(), OperatingError> {
        if self.class_count != next.class_count
            || next
                .classes
                .keys()
                .any(|class| self.classes.contains_key(class))
        {
            return Err(OperatingError::WrongOperation);
        }
        for (class, result) in next.classes {
            let _previous = self.classes.insert(class, result);
        }
        self.counts.passes.0 = self.counts.passes.0.saturating_add(next.counts.passes.0);
        self.counts.source_rows.0 = self
            .counts
            .source_rows
            .0
            .saturating_add(next.counts.source_rows.0);
        self.counts.predicate_rows.0 = self
            .counts
            .predicate_rows
            .0
            .saturating_add(next.counts.predicate_rows.0);
        self.counts.predicate_tests.0 = self
            .counts
            .predicate_tests
            .0
            .saturating_add(next.counts.predicate_tests.0);
        self.counts.predicate_reuses.0 = self
            .counts
            .predicate_reuses
            .0
            .saturating_add(next.counts.predicate_reuses.0);
        self.counts.decoded_rows.0 = self
            .counts
            .decoded_rows
            .0
            .saturating_add(next.counts.decoded_rows.0);
        self.counts.scratch_classes.0 = self
            .counts
            .scratch_classes
            .0
            .saturating_add(next.counts.scratch_classes.0);
        self.counts.result_slots.0 = self
            .counts
            .result_slots
            .0
            .saturating_add(next.counts.result_slots.0);
        Ok(())
    }
    pub(crate) fn counts(&self) -> crate::domain::shared_walk::types::WalkCounts {
        self.counts
    }
    pub(in crate::domain::shared_walk) fn record_pass(&mut self) {
        self.counts.passes.0 = self.counts.passes.0.saturating_add(1);
    }
    pub(in crate::domain::shared_walk) fn record_source_row(&mut self) {
        self.counts.source_rows.0 = self.counts.source_rows.0.saturating_add(1);
    }
    pub(in crate::domain::shared_walk) fn record_predicate_test(&mut self) {
        self.counts.predicate_tests.0 = self.counts.predicate_tests.0.saturating_add(1);
    }
    pub(in crate::domain::shared_walk) fn record_predicate_reuse(&mut self) {
        self.counts.predicate_reuses.0 = self.counts.predicate_reuses.0.saturating_add(1);
    }
    pub(in crate::domain::shared_walk) fn record_decoded_row(&mut self) {
        self.counts.decoded_rows.0 = self.counts.decoded_rows.0.saturating_add(1);
    }
    pub(in crate::domain::shared_walk) fn record_scratch_reset(&mut self) {
        self.counts.scratch_classes.0 = self.counts.scratch_classes.0.saturating_add(1);
    }
    pub(crate) fn class(
        &self,
        place: ClassPlace,
    ) -> Option<&crate::domain::shared_walk::types::ClassResult> {
        self.classes.get(&place).map(|(result, _)| result.as_ref())
    }
    pub(crate) fn lease(
        &self,
        place: ClassPlace,
    ) -> Option<(
        std::sync::Arc<crate::domain::shared_walk::types::ClassResult>,
        RowsVisited,
    )> {
        self.classes
            .get(&place)
            .map(|(result, visits)| (std::sync::Arc::clone(result), *visits))
    }
    #[cfg(test)]
    pub(crate) fn visits(&self, place: ClassPlace) -> RowsVisited {
        self.classes
            .get(&place)
            .map_or(RowsVisited::default(), |(_, visits)| *visits)
    }
    pub(in crate::domain::shared_walk) fn empty(class_count: ClassCount) -> Self {
        Self {
            class_count,
            classes: Default::default(),
            counts: Default::default(),
        }
    }
    pub(in crate::domain::shared_walk) fn store(
        &mut self,
        place: ClassPlace,
        result: crate::domain::shared_walk::types::ClassResult,
        visits: RowsVisited,
    ) {
        let _previous = self
            .classes
            .insert(place, (std::sync::Arc::new(result), visits));
        self.counts.result_slots.0 = self.counts.result_slots.0.saturating_add(1);
        self.counts.predicate_rows.0 = self.counts.predicate_rows.0.saturating_add(visits.0);
    }
}

use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::query_bindings::types::ParameterBinding;
