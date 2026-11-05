//! Opaque request preparation and independent results for shared relation walks.

use crate::domain::{
    coverage::types::RuleIndex,
    operating::errors::OperatingError,
    positions::types::{ClassPlace, Slots},
    step_classes::{
        service::merge,
        types::{Merge, StepClass},
    },
};
use ryo_values::domain::plans::types::Plan;

/// Number of rules whose preparations the native pass covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WalkRuleCount(pub usize);

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
}

impl PreparedWalk {
    pub(crate) fn compile(plans: &[Plan], bindings: &[Vec<ParameterBinding>]) -> Self {
        Self {
            classes: merge(plans, bindings),
        }
    }

    /// Resolve only a physical selection head whose rows this walk can deliver.
    pub(crate) fn class_for(
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
    ) {
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
                            .and_then(|instruction| self.class_for(program, instruction))
                            == Some(ClassPlace(at))
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        for (class, members) in self.classes.classes.iter_mut().zip(retained) {
            class.members = members;
        }
    }

    pub(crate) fn classes(&self) -> &[StepClass] {
        &self.classes.classes
    }

    pub(crate) fn rule_count(&self) -> WalkRuleCount {
        WalkRuleCount(self.classes.class_of.len())
    }

    pub(crate) fn class_of(&self, rule: RuleIndex) -> Option<&[Option<ClassPlace>]> {
        self.classes.class_of.get(rule.0).map(Vec::as_slice)
    }
}

/// None declined before semantic work; a reached class retains its own result.
#[derive(Debug)]
pub(crate) struct NativeBatch {
    classes: Vec<Option<crate::domain::shared_walk::types::ClassResult>>,
    counts: crate::domain::shared_walk::types::WalkCounts,
    visits: Vec<RowsVisited>,
}

impl NativeBatch {
    pub(crate) fn counts(&self) -> crate::domain::shared_walk::types::WalkCounts {
        self.counts
    }
    pub(in crate::domain::shared_walk) fn record_pass(&mut self) {
        self.counts.passes.0 = self.counts.passes.0.saturating_add(1);
    }
    pub(in crate::domain::shared_walk) fn record_source_row(&mut self) {
        self.counts.source_rows.0 = self.counts.source_rows.0.saturating_add(1);
    }
    pub(crate) fn class(
        &self,
        place: ClassPlace,
    ) -> Option<&crate::domain::shared_walk::types::ClassResult> {
        self.classes.get(place.0).and_then(Option::as_ref)
    }

    /// Candidate rows actually offered to a reached class, including its first error row.
    pub(crate) fn visits(&self, place: ClassPlace) -> RowsVisited {
        self.visits[place.0]
    }

    pub(in crate::domain::shared_walk) fn empty(count: ClassCount) -> Self {
        let mut classes = Vec::with_capacity(count.0);
        classes.resize_with(count.0, || None);
        Self {
            classes,
            visits: vec![RowsVisited::default(); count.0],
            counts: crate::domain::shared_walk::types::WalkCounts::default(),
        }
    }

    pub(in crate::domain::shared_walk) fn set(
        &mut self,
        place: ClassPlace,
        result: Result<Slots, OperatingError>,
    ) {
        use crate::domain::shared_walk::types::ClassResult;
        self.classes[place.0] = Some(match result {
            Ok(positions) => ClassResult {
                positions,
                completion: Ok(()),
            },
            Err(error) => ClassResult {
                positions: self.classes[place.0]
                    .take()
                    .map(|result| result.positions)
                    .unwrap_or_default(),
                completion: Err(error),
            },
        });
    }

    pub(in crate::domain::shared_walk) fn output(
        &mut self,
        place: ClassPlace,
    ) -> Option<&mut Slots> {
        match &mut self.classes[place.0] {
            Some(result) if result.completion.is_ok() => Some(&mut result.positions),
            Some(_) | None => None,
        }
    }

    pub(in crate::domain::shared_walk) fn record_visit(&mut self, place: ClassPlace) {
        self.visits[place.0].0 = self.visits[place.0].0.saturating_add(1);
    }
}

use ryo_values::domain::query_bindings::types::ParameterBinding;

use ryo_values::domain::plan_parts::types::Input;
