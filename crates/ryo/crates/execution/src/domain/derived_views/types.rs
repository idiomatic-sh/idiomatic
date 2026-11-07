//! Closed typed requirements, construction storage and immutable borrowed access.
use ryo_language::domain::{
    files::types::FilePath,
    symbols::types::{QuerySymbols, Symbols},
};
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, InputNumber, LiteralText},
    stored_facts::types::ArenaBytes,
};
use std::sync::Arc;

use crate::domain::positions::types::{Keys, Stride};
use {
    crate::domain::operator::contract::resources::DfsResources,
    crate::domain::operator::contract::
    crate::domain::operator::contract::views::PredicateProgram,
    crate::domain::stored_graph::types::GraphSource,
    crate::domain::stored_selection::types::Selector, crate::domain::table_cells::types::TreeRows,
    crate::domain::table_texts::types::Texts, ryo_values::domain::files::types::SourceView,
};
use crate::domain::table_model::types::representation_input::value::RepresentationInput as BorrowedRows;

#[derive(Clone, Debug)]
pub(crate) enum ViewRequest {
    RawKeyOrder {
        stride: Stride,
    },
    Equality {
        columns: Vec<ColumnIndex>,
    },
    PredicateMatches(Arc<Selector>),
    CandidatePostings {
        column: ColumnIndex,
        literals: Vec<LiteralText>,
    },
    DistinctRepresentatives {
        columns: Vec<ColumnIndex>,
    },
    Adjacency {
        endpoints: [ColumnIndex; 2],
        keys: Vec<ColumnIndex>,
    },
    Membership {
        column: ColumnIndex,
    },
    PartitionOrder {
        keys: Vec<ColumnIndex>,
    },
    ClosureAdjacency {
        source: ColumnIndex,
        destination: ColumnIndex,
    },
    CycleAdjacency {
        source: ColumnIndex,
        destination: ColumnIndex,
    },
    OrderedKeys {
        left_on: Vec<ColumnIndex>,
        right_on: Vec<ColumnIndex>,
        left_order: ColumnIndex,
        right_order: ColumnIndex,
    },
    OrderedDfs(DfsResources),
    NativeLookup {
        left_on: Vec<ColumnIndex>,
        right_on: Vec<ColumnIndex>,
        view: Vec<ColumnIndex>,
    },
}
/// The hash of a request's identity, prepared once.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct IdentityHash(pub u64);

pub(crate) mod contract;
use contract::value::DerivedContract;

/// Membership semantics can address values, existing dense graph vertices or a native tree.
pub(crate) enum MembershipSpace<'a> {
    Values,
    Dense(&'a [Value]),
    Native(TreeRows<'a>),
}
/// Inputs are semantic capabilities only; none can schedule upstream work or select a cache policy.
pub(crate) enum DerivedInputs<'a, 's> {
    RawWords(&'a Keys),
    Postings {
        rows: BorrowedRows<'a>,
        texts: &'a Texts<'a>,
        active: &'a [LiteralText],
    },
    Partition {
        rows: BorrowedRows<'a>,
        positions: &'a [RowIndex],
        key: &'a [Value],
    },
    Rows(&'a [BorrowedRows<'a>]),
    Predicate {
        rows: BorrowedRows<'a>,
        candidates: Option<&'a [u32]>,
        program: &'a mut dyn PredicateProgram,
        symbols: &'a Symbols,
    },
    Distinct {
        rows: BorrowedRows<'a>,
        positions: &'a [u32],
    },
    Text {
        rows: &'a [BorrowedRows<'a>],
        texts: &'a Texts<'a>,
    },
    Adjacency {
        rows: BorrowedRows<'a>,
        source: GraphSource,
    },
    Membership {
        rows: BorrowedRows<'a>,
        space: MembershipSpace<'a>,
    },
    Dfs {
        roots: BorrowedRows<'a>,
        edges: BorrowedRows<'a>,
        symbols: &'a mut QuerySymbols<'s>,
        source: SourceView<'a>,
        paths: &'a [FilePath],
        arenas: &'a [ArenaBytes],
    },
}
#[derive(Default)]
pub(crate) struct DerivedScratch {
    pub semantic: ryo_derived::domain::contract::types::BuildWorkspace,
    pub row: Vec<Value>,
}
/// How many of a prepared use's input places the view reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InputsUsed(pub u8);

/// Request-owned prepared use; population slots are immutable dependency metadata, not a scheduler.
#[derive(Clone, Debug)]
pub(crate) struct PreparedViewUse {
    pub plan: Arc<DerivedContract>,
    pub inputs: [InputNumber; 2],
    pub relations: [Option<ryo_relations::domain::relations::types::Relation>; 2],
    pub len: InputsUsed,
}
#[derive(Clone, Debug)]
pub(crate) struct PreparedViewGroup {
    pub primary: Arc<PreparedViewUse>,
    pub secondary: Option<Arc<PreparedViewUse>>,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct DerivedSlots {
    pub slots: [Option<Arc<PreparedViewGroup>>; DerivedKind::ALL.len()],
}

use ryo_values::domain::rows::types::RowIndex;

use ryo_derived::domain::contract::types::DerivedKind;

/// The address of a selector, which names it for the whole request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct SelectorAddress(pub usize);

/// Which group of equal selectors a selector belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ViewGroup(pub usize);

/// How many groups of equal selectors a request has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ViewGroupCount(pub usize);

/// Prepared construction contracts indexed by exact request-owned selector identity.
#[derive(Debug, Default)]
pub(crate) struct PreparedSelectionViews {
    /// Pins every selector address for the complete request lifetime.
    pub selectors: Vec<Arc<Selector>>,
    /// O(1) capability lookup; no metadata construction occurs while selecting.
    pub by_selector: std::collections::HashMap<SelectorAddress, Arc<PreparedSelectorViews>>,
    pub groups: ViewGroupCount,
}
#[derive(Debug)]
pub(crate) struct PreparedSelectorViews {
    pub group: ViewGroup,
    pub predicate: Arc<DerivedContract>,
    pub postings: Arc<DerivedContract>,
    pub distinct: Arc<DerivedContract>,
    /// Every statically possible equality discriminator, chosen without runtime preparation.
    pub equalities: std::collections::HashMap<ColumnIndex, Arc<DerivedContract>>,
}
