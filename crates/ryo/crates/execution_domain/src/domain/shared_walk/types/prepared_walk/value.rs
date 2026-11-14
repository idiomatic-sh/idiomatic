//! Opaque request preparation and independent results for shared relation walks.

use crate::domain::{
    coverage::types::RuleIndex,
    operating::errors::OperatingError,
    shared_walk::types::{
        PredicateNumber, PredicatePlace,
        walk_route::value::{WalkBarrier, WalkRoute, source_barrier},
    },
    step_classes::types::ClassPlace,
    step_classes::{
        service::merge,
        types::{BindingsAgreement, Merge, StepClass},
    },
};
use ryo_values::domain::{
    plan_parts::types::Input, plans::types::Plan, query_bindings::types::ParameterBinding,
};

/// A request's exact comparison and binding equivalence, with no retained Plan.
#[derive(Debug)]
pub struct PreparedWalk {
    classes: Merge,
    /// The walk route of each logical instruction, when it has one.
    pub physical: Vec<Option<WalkRoute>>,
    prefixes: Vec<Vec<PredicatePlace>>,
    predicates: Vec<PreparedPredicate>,
    needed: Vec<crate::domain::table_cells::types::ColumnPlaces>,
}

impl PreparedWalk {
    /// Prepares the shared walk of a request's plans and parameter bindings.
    pub fn compile(plans: &[Plan], bindings: &[Vec<ParameterBinding>]) -> Self {
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
        // Predicates bucketed by (parent, source): only these can be equal.
        let mut keyed: rustc_hash::FxHashMap<
            (
                Option<PredicatePlace>,
                ryo_relations::domain::relations::types::Relation,
            ),
            Vec<usize>,
        > = rustc_hash::FxHashMap::default();
        for (at, class) in self.classes.classes.iter().enumerate() {
            let mut prefixes = Vec::new();
            let mut parent = None;
            for (number, comparison) in class.comparisons.iter().enumerate() {
                let existing = keyed.get(&(parent, class.source)).and_then(|bucket| {
                    bucket.iter().copied().find(|&place| {
                        let candidate = &self.predicates[place];
                        let other = &self.classes.classes[candidate.class.0];
                        other.comparisons[candidate.number.0] == *comparison
                            && crate::domain::step_classes::service::predicate_bindings_match(
                                std::slice::from_ref(comparison),
                                bindings.get(other.first.rule.0).map(Vec::as_slice),
                                bindings.get(class.first.rule.0).map(Vec::as_slice),
                            ) == BindingsAgreement::Equal
                    })
                });
                let place = match existing {
                    Some(place) => PredicatePlace(place),
                    None => {
                        let place = PredicatePlace(self.predicates.len());
                        keyed
                            .entry((parent, class.source))
                            .or_default()
                            .push(place.0);
                        self.predicates.push(PreparedPredicate {
                            class: ClassPlace(at),
                            number: PredicateNumber(number),
                            comparison: comparison.clone(),
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
                &ryo_operators::domain::table_scratch::types::ComparisonNumbers(
                    (0..class.comparisons.len()).collect(),
                ),
                &mut needed,
            );
            self.prefixes.push(prefixes);
            self.needed.push(needed);
        }
    }
    /// Count the exact routes chosen during binding, independently of later demand.
    pub fn census(&self) -> crate::domain::shared_walk::types::WalkCensus {
        use crate::domain::shared_walk::types::{WalkCensus, WalkRouteCount};
        let mut census = WalkCensus {
            instructions: WalkRouteCount(self.physical.iter().flatten().count()),
            ..WalkCensus::default()
        };
        let mut classes = std::collections::BTreeSet::new();
        for route in self.physical.iter().flatten() {
            let count = match route {
                WalkRoute::Shared(class) => {
                    classes.insert(class.0);
                    &mut census.shared
                }
                WalkRoute::Barrier(WalkBarrier::Representation) => &mut census.representation,
                WalkRoute::Barrier(WalkBarrier::DependentInput) => &mut census.dependent,
                WalkRoute::Barrier(WalkBarrier::Observation) => &mut census.observation,
                WalkRoute::Barrier(WalkBarrier::RowSemantics) => &mut census.row,
                WalkRoute::Barrier(WalkBarrier::DerivedOrOrdered) => &mut census.derived,
                WalkRoute::Barrier(WalkBarrier::RequestOwnedSource) => &mut census.request,
                WalkRoute::Barrier(WalkBarrier::Continuation) => &mut census.continuation,
            };
            count.0 += 1;
        }
        census.classes = classes.into_iter().map(ClassPlace).collect();
        census
    }
    /// Resolve only a physical selection head whose rows this walk can deliver. A rule with
    /// an errored origin shares nothing; that rule is refused on its own roots by metadata.
    fn resolve_class(
        &self,
        program: &crate::domain::executable_program::types::executable_program::value::ExecutableProgram,
        instruction: &crate::domain::executable_program::types::instruction::value::Instruction,
    ) -> Option<ClassPlace> {
        let region = program.logical_regions().get(instruction.logical().0)?;
        let first = program
            .regions()
            .get(region.0)
            .and_then(|region| region.instructions().first())?;
        if first.logical() != instruction.logical() {
            return None;
        }
        let crate::domain::step_classes::types::PureSelection {
            from,
            comparisons,
            kept,
        } = crate::domain::step_classes::service::pure_selection(instruction.operation())?;
        let Input::Relation(relation) = from else {
            return None;
        };
        if kept.iter().any(|column| {
            !usize::try_from(column.0).is_ok_and(|at| {
                at < ryo_relations::domain::relations::service::columns(*relation).len()
            })
        }) {
            return None;
        }
        let rule = program.rules().get(instruction.producer().0)?;
        if rule.original_steps.iter().any(Result::is_err) {
            return None;
        }
        let original = rule
            .original_steps
            .iter()
            .flatten()
            .find(|origin| origin.logical == instruction.logical())?;
        let class = self
            .class_of(instruction.producer())
            .and_then(|classes| usize::try_from(original.step.0).map_or(None, |at| classes.get(at)))
            .copied()
            .flatten()?;
        let chosen = self.classes().get(class.0)?;
        (chosen.source == *relation && chosen.comparisons == comparisons).then_some(class)
    }

    /// Exclude logical classes with no physical consumer of the shared result.
    ///
    /// # Errors
    ///
    /// [`OperatingError::NotRun`] when the program's regions route one logical step twice or name a step outside the
    /// program.
    pub fn retain_program_consumers(
        &mut self,
        program: &crate::domain::executable_program::types::executable_program::value::ExecutableProgram,
    ) -> Result<(), OperatingError> {
        let mut retained = Vec::with_capacity(self.classes.classes.len());
        let mut dropped = Vec::new();
        for (at, class) in self.classes.classes.iter().enumerate() {
            let mut members = Vec::new();
            for &member in &class.members {
                let Some(origin) = program
                    .rules()
                    .get(member.rule.0)
                    .and_then(|rule| rule.original_steps.get(member.step.0))
                else {
                    continue;
                };
                // An errored origin leaves its whole rule out of sharing; only that rule is
                // refused, by its own roots in metadata.
                let Ok(origin) = origin else {
                    dropped.push(member);
                    continue;
                };
                let Some(region) = program
                    .logical_regions()
                    .get(origin.logical.0)
                    .and_then(|region| program.regions().get(region.0))
                else {
                    continue;
                };
                let Some(head) = region
                    .instructions()
                    .iter()
                    .find(|instruction| instruction.logical() == origin.logical)
                else {
                    continue;
                };
                if self.resolve_class(program, head) == Some(ClassPlace(at)) {
                    members.push(member);
                } else {
                    dropped.push(member);
                }
            }
            retained.push(members);
        }
        for member in dropped {
            if let Some(place) = self
                .classes
                .class_of
                .get_mut(member.rule.0)
                .and_then(|rule| rule.get_mut(member.step.0))
            {
                *place = None;
            }
        }
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
        // The trie was built once in `compile`; retention only drops the per-class
        // entries of removed classes. Predicates own their comparison, so shared
        // prefixes of the retained classes stay valid and equal.
        let mut kept = remap.iter().map(Option::is_some);
        self.prefixes.retain(|_| kept.next().unwrap_or(false));
        let mut kept = remap.iter().map(Option::is_some);
        self.needed.retain(|_| kept.next().unwrap_or(false));
        // Observation unions also own physical instructions, beyond the original
        // logical-to-region table. Allocate by actual instruction identity; holes
        // are absent instructions, never invented representation barriers.
        let slots = program
            .regions()
            .iter()
            .flat_map(|region| region.instructions().iter())
            .map(|instruction| instruction.logical().0)
            .max()
            .map_or(0, |last| last + 1);
        self.physical = vec![None; slots];
        for region in program.regions() {
            for (at, instruction) in region.instructions().iter().enumerate() {
                let route = self
                    .physical
                    .get_mut(instruction.logical().0)
                    .ok_or(OperatingError::NotRun)?;
                let barrier = if instruction.logical().0 >= program.logical_regions().len() {
                    WalkBarrier::Observation
                } else if at == 0 {
                    source_barrier(&instruction.operation(), &region.inputs())
                } else {
                    WalkBarrier::Continuation
                };
                if route.replace(WalkRoute::Barrier(barrier)).is_some() {
                    return Err(OperatingError::NotRun);
                }
            }
            let Some(head) = region.instructions().first() else {
                continue;
            };
            if let Some(class) = self.resolve_class(program, head) {
                *self
                    .physical
                    .get_mut(head.logical().0)
                    .ok_or(OperatingError::NotRun)? = Some(WalkRoute::Shared(class));
            }
        }
        Ok(())
    }
    pub(crate) fn prefixes(&self, class: ClassPlace) -> &[PredicatePlace] {
        &self.prefixes[class.0]
    }
    pub(crate) fn comparison(
        &self,
        predicate: PredicatePlace,
    ) -> &ryo_values::domain::plan_parts::types::PlanComparison {
        &self.predicates[predicate.0].comparison
    }
    pub(crate) fn needed(
        &self,
        class: ClassPlace,
    ) -> &crate::domain::table_cells::types::ColumnPlaces {
        &self.needed[class.0]
    }

    /// Direct lookup in the immutable physical program, prepared once per request.
    pub fn class_for(
        &self,
        instruction: &crate::domain::executable_program::types::instruction::value::Instruction,
    ) -> Option<ClassPlace> {
        match self.physical.get(instruction.logical().0)?.as_ref()? {
            WalkRoute::Shared(class) => Some(*class),
            WalkRoute::Barrier(_barrier) => None,
        }
    }

    /// The route bound to a physical instruction, if it was bound.
    pub fn route_of(
        &self,
        instruction: &crate::domain::executable_program::types::instruction::value::Instruction,
    ) -> Option<WalkRoute> {
        *self.physical.get(instruction.logical().0)?
    }

    /// The classes of the walk.
    pub fn classes(&self) -> &[StepClass] {
        &self.classes.classes
    }

    /// The class of each step of a rule, when the step has one.
    pub fn class_of(&self, rule: RuleIndex) -> Option<&[Option<ClassPlace>]> {
        self.classes.class_of.get(rule.0).map(Vec::as_slice)
    }
}

#[derive(Debug)]
struct PreparedPredicate {
    /// The class and place that first defined it; meaningful only while building the trie.
    class: ClassPlace,
    number: PredicateNumber,
    /// Owned, so retention can drop its defining class without invalidating the trie.
    comparison: ryo_values::domain::plan_parts::types::PlanComparison,
}
