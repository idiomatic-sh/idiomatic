//! Closed typed requirements, construction storage and immutable borrowed access.
use ryo_language::domain::{
    files::types::FilePath,
    symbols::types::{QuerySymbols, Symbols},
};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, LiteralText},
    stored_facts::types::ArenaBytes,
};
use std::sync::Arc;

use crate::domain::positions::types::{Keys, Stride};
use {
    crate::domain::operator::contract::resources::DfsResources,
    crate::domain::operator::contract::
    crate::domain::operator::contract::views::PredicateProgram,
    crate::domain::stored_graph::types::GraphSource,
    crate::domain::stored_selection::types::{BorrowedPositions, Selector}, crate::domain::table_cells::types::TreeRows,
    crate::domain::table_texts::types::Texts, ryo_values::domain::files::types::SourceView,
};
use crate::domain::table_model::types::RepresentationInput as BorrowedRows;

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
        candidates: Option<BorrowedPositions<'a>>,
        program: &'a mut dyn PredicateProgram,
        symbols: &'a Symbols,
    },
    Distinct {
        rows: BorrowedRows<'a>,
        positions: BorrowedPositions<'a>,
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

use ryo_values::domain::rows::types::RowIndex;


/// The address of a selector, which names it for the whole request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct SelectorAddress(pub usize);

/// Which group of equal selectors a selector belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ViewGroup(pub usize);

/// How many groups of equal selectors a request has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ViewGroupCount(pub usize);
