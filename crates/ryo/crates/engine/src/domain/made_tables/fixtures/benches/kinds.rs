//! The kinds of workload the fill bench times.

/// The fills the bench times, each one table made through the seam.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::made_tables::fixtures::benches::kinds::FILL_BENCHES;
/// use ryo_engine::domain::made_tables::fixtures::benches::kinds::FillBench;
///
/// assert!(FILL_BENCHES.contains(&FillBench::RowsProbe));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillBench {
    /// A rows fill: a scan of a relation, two reads, the write.
    RowsRead,
    /// A rows fill that keeps the rows a comparison holds for: a scan, two reads, a keep, the write.
    RowsKeep,
    /// A rows fill that probes the relation by a column of the batch: a scan, a read, a probe, a read, the write.
    RowsProbe,
    /// A rows fill that drops repeats: a scan, a read of a column of eight rows a cell, the write.
    RowsDropped,
    /// A counted grouping, eight rows a key, through its pipeline.
    GroupCount,
    /// A sorted table, eight rows a key, through its pipeline.
    Sorted,
    /// A rows fill with a `Use` of a pattern in call form, eight rows a key: the loop's stand-in answers one row a key.
    UseRows,
    /// A rows fill that walks to the children of every node.
    WalkChildren,
    /// A rows fill that walks to the ancestors of every token.
    WalkAncestors,
    /// A rows fill that reads a node row's kind and keeps one kind.
    NodeKeep,
    /// A rows fill with a `Call` of `SpanBounds` for every token: one row for one, two numbers.
    CallSpanBounds,
    /// A rows fill with a `Call` of `Replace` for every token's text: one row for one, a word of the file.
    CallReplace,
    /// A rows fill with a `Call` of `Capture` for every token's text: one way a text, two slots.
    CallCapture,
    /// A rows fill with a `Call` of `SourceMatches` for every group of seven tokens: seven occurrences a group.
    CallSourceMatches,
    /// An ordered match of the relation with itself, eight rows a key on each side.
    OrderedMatch,
    /// The rows at the greatest end of each key's rows, eight rows a key.
    GroupEnds,
    /// A rows fill with a `Product`: thirty-two rows each paired with every row of the relation, a read and a keep for
    /// each pair, one pair in each row's kept.
    Product,
    /// A rows fill whose row holds the file beside two columns read: a scan, two reads, the write of three columns.
    RowsFile,
    /// The same with repeats dropped (8 rows a key, 43 cells).
    RowsFileDropped,
    /// A rows fill that keeps the nodes whose kind's name matches any of 26 literal globs: a column of three different
    /// words, each a word of the run.
    KeepGlobFewWords,
    /// A rows fill, for a new file each run, that keeps the groups whose text matches none of 26 literal globs: a column
    /// in which every row holds its own word of the file.
    KeepGlobOwnWords,
    /// Every row of a made table's column read through the seam's `cell_of`, one call a read.
    SeamCellOf,
    /// The same column held once (`column_held`) and every row read through it (`cell_held`).
    SeamColumnHeld,
    /// The index of the file's nodes by kind, built for a new file and asked for the tokens.
    IndexNodesKind,
    /// The same over a file whose elements change kind from one to the next, sixteen kinds in no order.
    IndexNodesKindMixed,
    /// The index of the file's nodes by parent, built for a new file and asked for the root's children.
    IndexNodesParent,
    /// The index of the file's nodes by id, built for a new file and asked for one node.
    IndexNodesId,
}

/// How many rows the product's workload pairs with every row.
pub(crate) const PRODUCT_LEFT: usize = 32;

/// Every fill the bench times.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::made_tables::fixtures::benches::kinds::FILL_BENCHES;
///
/// assert_eq!(FILL_BENCHES.len(), 27);
/// ```
pub const FILL_BENCHES: &[FillBench] = &[
    FillBench::RowsRead,
    FillBench::RowsKeep,
    FillBench::RowsProbe,
    FillBench::RowsDropped,
    FillBench::GroupCount,
    FillBench::Sorted,
    FillBench::UseRows,
    FillBench::WalkChildren,
    FillBench::WalkAncestors,
    FillBench::NodeKeep,
    FillBench::CallSpanBounds,
    FillBench::CallReplace,
    FillBench::CallCapture,
    FillBench::CallSourceMatches,
    FillBench::OrderedMatch,
    FillBench::GroupEnds,
    FillBench::Product,
    FillBench::RowsFile,
    FillBench::RowsFileDropped,
    FillBench::KeepGlobFewWords,
    FillBench::KeepGlobOwnWords,
    FillBench::SeamCellOf,
    FillBench::SeamColumnHeld,
    FillBench::IndexNodesKind,
    FillBench::IndexNodesKindMixed,
    FillBench::IndexNodesParent,
    FillBench::IndexNodesId,
];

/// The name a fill's measure carries.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::made_tables::fixtures::benches::kinds::fill_name;
/// use ryo_engine::domain::made_tables::fixtures::benches::kinds::FillBench;
///
/// assert_eq!(fill_name(FillBench::RowsProbe), "rows_probe");
/// ```
#[must_use]
pub const fn fill_name(bench: FillBench) -> &'static str {
    match bench {
        FillBench::RowsRead => "rows_read",
        FillBench::RowsKeep => "rows_keep",
        FillBench::RowsProbe => "rows_probe",
        FillBench::RowsDropped => "rows_repeats_dropped",
        FillBench::GroupCount => "group_count",
        FillBench::Sorted => "sorted",
        FillBench::UseRows => "use_rows",
        FillBench::WalkChildren => "walk_children",
        FillBench::WalkAncestors => "walk_ancestors",
        FillBench::NodeKeep => "node_read_keep",
        FillBench::CallSpanBounds => "call_span_bounds",
        FillBench::CallReplace => "call_replace",
        FillBench::CallCapture => "call_capture",
        FillBench::CallSourceMatches => "call_source_matches",
        FillBench::OrderedMatch => "ordered_match",
        FillBench::GroupEnds => "group_ends",
        FillBench::Product => "product_pairs",
        FillBench::RowsFile => "rows_with_the_file",
        FillBench::RowsFileDropped => "rows_with_the_file_repeats_dropped",
        FillBench::KeepGlobFewWords => "keep_glob_few_words",
        FillBench::KeepGlobOwnWords => "keep_glob_own_words",
        FillBench::SeamCellOf => "seam_cell_of",
        FillBench::SeamColumnHeld => "seam_column_held",
        FillBench::IndexNodesKind => "index_nodes_by_kind",
        FillBench::IndexNodesKindMixed => "index_nodes_by_kind_mixed",
        FillBench::IndexNodesParent => "index_nodes_by_parent",
        FillBench::IndexNodesId => "index_nodes_by_id",
    }
}

/// What a fill's time over `rows` rows is divided by: the rows of its source, or for the product the pairs it makes
/// (thirty-two rows, or every row of fewer, each with every row).
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::made_tables::fixtures::benches::kinds::fill_units;
/// use ryo_engine::domain::made_tables::fixtures::benches::kinds::FillBench;
///
/// assert_eq!(fill_units(FillBench::RowsRead, 100), 100);
/// assert_eq!(fill_units(FillBench::Product, 100), 3_200);
/// ```
#[must_use]
pub const fn fill_units(bench: FillBench, rows: usize) -> usize {
    let rows = if rows == 0 { 1 } else { rows };
    match bench {
        FillBench::Product => {
            let left = if rows < PRODUCT_LEFT {
                rows
            } else {
                PRODUCT_LEFT
            };
            left.saturating_mul(rows)
        }
        FillBench::RowsRead
        | FillBench::RowsKeep
        | FillBench::RowsProbe
        | FillBench::RowsDropped
        | FillBench::GroupCount
        | FillBench::Sorted
        | FillBench::UseRows
        | FillBench::WalkChildren
        | FillBench::WalkAncestors
        | FillBench::NodeKeep
        | FillBench::CallSpanBounds
        | FillBench::CallReplace
        | FillBench::CallCapture
        | FillBench::CallSourceMatches
        | FillBench::OrderedMatch
        | FillBench::GroupEnds
        | FillBench::RowsFile
        | FillBench::RowsFileDropped
        | FillBench::KeepGlobFewWords
        | FillBench::KeepGlobOwnWords
        | FillBench::SeamCellOf
        | FillBench::SeamColumnHeld
        | FillBench::IndexNodesKind
        | FillBench::IndexNodesKindMixed
        | FillBench::IndexNodesParent
        | FillBench::IndexNodesId => rows,
    }
}
