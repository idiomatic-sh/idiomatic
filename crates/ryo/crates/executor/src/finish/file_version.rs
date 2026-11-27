//! Data shared by every program visiting one source version in one symbol dictionary.
use super::{Index, KindBits, NodeTable, Position, Run, Tokens, node_table};
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

pub(crate) struct FileVersion<'a> {
    pub(crate) table: &'a NodeTable,
    pub(crate) src: &'a [u8],
    pub(crate) file: u32,
    pub(crate) n: usize,
    /// How many elements are the file's own tree: the table below `own_end`, the elements of a relation of extent `Own`.
    pub(crate) own: usize,
    pub(crate) kind: &'a [u16],
    pub(crate) start: &'a [u32],
    pub(crate) end: &'a [u32],
    pub(crate) parent: &'a [u32],
    pub(crate) field: &'a [u16],
    pub(crate) text_memo: RefCell<Vec<u32>>,
    pub(crate) present_fields: OnceCell<rustc_hash::FxHashSet<u32>>,
    pub(crate) by_start: RefCell<Option<Index>>,
    pub(crate) by_end: RefCell<Option<Index>>,
    pub(crate) by_line: [RefCell<Option<Index>>; 2],
    pub(crate) tokens: RefCell<Option<Rc<Tokens>>>,
    pub(crate) roots: OnceCell<Vec<(u16, u32)>>,
    pub(crate) attached: OnceCell<Vec<(u32, u32)>>,
    pub(crate) lines: OnceCell<Vec<u32>>,
    pub(crate) kinds: OnceCell<Vec<u64>>,
}

impl<'a> FileVersion<'a> {
    pub fn new(table: &'a NodeTable, src: &'a [u8], file: u32, text: Option<Vec<u32>>) -> Self {
        let kind = node_table::kind_codes(table);
        if let Some(symbols) = &text {
            assert_eq!(symbols.len(), kind.len());
        }
        debug_assert!(node_table::embedded_segments(table).all(|segment| usize::from(segment.root) >= usize::try_from(node_table::own_end(table).0).unwrap_or(usize::MAX)), "every embedded root lies at or after own_end");
        Self {
            table, src, file, n: kind.len(), own: usize::try_from(node_table::own_end(table).0).unwrap_or(usize::MAX).min(kind.len()), kind,
            start: node_table::starts(table),
            end: node_table::ends(table),
            parent: node_table::parent_ids(table),
            field: node_table::field_codes(table),
            text_memo: RefCell::new(text.unwrap_or_default()),
            present_fields: Default::default(),
            by_start: Default::default(), by_end: Default::default(), by_line: Default::default(),
            attached: Default::default(), tokens: Default::default(), roots: Default::default(), lines: Default::default(),
            kinds: Default::default(),
        }
    }

    /// How many elements of the table a node relation of `extent` covers.
    pub(crate) fn limit(&self, extent: super::Extent) -> usize {
        match extent {
            super::Extent::Own => self.own,
            super::Extent::Whole => self.n,
        }
    }

    /// The ids of a bucket the extent covers: below `own_end` for the file's own tree. One compare to pick the slice, none a row;
    /// a file with no embedded segment has `own == n` and the slice is returned as it is.
    pub(crate) fn cut<'s>(&self, ids: &'s [Position], extent: super::Extent) -> &'s [Position] {
        if extent == super::Extent::Whole || self.own >= self.n {
            return ids;
        }
        debug_assert!(ids.windows(2).all(|w| w[0] < w[1]), "a kind or field list is in ascending position order");
        let own = self.own as u32;
        &ids[..ids.partition_point(|p| p.0 < own)]
    }


    pub(crate) fn extra_roots(&self) -> &Vec<(u16, u32)> {
        self.roots.get_or_init(|| self.parent.iter().enumerate().skip(1)
            .filter(|(i, parent)| *i == **parent as usize)
            .map(|(i, _)| (self.field[i], i as u32))
            .collect())
    }

    /// The inverse of the parent column also includes roots attached after their host closed.
    /// The concrete child slice only contains positions from the host's own segment.
    pub(crate) fn child_rows(&self, parent: u64) -> impl Iterator<Item = u32> + '_ {
        let host = u32::try_from(parent).unwrap_or(u32::MAX);
        let attached = self.attached.get_or_init(|| {
            let mut roots: Vec<_> = node_table::embedded_segments(self.table)
                .map(|segment| (segment.host.0, segment.root.0))
                .collect();
            roots.sort_unstable();
            roots
        });
        let start = attached.partition_point(|&(id, _)| id < host);
        node_table::children_of(self.table, Position(host)).iter().map(|child| child.0)
            .chain(attached[start..].iter().take_while(move |&&(id, _)| id == host).map(|&(_, root)| root))
            .filter(move |&child| child != host && self.parent.get(child as usize) == Some(&host))
    }

    pub(crate) fn line_offsets(&self) -> &[u32] {
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
    pub(crate) fn line(&self, off: u32) -> i64 {
        self.line_offsets().partition_point(|&s| s <= off) as i64
    }
    pub(crate) fn column(&self, off: u32) -> i64 {
        let starts = self.line_offsets();
        i64::from(off - starts[starts.partition_point(|&s| s <= off) - 1])
    }
    pub(crate) fn kind_bits(&self, run: &Run) -> &[u64] {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::{Count, EmbeddedMapping, EmbeddedText, FieldCode, NameText, Offset};

    #[test]
    fn parent_probe_includes_each_attached_root_and_concrete_child() {
        let mut table = NodeTable::default();
        let mut writer = node_table::builder(&mut table, Count(2));
        node_table::enter_node(&mut writer, KindBits(0), FieldCode(0), Offset(0));
        node_table::token(&mut writer, KindBits(1), FieldCode(0), Offset(0), Offset(1));
        node_table::leave_node(&mut writer, Offset(1));
        for _ in 0..2 {
            node_table::embed(&mut writer, Position(0), EmbeddedText {
                text: NameText("x"), mapping: EmbeddedMapping::Exact(Offset(0)),
            });
            node_table::enter_node(&mut writer, KindBits(0), FieldCode(0), Offset(0));
            node_table::leave_node(&mut writer, Offset(1));
        }
        let file = FileVersion::new(node_table::finished(&mut writer), b"x", 0, None);
        assert_eq!(file.child_rows(0).collect::<Vec<_>>(), [1, 2, 3]);
        assert_eq!(file.child_rows(u64::MAX).count(), 0);
    }
}
