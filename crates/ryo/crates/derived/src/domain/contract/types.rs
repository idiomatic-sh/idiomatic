//! The complete derived family, independent of scheduler and cache policy.
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};
use std::sync::Arc;

macro_rules! kinds {
    ($($kind:ident),+ $(,)?) => {
        /// Adding a kind changes the authoritative inventory and exhaustive dispatch.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum DerivedKind { $($kind),+ }
        impl DerivedKind {
            pub const ALL: &'static [Self] = &[$(Self::$kind),+];
            #[must_use]
            pub const fn ordinal(self) -> usize {
                match self { $(Self::$kind => Self::$kind as usize),+ }
            }
        }
    };
}
kinds!(
    KeyIndex,
    PredicateMatches,
    CandidatePostings,
    DistinctRepresentatives,
    Adjacency,
    Membership,
    PartitionOrder,
    ClosureAdjacency,
    CycleAdjacency,
    OrderedKeys,
    OrderedDfs,
    NativeLookup,
    RawKeyOrder
);

/// Portable failures; an interrupted engine callback retains its exact error outside this crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DerivedFault {
    MissingInput { position: usize },
    NoSuchColumn { column: ColumnIndex },
    DuplicateOrder,
    DuplicateDfsOrder,
    ShapeMismatch,
    NotRun,
    WrongOperation,
    Interrupted,
}

/// Immutable shared positions; its allocation identity never depends on liveness.
#[derive(Clone, Debug)]
pub struct SharedPositions(pub(crate) Arc<Vec<u32>>);
/// Weak sharing observes an allocation without retaining row IDs or mutable map equality.
#[derive(Clone, Debug)]
pub struct WeakPositions {
    allocation_key: usize,
    capacity: usize,
    rows: std::sync::Weak<Vec<u32>>,
}
impl WeakPositions {
    #[must_use]
    pub fn allocation_key(&self) -> usize {
        self.allocation_key
    }
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
    #[must_use]
    pub fn upgrade(&self) -> Option<SharedPositions> {
        self.rows.upgrade().map(SharedPositions)
    }
}
impl SharedPositions {
    #[must_use]
    pub fn as_slice(&self) -> &[u32] {
        self.0.as_slice()
    }
    #[must_use]
    pub fn allocation_key(&self) -> usize {
        Arc::as_ptr(&self.0) as usize
    }
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }
    #[must_use]
    pub fn same_allocation(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    #[must_use]
    pub fn downgrade(&self) -> WeakPositions {
        WeakPositions {
            allocation_key: self.allocation_key(),
            capacity: self.capacity(),
            rows: Arc::downgrade(&self.0),
        }
    }
}

/// Raw-word ordering deliberately differs from typed, cluster-preserving PartitionOrder.
#[derive(Clone, Debug)]
pub struct RawOrder {
    pub(crate) stride: usize,
    pub(crate) keys: Vec<u64>,
    pub(crate) order: Vec<usize>,
}
impl RawOrder {
    #[must_use]
    pub fn hits(&self, probe: &[u64]) -> std::ops::Range<usize> {
        crate::domain::raw_order::hits(self, probe)
    }
    pub fn firsts(&self, keep: &mut Vec<bool>) {
        crate::domain::raw_order::firsts(self, keep);
    }

    #[must_use]
    pub fn rows(&self) -> &[usize] {
        &self.order
    }
    #[must_use]
    pub fn capacities(&self) -> (usize, usize) {
        (self.keys.capacity(), self.order.capacity())
    }
}

/// A finite already-admitted row view. Reading a cell cannot schedule upstream work.
#[derive(Clone, Copy)]
pub enum BulkRows<'a> {
    Stored(ryo_relations::domain::relations::types::ColumnsView<'a>),
    Selected {
        source: ryo_relations::domain::relations::types::ColumnsView<'a>,
        positions: &'a [u32],
        columns: &'a [ColumnIndex],
    },
    Computed(&'a Table),
}
pub trait RowCells {
    /// Decide once per build whether physical bulk decoding is available.
    fn bulk(&self) -> Option<BulkRows<'_>>;

    fn read_row<'a>(
        &self,
        row: usize,
        buffer: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
    ) -> Option<&'a [ryo_relations::domain::relations::types::Value]> {
        if row >= self.count() {
            return None;
        }
        buffer.clear();
        for column in 0..self.kinds().len() {
            buffer.push(self.cell(row, ColumnIndex(u32::try_from(column).ok()?))?);
        }
        Some(buffer.as_slice())
    }
    fn validate(&self) -> Result<(), DerivedFault>;
    fn kinds(&self) -> &[ryo_relations::domain::relations::types::CellKind];
    fn count(&self) -> usize;
    fn cell(
        &self,
        row: usize,
        column: ColumnIndex,
    ) -> Option<ryo_relations::domain::relations::types::Value>;
}
/// Plain row/index data models; only DerivedPayload certifies a successful semantic build.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HashWord(pub u64);
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyCellsHash(pub u64);
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChainLink(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct HashedRow {
    pub hash: KeyCellsHash,
    pub row: RowIndex,
}
#[derive(Debug, Default)]
pub struct KeyIndex {
    pub(crate) rows: Vec<RowIndex>,
    pub(crate) hashes: Vec<KeyCellsHash>,
    pub(crate) heads: Vec<ChainLink>,
    pub(crate) next: Vec<ChainLink>,
    pub(crate) sorted: Vec<RowIndex>,
    pub(crate) pairs: Vec<HashedRow>,
    pub(crate) runs: hashbrown::HashSet<KeyCellsHash>,
}
/// A finite predicate decision; the engine owns any sticky interrupted error.
pub trait PredicateRows {
    fn count(&self) -> usize;
    fn holds(&mut self, row: usize) -> Result<bool, DerivedFault>;
}
#[derive(Debug)]
pub struct LiteralPostings {
    pub(crate) text: String,
    pub(crate) rows: SharedPositions,
}
impl LiteralPostings {
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
    #[must_use]
    pub fn rows(&self) -> &[u32] {
        self.rows.as_slice()
    }
}

/// Static requirements contain semantics only; population/generation and binding identities stay in the engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewRequest {
    Equality {
        columns: Vec<ColumnIndex>,
    },
    PredicateMatches {
        needed: Vec<ColumnIndex>,
    },
    CandidatePostings {
        column: ColumnIndex,
        literals: Vec<ryo_values::domain::plan_parts::types::LiteralText>,
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
    OrderedDfs {
        root: ColumnIndex,
        root_keys: Vec<ColumnIndex>,
        source: ColumnIndex,
        destination: ColumnIndex,
        order: ColumnIndex,
        edge: ColumnIndex,
        edge_keys: Vec<ColumnIndex>,
    },
    NativeLookup {
        left_on: Vec<ColumnIndex>,
        right_on: Vec<ColumnIndex>,
        view: Vec<ColumnIndex>,
    },
    RawKeyOrder {
        stride: usize,
    },
}
/// Preparation stores metadata failures until the original consumer demand gate.
#[derive(Clone, Debug)]
pub struct PreparedDerived {
    pub(crate) request: ViewRequest,
    pub(crate) schemas: Vec<Vec<ryo_relations::domain::relations::types::CellKind>>,
    pub(crate) metadata: Result<(), DerivedFault>,
}
impl PreparedDerived {
    #[must_use]
    pub fn request(&self) -> &ViewRequest {
        &self.request
    }
    #[must_use]
    pub fn schemas(&self) -> &[Vec<ryo_relations::domain::relations::types::CellKind>] {
        &self.schemas
    }
    pub fn validate(&self) -> Result<(), DerivedFault> {
        self.metadata.clone()
    }
}

/// Adjacency carries semantic data only; exact population/generation identity is held by the engine.
#[derive(Debug)]
pub struct Adjacency {
    pub(crate) partitions: Vec<Arc<[ryo_relations::domain::relations::types::Value]>>,
    pub(crate) partition_ids:
        hashbrown::HashMap<Arc<[ryo_relations::domain::relations::types::Value]>, u32>,
    pub(crate) vertices: Vec<ryo_relations::domain::relations::types::Value>,
    pub(crate) vertex_partitions: Vec<u32>,
    pub(crate) vertex_ids:
        hashbrown::HashMap<(u32, ryo_relations::domain::relations::types::Value), u32>,
    pub(crate) offsets: [Vec<u32>; 2],
    pub(crate) neighbors: [Vec<u32>; 2],
    pub(crate) forests: [Option<Vec<u32>>; 2],
}
impl Adjacency {
    #[must_use]
    pub fn partition(
        &self,
        tuple: &[ryo_relations::domain::relations::types::Value],
    ) -> Option<u32> {
        self.partition_ids.get(tuple).copied()
    }
    #[must_use]
    pub fn partition_values(
        &self,
        id: usize,
    ) -> Option<&[ryo_relations::domain::relations::types::Value]> {
        self.partitions.get(id).map(|values| values.as_ref())
    }
    #[must_use]
    pub fn vertices(&self) -> &[ryo_relations::domain::relations::types::Value] {
        &self.vertices
    }
    #[must_use]
    pub fn vertex_partitions(&self) -> &[u32] {
        &self.vertex_partitions
    }
    #[must_use]
    pub fn vertex(
        &self,
        partition: u32,
        value: ryo_relations::domain::relations::types::Value,
    ) -> Option<u32> {
        self.vertex_ids.get(&(partition, value)).copied()
    }
    #[must_use]
    pub fn offsets(&self) -> &[Vec<u32>; 2] {
        &self.offsets
    }
    #[must_use]
    pub fn neighbors(&self) -> &[Vec<u32>; 2] {
        &self.neighbors
    }
    #[must_use]
    pub fn forests(&self) -> &[Option<Vec<u32>>; 2] {
        &self.forests
    }
}

#[derive(Clone, Debug)]
pub struct SharedMarks(pub(crate) Arc<Vec<bool>>);
impl SharedMarks {
    #[must_use]
    pub fn as_slice(&self) -> &[bool] {
        &self.0
    }
    #[must_use]
    pub fn allocation_key(&self) -> usize {
        Arc::as_ptr(&self.0) as usize
    }
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }
}
pub enum MembershipSpace<'a> {
    Values,
    Dense(&'a [ryo_relations::domain::relations::types::Value]),
    Native {
        file: ryo_relations::domain::relations::types::FileId,
        elements: usize,
    },
}
#[derive(Debug)]
pub enum Membership {
    Values(hashbrown::HashSet<ryo_relations::domain::relations::types::Value>),
    Dense(SharedMarks),
}

#[derive(Debug, Default)]
pub struct CycleGraph {
    pub(crate) values: Vec<ryo_relations::domain::relations::types::Value>,
    pub(crate) starts: Vec<usize>,
    pub(crate) leaving: Vec<usize>,
}
impl CycleGraph {
    #[must_use]
    pub fn values(&self) -> &[ryo_relations::domain::relations::types::Value] {
        &self.values
    }
    #[must_use]
    pub fn starts(&self) -> &[usize] {
        &self.starts
    }
    #[must_use]
    pub fn leaving(&self) -> &[usize] {
        &self.leaving
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeForm {
    Elements,
    Edges,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeFinding {
    Node(ColumnIndex, ColumnIndex),
    Stretch(ColumnIndex, ColumnIndex),
    Span(ColumnIndex),
}

impl KeyIndex {
    #[must_use]
    pub fn rows(&self) -> &[RowIndex] {
        &self.rows
    }
    #[must_use]
    pub fn hashes(&self) -> &[KeyCellsHash] {
        &self.hashes
    }
    #[must_use]
    pub fn heads(&self) -> &[ChainLink] {
        &self.heads
    }
    #[must_use]
    pub fn links(&self) -> &[ChainLink] {
        &self.next
    }
    #[must_use]
    pub fn sorted(&self) -> &[RowIndex] {
        &self.sorted
    }
    #[must_use]
    pub fn capacities(&self) -> [usize; 7] {
        [
            self.rows.capacity(),
            self.hashes.capacity(),
            self.heads.capacity(),
            self.next.capacity(),
            self.sorted.capacity(),
            self.pairs.capacity(),
            self.runs.capacity(),
        ]
    }
}
impl RowCells for Table {
    fn bulk(&self) -> Option<BulkRows<'_>> {
        Some(BulkRows::Computed(self))
    }
    fn validate(&self) -> Result<(), DerivedFault> {
        if self.kinds.len().checked_mul(self.count.0) != Some(self.cells.len()) {
            return Err(DerivedFault::ShapeMismatch);
        }
        Ok(())
    }
    fn kinds(&self) -> &[ryo_relations::domain::relations::types::CellKind] {
        &self.kinds
    }
    fn count(&self) -> usize {
        self.count.0
    }
    fn cell(
        &self,
        row: usize,
        column: ColumnIndex,
    ) -> Option<ryo_relations::domain::relations::types::Value> {
        if row >= self.count.0 {
            return None;
        }
        let column = usize::try_from(column.0).ok()?;
        if column >= self.kinds.len() {
            return None;
        }
        self.cells
            .get(row.checked_mul(self.kinds.len())?.checked_add(column)?)
            .copied()
    }
}
#[derive(Debug, Default)]
pub struct ClosurePartition {
    pub(crate) key: Vec<ryo_relations::domain::relations::types::Value>,
    pub(crate) table: Table,
    pub(crate) index: KeyIndex,
    pub(crate) sources: Vec<ryo_relations::domain::relations::types::Value>,
}
impl ClosurePartition {
    #[must_use]
    pub fn key(&self) -> &[ryo_relations::domain::relations::types::Value] {
        &self.key
    }
    #[must_use]
    pub fn table(&self) -> &Table {
        &self.table
    }
    #[must_use]
    pub fn index(&self) -> &KeyIndex {
        &self.index
    }
    #[must_use]
    pub fn sources(&self) -> &[ryo_relations::domain::relations::types::Value] {
        &self.sources
    }
    #[must_use]
    pub fn capacities(&self) -> (usize, usize) {
        (self.key.capacity(), self.sources.capacity())
    }
}

/// Finite textual decoding only; cannot schedule or retain a population.
pub trait ValueTexts {
    fn text(&self, value: ryo_relations::domain::relations::types::Value) -> Option<&str>;
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OrderedKey {
    Text(String),
    Value(ryo_relations::domain::relations::types::Value),
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OrderedRow {
    pub(crate) keys: Vec<OrderedKey>,
    pub(crate) order: ryo_relations::domain::relations::types::Integer,
    pub(crate) row: usize,
}
impl OrderedRow {
    #[must_use]
    pub fn keys(&self) -> &[OrderedKey] {
        &self.keys
    }
    #[must_use]
    pub fn order(&self) -> ryo_relations::domain::relations::types::Integer {
        self.order
    }
    #[must_use]
    pub fn row(&self) -> usize {
        self.row
    }
}
#[derive(Debug)]
pub struct OrderedMatching {
    pub(crate) left: Vec<OrderedRow>,
    pub(crate) right: Vec<OrderedRow>,
}
impl OrderedMatching {
    #[must_use]
    pub fn left(&self) -> &[OrderedRow] {
        &self.left
    }
    #[must_use]
    pub fn right(&self) -> &[OrderedRow] {
        &self.right
    }
}

/// Source text access and query-symbol storage are finite data services, not acquisition policy.
pub trait CanonicalTexts: ValueTexts {
    fn intern(&mut self, text: &str) -> ryo_relations::domain::relations::types::Value;
}
#[derive(Debug)]
pub struct PreparedDfs {
    pub(crate) roots: Table,
    pub(crate) edges: Table,
    pub(crate) root_index: KeyIndex,
    pub(crate) edge_index: KeyIndex,
    pub(crate) root_order: Vec<usize>,
    pub(crate) edge_order: Vec<usize>,
    pub(crate) root_columns: Vec<ColumnIndex>,
    pub(crate) edge_columns: Vec<ColumnIndex>,
    pub(crate) adjacency: Vec<std::ops::Range<usize>>,
}
impl PreparedDfs {
    #[must_use]
    pub fn roots(&self) -> &Table {
        &self.roots
    }
    #[must_use]
    pub fn edges(&self) -> &Table {
        &self.edges
    }
    #[must_use]
    pub fn root_index(&self) -> &KeyIndex {
        &self.root_index
    }
    #[must_use]
    pub fn edge_index(&self) -> &KeyIndex {
        &self.edge_index
    }
    #[must_use]
    pub fn root_order(&self) -> &[usize] {
        &self.root_order
    }
    #[must_use]
    pub fn edge_order(&self) -> &[usize] {
        &self.edge_order
    }
    #[must_use]
    pub fn root_columns(&self) -> &[ColumnIndex] {
        &self.root_columns
    }
    #[must_use]
    pub fn edge_columns(&self) -> &[ColumnIndex] {
        &self.edge_columns
    }
    #[must_use]
    pub fn adjacency(&self) -> &[std::ops::Range<usize>] {
        &self.adjacency
    }
}

/// Successful payload construction is private to the capability's build dispatcher.
#[derive(Debug)]
pub struct DerivedPayload {
    pub(crate) data: PayloadData,
}
#[derive(Debug)]
pub(crate) enum PayloadData {
    Equality(Arc<KeyIndex>),
    PredicateMatches(SharedPositions),
    CandidatePostings(Vec<LiteralPostings>),
    DistinctRepresentatives(SharedPositions),
    Adjacency(Arc<Adjacency>),
    Membership(Membership),
    PartitionOrder(Vec<usize>),
    ClosureAdjacency(Arc<ClosurePartition>),
    CycleAdjacency(Arc<CycleGraph>),
    OrderedKeys(Arc<OrderedMatching>),
    OrderedDfs(Arc<PreparedDfs>),
    NativeLookup(NativeFinding),
    RawKeyOrder(Arc<RawOrder>),
}
/// Unsupported readable native/oversized layouts decline distinctly from faults.
#[derive(Debug)]
pub enum BuildOutcome {
    Built(DerivedPayload),
    Declined,
}
/// Borrowing an existing payload cannot build, acquire inputs, or mutate its storage.
pub enum DerivedView<'a> {
    Equality(&'a KeyIndex),
    RowIds(&'a [u32]),
    Postings(&'a [LiteralPostings]),
    Adjacency(&'a Adjacency),
    Membership(MembershipView<'a>),
    PartitionOrder(&'a [usize]),
    ClosureAdjacency(&'a ClosurePartition),
    CycleAdjacency(&'a CycleGraph),
    OrderedKeys(&'a OrderedMatching),
    OrderedDfs(&'a PreparedDfs),
    NativeLookup(&'a NativeFinding),
    RawKeyOrder(&'a RawOrder),
}
pub enum DerivedInputs<'a> {
    Rows(&'a dyn RowCells),
    Predicate {
        rows: &'a mut dyn PredicateRows,
        candidates: Option<&'a [u32]>,
    },
    Postings {
        rows: &'a dyn RowCells,
        symbols: &'a ryo_language::domain::symbols::types::Symbols,
        source: ryo_language::domain::symbols::types::ByteSlice<'a>,
        active: &'a [ryo_values::domain::plan_parts::types::LiteralText],
    },
    Distinct {
        rows: &'a dyn RowCells,
        positions: &'a [u32],
    },
    Membership {
        rows: &'a dyn RowCells,
        space: MembershipSpace<'a>,
    },
    Partition {
        rows: &'a dyn RowCells,
        positions: Option<&'a [usize]>,
        key: &'a [ryo_relations::domain::relations::types::Value],
    },
    Ordered {
        left: &'a dyn RowCells,
        right: &'a dyn RowCells,
        texts: &'a dyn ValueTexts,
    },
    Dfs {
        roots: &'a dyn RowCells,
        edges: &'a dyn RowCells,
        texts: &'a mut dyn CanonicalTexts,
    },
    Native(NativeForm),
    RawWords(&'a [u64]),
}
#[derive(Default)]
pub struct BuildWorkspace {
    pub(crate) row: Vec<ryo_relations::domain::relations::types::Value>,
}

/// Physical immutable sub-allocation description, without any lease/admission class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SharedBacking {
    pub tag: u8,
    pub allocation_key: usize,
    pub bytes: usize,
    pub position_capacity: usize,
}
#[derive(Debug)]
pub struct PayloadFootprint {
    pub owned_bytes: usize,
    pub shared: Vec<SharedBacking>,
}
impl DerivedPayload {
    pub fn footprint(&self) -> Result<PayloadFootprint, DerivedFault> {
        crate::domain::footprint::describe(self)
    }
}

/// Immutable borrowed membership never lends an owned sub-allocation handle to a primitive.
pub enum MembershipView<'a> {
    Values(&'a hashbrown::HashSet<ryo_relations::domain::relations::types::Value>),
    Dense(&'a [bool]),
}
impl DerivedPayload {
    #[must_use]
    pub fn share_positions(&self) -> Option<SharedPositions> {
        match &self.data {
            PayloadData::PredicateMatches(rows) | PayloadData::DistinctRepresentatives(rows) => {
                Some(rows.clone())
            }
            _ => None,
        }
    }
    #[must_use]
    pub fn share_posting_positions(&self, text: &str) -> Option<SharedPositions> {
        let PayloadData::CandidatePostings(values) = &self.data else {
            return None;
        };
        values
            .binary_search_by(|entry| entry.text.as_str().cmp(text))
            .ok()
            .and_then(|at| values.get(at))
            .map(|entry| entry.rows.clone())
    }
    #[must_use]
    pub fn share_marks(&self) -> Option<SharedMarks> {
        match &self.data {
            PayloadData::Membership(Membership::Dense(rows)) => Some(rows.clone()),
            _ => None,
        }
    }
    #[must_use]
    pub fn share_equality(&self) -> Option<Arc<KeyIndex>> {
        match &self.data {
            PayloadData::Equality(data) => Some(Arc::clone(data)),
            _ => None,
        }
    }
    #[must_use]
    pub fn share_adjacency(&self) -> Option<Arc<Adjacency>> {
        match &self.data {
            PayloadData::Adjacency(data) => Some(Arc::clone(data)),
            _ => None,
        }
    }
}
