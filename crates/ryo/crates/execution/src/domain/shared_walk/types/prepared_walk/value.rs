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
    physical: Vec<Option<ClassPlace>>,
}

impl PreparedWalk {
    pub(crate) fn compile(plans: &[Plan], bindings: &[Vec<ParameterBinding>]) -> Self {
        Self {
            classes: merge(plans, bindings),
            physical: Vec::new(),
        }
    }

    /// Direct lookup in the immutable physical program, prepared once per request.
    pub(crate) fn class_for(
        &self,
        instruction: &crate::domain::executable_program::types::Instruction,
    ) -> Option<ClassPlace> {
        self.physical.get(instruction.logical.0).copied().flatten()
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
        let ryo_values::domain::plans::types::Operation::Select {
            from: Input::Relation(relation),
            comparisons,
            kept,
        } = &instruction.operation
        else {
            return None;
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
            && self.classes().get(class.0)?.comparisons == *comparisons)
            .then_some(class)
    }

    /// Exclude logical classes with no physical consumer of the shared result.
    pub(crate) fn retain_program_consumers(
        &mut self,
        program: &crate::domain::executable_program::types::ExecutableProgram,
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
        self.physical = vec![None; program.logical_regions().len()];
        for region in program.regions() {
            let Some(head) = region.instructions.first() else {
                continue;
            };
            // Synthetic observation unions are not graph-backed selection heads.
            // Resolve eligibility before indexing the ordinary logical namespace.
            if let Some(class) = self.resolve_class(program, head) {
                *self
                    .physical
                    .get_mut(head.logical.0)
                    .ok_or(OperatingError::NotRun)? = Some(class);
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

/// Sparse results for only classes reached in this admitted population.
#[derive(Debug)]
pub(crate) struct NativeBatch {
    class_count: ClassCount,
    classes: std::collections::BTreeMap<
        ClassPlace,
        (crate::domain::shared_walk::types::ClassResult, RowsVisited),
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
    pub(in crate::domain::shared_walk) fn record_scratch_reset(&mut self) {
        self.counts.scratch_classes.0 = self.counts.scratch_classes.0.saturating_add(1);
    }
    pub(crate) fn class(
        &self,
        place: ClassPlace,
    ) -> Option<&crate::domain::shared_walk::types::ClassResult> {
        self.classes.get(&place).map(|(result, _)| result)
    }
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
        let _previous = self.classes.insert(place, (result, visits));
        self.counts.result_slots.0 = self.counts.result_slots.0.saturating_add(1);
        self.counts.predicate_rows.0 = self.counts.predicate_rows.0.saturating_add(visits.0);
    }
}

use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::query_bindings::types::ParameterBinding;
