//! Provider-owned keyed and per-node columns for the vector benchmark.

use crate::languages::Relation;

/// The engine's text ids: the provider writes every text cell as the id the engine gives the text.
pub trait TextIds {
    /// The id of `text`.
    fn id(&mut self, text: &[u8]) -> u32;
}

/// How a fact is laid out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Layout {
    /// One row at most a node: a column's value at a node is the row's, and `present` says which nodes have one.
    #[default]
    PerNode,
    /// Any number of rows a node: the rows of node `k` are `offsets[k]..offsets[k + 1]`.
    Keyed,
}

/// One fact asked of a file: a relation, the column its rows are found by, and whether only "has this node a row" is wanted.
#[derive(Clone, Copy, Debug)]
pub struct Ask {
    /// The relation.
    pub relation: Relation,
    /// The place, among the relation's catalog columns, of the node column its rows are found by.
    pub key: usize,
    /// Only the yes/no question: the presence bits, and no column.
    pub exists: bool,
    /// The catalog places of the columns wanted, one bit a place (`u64::MAX`: all): a column no test reads is not written.
    pub columns: u64,
}

/// One fact of one file, as the provider wrote it.
#[derive(Clone, Debug, Default)]
pub struct FactTable {
    // The answer writer consumes rows in insertion order. Its buffer needs
    // neither a node index nor space for values at nodes with no row.
    ordered_rows: bool,
    /// The layout.
    pub layout: Layout,
    /// Whether the provider answered the fact whole for this file (false: the file was not read whole, a superset holds).
    pub answered: bool,
    /// Per node, one bit: the node has a row (both layouts, so the yes/no question is one bit read).
    pub present: Vec<u64>,
    /// `Keyed`: per node that has rows (in node order), where its rows start in `order`; one more entry at the end.
    pub offsets: Vec<u32>,
    /// `Keyed`: per word of `present`, how many nodes before it have rows.
    pub rank: Vec<u32>,
    /// `Keyed`: the rows by key, as row numbers into the columns (empty: the rows came in key order, row `i` is `order[i]`).
    pub order: Vec<u32>,
    /// By catalog place, the values: by node (`PerNode`) or by row (`Keyed`); empty for a column not written.
    pub columns: Vec<Vec<u32>>,
    /// How many rows.
    pub rows: usize,
    /// `Keyed`, while writing: each row's key node.
    pub keys: Vec<u32>,
    /// The columns wanted, one bit a catalog place.
    wanted: u64,
}

impl FactTable {
    /// A buffer for sequential column consumption in provider row order.
    /// Indexed first-half reads use the default buffer.
    #[must_use]
    pub fn ordered() -> Self { Self { ordered_rows: true, ..Self::default() } }

    pub fn begin(&mut self, layout: Layout, width: usize, elements: usize, written: &[usize], exists: bool, wanted: u64) {
        self.wanted = wanted;
        self.layout = layout;
        self.answered = true;
        self.rows = 0;
        self.present.clear();
        if !self.ordered_rows { self.present.resize(elements.div_ceil(64).max(1), 0); }
        self.offsets.clear();
        self.order.clear();
        self.keys.clear();
        self.columns.resize_with(width.max(self.columns.len()), Vec::new);
        for column in &mut self.columns {
            column.clear();
        }
        if !self.ordered_rows && layout == Layout::PerNode && !exists {
            for place in written {
                if wanted >> place & 1 == 0 {
                    continue;
                }
                if let Some(column) = self.columns.get_mut(*place) {
                    column.resize(elements, 0);
                }
            }
        }
    }

    #[inline]
    pub fn wants(&self, place: usize) -> bool {
        self.wanted >> place & 1 == 1
    }

    #[inline]
    pub fn mark(&mut self, node: u32) {
        if let Some(word) = self.present.get_mut(node as usize / 64) {
            *word |= 1 << (node % 64);
        }
    }

    /// A per-node row: its values by catalog place.
    #[inline]
    pub fn put(&mut self, node: u32, values: &[(usize, u32)]) {
        if self.ordered_rows {
            self.push(node, values);
            return;
        }
        if !self.ordered_rows { self.mark(node); }
        self.keys.push(node);
        self.rows += 1;
        for (place, value) in values {
            if let Some(cell) = self.columns.get_mut(*place).and_then(|column| column.get_mut(node as usize)) {
                *cell = *value;
            }
        }
    }

    /// A keyed row.
    #[inline]
    pub fn push(&mut self, node: u32, values: &[(usize, u32)]) {
        if !self.ordered_rows { self.mark(node); }
        self.keys.push(node);
        self.rows += 1;
        for (place, value) in values {
            if self.wanted >> place & 1 == 1
                && let Some(column) = self.columns.get_mut(*place)
            {
                column.push(*value);
            }
        }
    }

    /// Ends a keyed table: the rows are put in key order by a sort of (key, row) pairs, O(rows log rows), and indexed by
    /// key through the presence bits and a rank a word, O(elements / 64): no pass over the file's elements.
    pub fn finish(&mut self, _elements: usize) {
        if self.ordered_rows || self.layout != Layout::Keyed {
            return;
        }
        self.rank.clear();
        let mut before = 0u32;
        for word in &self.present {
            self.rank.push(before);
            before += word.count_ones();
        }
        self.offsets.clear();
        self.order.clear();
        if self.keys.is_sorted() {
            let mut last = u32::MAX;
            for (row, key) in self.keys.iter().enumerate() {
                if *key != last {
                    self.offsets.push(row as u32);
                    last = *key;
                }
            }
        } else {
            let mut pairs: Vec<u64> = self.keys.iter().enumerate().map(|(row, key)| u64::from(*key) << 32 | row as u64).collect();
            pairs.sort_unstable();
            let mut last = u32::MAX;
            for (at, pair) in pairs.iter().enumerate() {
                let key = (pair >> 32) as u32;
                if key != last {
                    self.offsets.push(at as u32);
                    last = key;
                }
                self.order.push(*pair as u32);
            }
        }
        self.offsets.push(self.keys.len() as u32);
    }

    /// The rows of `node`, as places into `order` (or into the columns when `order` is empty); none when it has no row.
    #[inline]
    #[must_use]
    pub fn rows_of(&self, node: usize) -> std::ops::Range<usize> {
        let Some(word) = self.present.get(node / 64) else { return 0..0 };
        let bit = 1u64 << (node % 64);
        if word & bit == 0 {
            return 0..0;
        }
        if self.layout == Layout::PerNode {
            return node..node + 1;
        }
        let at = (self.rank[node / 64] + (word & (bit - 1)).count_ones()) as usize;
        self.offsets[at] as usize..self.offsets[at + 1] as usize
    }

    /// redo-fills pass 2: each row's key node, in the order the rows were written (both layouts; empty for a yes/no ask).
    #[must_use]
    pub fn keys(&self) -> &[u32] {
        &self.keys
    }

    /// The row number in the columns of a place `rows_of` gave.
    #[inline]
    #[must_use]
    pub fn row(&self, place: usize) -> usize {
        if self.layout == Layout::Keyed && !self.order.is_empty() { self.order[place] as usize } else { place }
    }
}

/// The answers to one file's asks, one table an ask in ask order; the buffers are kept from file to file.
#[derive(Clone, Debug, Default)]
pub struct FactColumns {
    /// The tables.
    pub tables: Vec<FactTable>,
}
