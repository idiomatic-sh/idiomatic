//! The kinds of workload the fill bench times.

/// The fills the bench times, each one table made through the seam.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::made_tables::fixtures::benches::FILL_BENCHES;
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
    /// The index of the file's nodes by kind, built for a new file and asked for the tokens.
    IndexNodesKind,
    /// The index of the file's nodes by parent, built for a new file and asked for the root's children.
    IndexNodesParent,
    /// The index of the file's nodes by id, built for a new file and asked for one node.
    IndexNodesId,
}
