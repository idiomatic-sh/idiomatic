//! Opaque request preparation and independent results for the shared tree walk.

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
    classes: Vec<Option<Result<Slots, OperatingError>>>,
    visits: Vec<RowsVisited>,
}

impl NativeBatch {
    pub(crate) fn class(&self, place: ClassPlace) -> Option<&Result<Slots, OperatingError>> {
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
        }
    }

    pub(in crate::domain::shared_walk) fn set(
        &mut self,
        place: ClassPlace,
        result: Result<Slots, OperatingError>,
    ) {
        self.classes[place.0] = Some(result);
    }

    pub(in crate::domain::shared_walk) fn output(
        &mut self,
        place: ClassPlace,
    ) -> Option<&mut Slots> {
        match &mut self.classes[place.0] {
            Some(Ok(slots)) => Some(slots),
            Some(Err(_)) | None => None,
        }
    }

    pub(in crate::domain::shared_walk) fn record_visit(&mut self, place: ClassPlace) {
        self.visits[place.0].0 = self.visits[place.0].0.saturating_add(1);
    }
}

use ryo_values::domain::query_bindings::types::ParameterBinding;
