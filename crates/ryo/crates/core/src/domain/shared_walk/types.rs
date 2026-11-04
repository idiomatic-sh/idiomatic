//! File-local scratch for one borrowed tree pass.

use crate::domain::{
    operating::types::Memo,
    positions::types::ClassPlace,
    step_classes::constants::ELEMENT_COLUMNS,
    table_cells::types::ColumnPlaces,
    table_scratch::types::{ComparisonNumbers, TextVerdicts},
};
use ryo_relations::domain::relations::types::{Flag, Value};

pub(crate) mod prepared_walk;

/// The number of kinds named by one prepared class.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct KindCount(pub usize);

/// Whether the caller admitted a rule into this file's shared native pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuleActivity {
    Active,
    Inactive,
}

/// Whether a class can be decided by the shared pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Eligibility {
    #[default]
    Declined,
    Ready,
}

/// Cells of one source row are read at most once for its admitted classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowRead {
    Pending,
    Done,
}

/// Reusable file-local comparison work. No request plan or published rows live here.
#[derive(Debug, Default)]
pub(crate) struct ClassWork {
    pub kinds: KindCount,
    pub gate: ComparisonNumbers,
    pub text_ok: TextVerdicts,
    pub rest: ComparisonNumbers,
    pub needed: ColumnPlaces,
    pub memo: Vec<Memo>,
    pub eligible: Eligibility,
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
