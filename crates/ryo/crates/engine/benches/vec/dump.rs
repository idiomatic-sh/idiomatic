//! THROWAWAY (ryo-fmx3.1, one pass): `--dump <file>` on `vec funnel`: per file, one hash per part of what the first half
//! reads, so two binaries are compared row for row (`build/redo-onepass/scripts/equal.sh`). Each line is
//! `<file number>\t<part>\t<rows>\t<hash>`; a part's hash is over the `Debug` text of every row, so any changed cell, order
//! or count changes it. Parts: the node table through every reader of its seam, per element and per kind; the structure
//! columns; and the word index.

use ryo::languages::{KindBits, NodeTable, Position, node_table};
use std::fmt::Write as _;
use std::hash::Hasher as _;

/// A hasher written through `fmt::Write`, so a `Debug` text is hashed without being kept.
pub(super) struct Hashed(std::hash::DefaultHasher, pub(super) u64);

impl Default for Hashed {
    fn default() -> Self {
        Self(std::hash::DefaultHasher::new(), 0)
    }
}

impl std::fmt::Write for Hashed {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.write(s.as_bytes());
        Ok(())
    }
}

impl Hashed {
    /// One row.
    pub(super) fn row(&mut self, row: &dyn std::fmt::Debug) {
        let _ = write!(self, "{row:?};");
        self.1 += 1;
    }

    /// Every element of a column, each a row.
    pub(super) fn rows<T: std::fmt::Debug>(&mut self, column: &[T]) {
        for each in column {
            self.row(each);
        }
    }

    /// The finished line of `part` for file `file`.
    pub(super) fn line(self, out: &mut String, file: usize, part: &str) {
        let _ = writeln!(out, "{file}\t{part}\t{}\t{:016x}", self.1, self.0.finish());
    }
}

/// The structure the checks read, one row an element: parent kind, child-kind set, child-field set (codes below 128) and
/// whether it is a root; then the enclosing column of each kind set: the node table's own columns.
pub(super) fn structure(out: &mut String, file: usize, t: &NodeTable) {
    let mut h = Hashed::default();
    h.rows(node_table::parent_ids(t));
    for x in node_table::element_sets(t) {
        h.row(&(Some(&x.parent_kind), Some(&x.child_kinds), x.child_fields, x.root));
    }
    for col in node_table::enclosing_ids(t) {
        h.rows(col);
    }
    h.line(out, file, "structure");
}

/// The node table, through its readers: per element, then per kind, then the file's own facts.
pub(super) fn table(out: &mut String, file: usize, t: &NodeTable, kinds: usize) {
    let mut h = Hashed::default();
    let n = node_table::element_count(t).0 as u32;
    for id in 0..n {
        let p = Position(id);
        h.row(&(
            id,
            node_table::kind_of(t, p),
            node_table::parent_of(t, p),
            node_table::subtree_end_of(t, p),
            node_table::span_of(t, p),
            node_table::field_of(t, p),
            node_table::ordinal_of(t, p),
            node_table::children_of(t, p),
            node_table::decoded_of(t, p),
            node_table::implied_of(t, p),
        ));
    }
    h.line(out, file, "table/elements");
    let mut h = Hashed::default();
    for kind in 0..kinds.max(1) {
        h.row(&(kind, node_table::ids_of_kind(t, KindBits(kind as u16))));
    }
    h.row(node_table::kinds_present(t));
    h.line(out, file, "table/kinds");
    let mut h = Hashed::default();
    h.row(&(
        node_table::node_count(t),
        node_table::element_count(t),
        node_table::own_end(t),
        node_table::syntax_error_count(t),
        node_table::acceptance_of(t),
    ));
    h.rows(&node_table::embedded_segments(t).collect::<Vec<_>>());
    h.line(out, file, "table/file");
}
