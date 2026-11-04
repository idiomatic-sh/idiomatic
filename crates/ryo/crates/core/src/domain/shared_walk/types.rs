//! Prepared equivalence classes and file-local scratch for one borrowed tree pass.

use crate::domain::{
    operating::{errors::OperatingError, types::Memo},
    positions::types::{ClassPlace, Slots},
    step_classes::{
        constants::ELEMENT_COLUMNS,
        types::{Merge, StepClass},
    },
    table_cells::types::ColumnPlaces,
    table_scratch::types::{ComparisonNumbers, TextVerdicts},
};
use ryo_relations::domain::relations::types::{Flag, Value};

/// Request preparation retains comparisons and exact parameter-equivalence classes,
/// never a Plan, physical layout, provider session, or mutable file result.
#[derive(Debug)]
pub(crate) struct PreparedWalk {
    pub(super) classes: Merge,
}

impl PreparedWalk {
    pub(crate) fn classes(&self) -> &[StepClass] {
        &self.classes.classes
    }

    pub(crate) fn class_of(
        &self,
        rule: crate::domain::coverage::types::RuleIndex,
    ) -> Option<&[Option<ClassPlace>]> {
        self.classes.class_of.get(rule.0).map(Vec::as_slice)
    }
}

/// An absent class declined before semantic work. A reached class keeps its
/// successful slots or its own error; the caller may not retry a failed class.
#[derive(Debug)]
pub(crate) struct NativeBatch {
    classes: Vec<Option<Result<Slots, OperatingError>>>,
    visits: Vec<u64>,
}

impl NativeBatch {
    pub(crate) fn class(&self, place: ClassPlace) -> Option<&Result<Slots, OperatingError>> {
        self.classes.get(place.0).and_then(Option::as_ref)
    }

    pub(crate) fn into_classes(self) -> Vec<Option<Result<Slots, OperatingError>>> {
        self.classes
    }

    /// Candidate rows actually offered to a reached class, including the row
    /// that first failed its semantic comparison.
    pub(crate) fn visits(&self, place: ClassPlace) -> u64 {
        self.visits.get(place.0).copied().unwrap_or(0)
    }

    pub(super) fn empty(count: usize) -> Self {
        let mut classes = Vec::with_capacity(count);
        classes.resize_with(count, || None);
        Self {
            classes,
            visits: vec![0; count],
        }
    }

    pub(super) fn set(&mut self, place: ClassPlace, result: Result<Slots, OperatingError>) {
        if let Some(slot) = self.classes.get_mut(place.0) {
            *slot = Some(result);
        }
    }

    pub(super) fn output(&mut self, place: ClassPlace) -> Option<&mut Slots> {
        self.classes
            .get_mut(place.0)
            .and_then(Option::as_mut)
            .and_then(|result| result.as_mut().ok())
    }

    pub(super) fn record_visit(&mut self, place: ClassPlace) {
        if let Some(count) = self.visits.get_mut(place.0) {
            *count = count.saturating_add(1);
        }
    }
}

/// Reusable file-local comparison work. No request plan or published rows live here.
#[derive(Debug, Default)]
pub(crate) struct ClassWork {
    pub kinds: usize,
    pub gate: ComparisonNumbers,
    pub text_ok: TextVerdicts,
    pub rest: ComparisonNumbers,
    pub needed: ColumnPlaces,
    pub memo: Vec<Memo>,
    pub eligible: bool,
}

#[derive(Debug)]
pub(crate) struct WalkWork {
    pub work: Vec<ClassWork>,
    pub by_kind: Vec<Vec<ClassPlace>>,
    pub columns_by_kind: Vec<ColumnPlaces>,
    pub every: Vec<ClassPlace>,
    pub cells: [Value; ELEMENT_COLUMNS],
}

impl Default for WalkWork {
    fn default() -> Self {
        Self {
            work: Vec::new(),
            by_kind: Vec::new(),
            columns_by_kind: Vec::new(),
            every: Vec::new(),
            cells: [Value::Flag(Flag::Lowered); ELEMENT_COLUMNS],
        }
    }
}
