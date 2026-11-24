//! Data shared by every program visiting one source version in one symbol dictionary.
use super::{Index, KindBits, NodeTable, Run, Tokens, node_table};
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

pub(crate) struct FileVersion<'a> {
    pub(crate) table: &'a NodeTable,
    pub(crate) src: &'a [u8],
    pub(crate) file: u32,
    pub(crate) n: usize,
    pub(crate) kind: &'a [u16],
    pub(crate) start: &'a [u32],
    pub(crate) end: &'a [u32],
    pub(crate) parent: &'a [u32],
    pub(super) field: &'a [u16],
    pub(super) text_memo: RefCell<Vec<u32>>,
    pub(super) present_fields: OnceCell<rustc_hash::FxHashSet<u32>>,
    pub(super) by_start: RefCell<Option<Index>>,
    pub(super) by_end: RefCell<Option<Index>>,
    pub(super) by_line: [RefCell<Option<Index>>; 2],
    pub(super) tokens: RefCell<Option<Rc<Tokens>>>,
    pub(super) facts: RefCell<Vec<Option<super::request::Delivered>>>,
    pub(super) supplied: rustc_hash::FxHashMap<String, Rc<super::Table>>,
    roots: OnceCell<Vec<(u16, u32)>>,
    lines: OnceCell<Vec<u32>>,
    kinds: OnceCell<Vec<u64>>,
}

impl<'a> FileVersion<'a> {
    pub(crate) fn new(table: &'a NodeTable, src: &'a [u8], file: u32, text: Option<&[u32]>) -> Self {
        let kind = node_table::kind_codes(table);
        if let Some(symbols) = text {
            assert_eq!(symbols.len(), kind.len());
        }
        Self {
            table, src, file, n: kind.len(), kind,
            start: node_table::starts(table),
            end: node_table::ends(table),
            parent: node_table::parent_ids(table),
            field: node_table::field_codes(table),
            text_memo: RefCell::new(text.unwrap_or_default().to_vec()),
            present_fields: Default::default(),
            by_start: Default::default(), by_end: Default::default(), by_line: Default::default(),
            facts: Default::default(), supplied: Default::default(),
            tokens: Default::default(), roots: Default::default(), lines: Default::default(),
            kinds: Default::default(),
        }
    }

    pub(crate) fn supply(&mut self, tables: impl IntoIterator<Item = (String, Rc<super::Table>)>) {
        self.supplied.extend(tables);
    }

    pub(super) fn extra_roots(&self) -> &Vec<(u16, u32)> {
        self.roots.get_or_init(|| self.parent.iter().enumerate().skip(1)
            .filter(|(i, parent)| *i == **parent as usize)
            .map(|(i, _)| (self.field[i], i as u32))
            .collect())
    }

    fn line_offsets(&self) -> &[u32] {
        self.lines.get_or_init(|| {
            let mut offsets = vec![0];
            for (i, &byte) in self.src.iter().enumerate() {
                if byte == b'\n' || (byte == b'\r' && self.src.get(i + 1) != Some(&b'\n')) {
                    offsets.push(i as u32 + 1);
                }
            }
            offsets
        })
    }
    pub(super) fn line(&self, off: u32) -> i64 {
        self.line_offsets().partition_point(|&s| s <= off) as i64
    }
    pub(super) fn column(&self, off: u32) -> i64 {
        let starts = self.line_offsets();
        i64::from(off - starts[starts.partition_point(|&s| s <= off) - 1])
    }
    pub(super) fn kind_bits(&self, run: &Run) -> &[u64] {
        self.kinds.get_or_init(|| {
            let mut bits = vec![0; run.kind_sym.len().div_ceil(64).max(1)];
            for c in 0..run.kind_sym.len() {
                if !node_table::ids_of_kind(self.table, KindBits(c as u16)).is_empty() {
                    bits[c / 64] |= 1 << (c % 64);
                }
            }
            bits
        })
    }
}
