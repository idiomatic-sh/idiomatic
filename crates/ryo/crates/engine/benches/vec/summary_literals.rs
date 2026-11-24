//! Share one lookup of each name bucket entry between the free word filter
//! and the summary's literal-node tests. Other declared text buckets are read
//! on the first literal request in the file.
use super::{
    pair_summary::{Domain, Id, Program},
};
use rustc_hash::FxHashMap;
use ryo::languages::{KindBits, NodeTable, Position, node_table};
#[derive(Default, Clone, Copy)]
struct Entry {
    skip: Option<u16>,
    slot: Option<Id>,
}
pub(super) struct Layout {
    map: FxHashMap<Vec<u8>, Entry>,
    slots: Vec<Option<Id>>,
    count: usize,
    longest: usize,
    name_kind: Option<u16>,
    read_names: bool,
    other: Vec<u16>,
}
#[derive(Default)]
pub(super) struct Observed {
    nodes: Vec<Vec<u32>>,
    complete: bool,
}
impl Layout {
    pub(super) fn new(p: &Program, names: Option<(&std::collections::HashMap<Vec<u8>, u16>, u16)>) -> Self {
        let mut map = FxHashMap::<Vec<u8>, Entry>::default();
        if let Some((words, _)) = names {
            for (bytes, &skip) in words {
                map.entry(bytes.clone()).or_default().skip = Some(skip);
            }
        }
        let name_kind = names.map(|(_, kind)| kind);
        let mut slots = vec![None; p.domains.len()];
        let mut count = 0;
        for (id, d) in p.domains.iter().enumerate() {
            if let Domain::Text(s) = d {
                let slot = *map
                    .entry(s.as_bytes().to_vec())
                    .or_default()
                    .slot
                    .get_or_insert_with(|| {
                        let k = count;
                        count += 1;
                        k
                    });
                slots[id] = Some(slot);
            }
        }
        let longest = map.keys().map(Vec::len).max().unwrap_or(0);
        let read_names = name_kind.is_some_and(|k| p.text_kinds.contains(&k));
        let mut other: Vec<_> = p
            .text_kinds
            .iter()
            .copied()
            .filter(|&k| Some(k) != name_kind)
            .collect();
        other.sort_unstable();
        other.dedup();
        Self {
            map,
            slots,
            count,
            longest,
            name_kind,
            read_names,
            other,
        }
    }
    #[inline]
    fn lookup(&self, text: &[u8]) -> Entry {
        if text.len() > self.longest {
            return Entry::default();
        }
        self.map.get(text).copied().unwrap_or_default()
    }
    #[allow(dead_code, reason = "Only hosts exposing syntax-symbol columns use this entry point.")]
    pub(super) fn syntax(&self, ids: &[u32], symbols: &[Box<[u8]>], state: &mut Observed) {
        state.nodes.resize_with(self.count, Vec::new);
        for hits in &mut state.nodes { hits.clear(); }
        for (at, &symbol) in ids.iter().enumerate() {
            let text = &symbols[symbol as usize];
            if !text.is_empty() && let Some(slot) = self.lookup(text).slot {
                state.nodes[slot].push(at as u32);
            }
        }
        state.complete = true;
    }
    pub(super) fn names(
        &self,
        t: &NodeTable,
        src: &[u8],
        present: &mut [u64],
        state: &mut Observed,
    ) {
        state.nodes.resize_with(self.count, Vec::new);
        for ids in &mut state.nodes {
            ids.clear();
        }
        state.complete = self.other.is_empty();
        let ids = node_table::ids_of_kind(t, KindBits(self.name_kind.expect("declared name bucket")));
        for &at in ids {
            let raw = &src[super::span_bytes(t, at.0 as usize)];
            let entry = self.lookup(raw);
            if let Some(skip) = entry.skip {
                present[usize::from(skip) / 64] |= 1 << (skip % 64);
            }
            if self.read_names {
                let slot = match node_table::decoded_of(t, at) {
                    Some(text) if text.0.as_bytes() != raw => self.lookup(text.0.as_bytes()).slot,
                    _ => entry.slot,
                };
                if let Some(i) = slot {
                    state.nodes[i].push(at.0);
                }
            }
        }
    }
    pub(super) fn nodes<'a>(
        &self,
        id: Id,
        t: &NodeTable,
        src: &[u8],
        state: &'a mut Observed,
    ) -> &'a [u32] {
        if !state.complete {
            for &k in &self.other {
                let ids = node_table::ids_of_kind(t, KindBits(k));
                for &at in ids {
                    let text = node_table::decoded_of(t, Position(at.0)).map_or_else(
                        || &src[super::span_bytes(t, at.0 as usize)],
                        |v| v.0.as_bytes(),
                    );
                    if let Some(i) = self.lookup(text).slot {
                        state.nodes[i].push(at.0);
                    }
                }
            }
            state.complete = true;
        }
        &state.nodes[self.slots[id].expect("literal register")]
    }
}
