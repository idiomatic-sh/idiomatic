//! The types of the `request` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use ryo_values::domain::plan_parts::types::ColumnIndex;

macro_rules! kinds {
    ($($kind:ident),+ $(,)?) => {
        /// Adding a kind changes the authoritative inventory and exhaustive dispatch.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum DerivedKind { $($kind),+ }
        impl DerivedKind {
            /// Every kind, in declaration order.
            pub const ALL: &'static [Self] = &[$(Self::$kind),+];
            /// The place of the kind in declaration order.
            #[must_use]
            pub const fn ordinal(self) -> usize {
                match self { $(Self::$kind => Self::$kind as usize),+ }
            }
        }
    };
}
kinds!(
    Equality,
    Adjacency,
    Membership,
    PartitionOrder,
    ClosureAdjacency,
    CycleAdjacency,
    OrderedKeys,
    OrderedDfs,
    NativeLookup
);

/// Static requirements contain semantics only; population/generation and binding identities stay in the engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DerivedRequest {
    /// A hash-chain index of the keys of the rows.
    Equality {
        /// The key columns.
        columns: Vec<ColumnIndex>,
    },
    /// A compact adjacency over two endpoint columns.
    Adjacency {
        /// The source and destination columns.
        endpoints: [ColumnIndex; 2],
        /// The partition key columns.
        keys: Vec<ColumnIndex>,
    },
    /// The members of a column.
    Membership {
        /// The member column.
        column: ColumnIndex,
    },
    /// The typed partition order of the rows.
    PartitionOrder {
        /// The key columns.
        keys: Vec<ColumnIndex>,
    },
    /// One partition's closure adjacency.
    ClosureAdjacency {
        /// The source column.
        source: ColumnIndex,
        /// The destination column.
        destination: ColumnIndex,
    },
    /// A dense cycle adjacency.
    CycleAdjacency {
        /// The source column.
        source: ColumnIndex,
        /// The destination column.
        destination: ColumnIndex,
    },
    /// Two ordered sides matched by key.
    OrderedKeys {
        /// The left key columns.
        left_on: Vec<ColumnIndex>,
        /// The right key columns.
        right_on: Vec<ColumnIndex>,
        /// The left order column.
        left_order: ColumnIndex,
        /// The right order column.
        right_order: ColumnIndex,
    },
    /// Roots and ordered edges prepared for a depth-first traversal.
    OrderedDfs {
        /// The root column.
        root: ColumnIndex,
        /// The root key columns.
        root_keys: Vec<ColumnIndex>,
        /// The edge source column.
        source: ColumnIndex,
        /// The edge destination column.
        destination: ColumnIndex,
        /// The edge order column.
        order: ColumnIndex,
        /// The edge identity column.
        edge: ColumnIndex,
        /// The edge key columns.
        edge_keys: Vec<ColumnIndex>,
    },
    /// A static native lookup shape.
    NativeLookup {
        /// The left key columns.
        left_on: Vec<ColumnIndex>,
        /// The right key columns.
        right_on: Vec<ColumnIndex>,
        /// The view columns.
        view: Vec<ColumnIndex>,
    },
}
