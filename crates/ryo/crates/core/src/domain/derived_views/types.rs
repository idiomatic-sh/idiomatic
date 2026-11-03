//! Closed typed requirements, construction storage and immutable borrowed access.
use crate::domain::{
    operator::contract::{resources::DfsResources, types::PredicateProgram, views::BorrowedRows},
    stored_graph::types::GraphSource,
    stored_selection::types::{LiteralPostings, Selector},
    table_cells::types::TreeRows,
    table_texts::types::{SourceView, Texts},
};
use ryo_derived::domain::contract::types::KeyIndex;
use ryo_derived::domain::contract::types::{
    ClosurePartition, CompactGraph, CycleGraph, Finding, OrderedMatching, PreparedDfs,
};
use ryo_language::domain::{
    files::types::FilePath,
    symbols::types::{QuerySymbols, Symbols},
};
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, LiteralText},
    stored_facts::types::ArenaBytes,
};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub(crate) enum ViewRequest {
    RawKeyOrder {
        stride: usize,
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
#[derive(Clone, Debug)]
pub(crate) struct DerivedContract {
    pub semantic: Arc<ryo_derived::domain::contract::types::PreparedDerived>,
    pub kind: DerivedKind,
    pub request: ViewRequest,
    pub schemas: Vec<Vec<CellKind>>,
    pub metadata: Result<(), crate::domain::operating::errors::OperatingError>,
    pub needed: crate::domain::table_cells::types::ColumnPlaces,
}
/// Membership semantics can address values, existing dense graph vertices or a native tree.
pub(crate) enum MembershipSpace<'a> {
    Values,
    Dense(&'a [Value]),
    Native(TreeRows<'a>),
}
/// Inputs are semantic capabilities only; none can schedule upstream work or select a cache policy.
pub(crate) enum DerivedInputs<'a, 's> {
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
/// Storage has no borrowed provider data. Only the application retains it or chooses its budget.
#[derive(Debug, Default)]
pub(crate) enum DerivedStorage {
    #[default]
    Unbuilt,
    Equality(KeyIndex),
    /// Existing engine-retained stored index, charged once by its file backend.
    EqualityShared(Arc<KeyIndex>),
    RowIds {
        kind: DerivedKind,
        rows: Arc<Vec<u32>>,
    },
    Postings(Arc<Vec<LiteralPostings>>),
    Adjacency(Arc<CompactGraph>),
    Membership(MembershipData),
    PartitionOrder(Vec<RowIndex>),
    ClosureAdjacency(ClosurePartition),
    CycleAdjacency(CycleGraph),
    OrderedKeys(OrderedMatching),
    OrderedDfs(PreparedDfs),
    NativeLookup(Finding),
}
#[derive(Debug)]
pub(crate) enum MembershipData {
    Values(hashbrown::HashSet<Value>),
    Dense(Arc<Vec<bool>>),
}
#[derive(Default)]
pub(crate) struct DerivedScratch {
    pub semantic: ryo_derived::domain::contract::types::BuildWorkspace,
    pub row: Vec<Value>,
}
/// No owned handle or writable storage is exposed by view().
pub(crate) enum DerivedView<'a> {
    Equality(&'a KeyIndex),
    RowIds(&'a [u32]),
    Postings(&'a [LiteralPostings]),
    Adjacency(&'a CompactGraph),
    Membership(&'a MembershipData),
    PartitionOrder(&'a [RowIndex]),
    ClosureAdjacency(&'a ClosurePartition),
    CycleAdjacency(&'a CycleGraph),
    OrderedKeys(&'a OrderedMatching),
    OrderedDfs(&'a PreparedDfs),
    NativeLookup(Finding),
}

/// Request-owned prepared use; population slots are immutable dependency metadata, not a scheduler.
#[derive(Clone, Debug)]
pub(crate) struct PreparedViewUse {
    pub plan: Arc<DerivedContract>,
    pub inputs: [usize; 2],
    pub relations: [Option<ryo_relations::domain::relations::types::Relation>; 2],
    pub len: u8,
}
#[derive(Clone, Debug)]
pub(crate) struct PreparedViewGroup {
    pub primary: Arc<PreparedViewUse>,
    pub secondary: Option<Arc<PreparedViewUse>>,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct DerivedSlots {
    pub streaming: [Option<Arc<PreparedViewGroup>>; DerivedKind::ALL.len()],
    pub collected: [Option<Arc<PreparedViewGroup>>; DerivedKind::ALL.len()],
}

use ryo_values::domain::rows::types::RowIndex;

use ryo_derived::domain::contract::types::DerivedKind;
