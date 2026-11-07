//! Closed typed requirements, construction storage and immutable borrowed access.
use ryo_language::domain::{
    files::types::FilePath,
    symbols::types::QuerySymbols,
};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::{
    plan_parts::types::ColumnIndex,
    stored_facts::types::ArenaBytes,
};

use {
    crate::domain::operator::contract::resources::DfsResources,
    crate::domain::stored_graph::types::GraphSource,
    crate::domain::table_cells::types::TreeRows,
    crate::domain::table_texts::types::Texts, ryo_values::domain::files::types::SourceView,
};
use crate::domain::table_model::types::RepresentationInput as BorrowedRows;

#[derive(Clone, Debug)]
pub(crate) enum ViewRequest {
    Equality {
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

/// Membership semantics can address values, existing dense graph vertices or a native tree.
pub(crate) enum MembershipSpace<'a> {
    Values,
    Dense(&'a [Value]),
    Native(TreeRows<'a>),
}
/// Inputs are semantic capabilities only; none can schedule upstream work or select a cache policy.
pub(crate) enum DerivedInputs<'a, 's> {
    Partition {
        rows: BorrowedRows<'a>,
        positions: &'a [RowIndex],
        key: &'a [Value],
    },
    Rows(&'a [BorrowedRows<'a>]),
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

use ryo_values::domain::rows::types::RowIndex;

