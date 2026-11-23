//! Source-order tree and token intervals from the provider's open/close events.
//! Typed nodes and tokens have separate contiguous preorders within a segment.
use super::types::NodeTable;
use ryo_relations::domain::relations::types::Position;
use std::ops::Range;

#[derive(Debug, Default)]
pub(super) struct TokenRanges {
    pub(super) active: bool,
    bounds: Vec<(u32, u32)>,
    segments: Vec<(u32, u32)>,
}
impl TokenRanges {
    pub(super) fn reset(&mut self) {
        self.active = false;
        self.bounds.clear();
        self.segments.clear();
    }
    pub(super) fn enter(&mut self, id: u32, tokens: usize) {
        if self.active {
            self.bounds.resize(id as usize, (0, 0));
            let n = u32::try_from(tokens).expect("token positions fit u32");
            self.bounds.push((n, n));
        }
    }
    pub(super) fn close(&mut self, id: u32, tokens: usize) {
        if self.active {
            self.bounds[id as usize].1 = u32::try_from(tokens).expect("token positions fit u32");
        }
    }
    pub(super) fn segment(&mut self, start: u32, first_token: usize) {
        if self.active && first_token > start as usize {
            self.segments.push((
                start,
                u32::try_from(first_token).expect("token positions fit u32"),
            ));
        }
    }
    fn base(&self, id: u32) -> Option<u32> {
        if self.segments.len() == 1 {
            let (start, tokens) = self.segments[0];
            return (start <= id && id < tokens).then_some(tokens);
        }
        let n = self.segments.partition_point(|&(start, _)| start <= id);
        let &(_, base) = self.segments.get(n.checked_sub(1)?)?;
        (id < base).then_some(base)
    }
}

#[derive(Debug, Clone)]
pub struct DescendantRanges {
    pub nodes: Range<u32>,
    pub tokens: Range<u32>,
}
/// The two exact intervals of ordinary descendants. Embedded segments have
/// their own token numbering and are separate from the host's descendants.
pub fn descendant_ranges(t: &NodeTable, id: Position) -> Option<DescendantRanges> {
    let c = &t.0;
    let n = &c.navigation;
    if !n.active {
        return None;
    }
    let Some(base) = n.base(id.0) else {
        return Some(DescendantRanges {
            nodes: 0..0,
            tokens: 0..0,
        });
    };
    let (start, end) = n.bounds[id.0 as usize];
    Some(DescendantRanges {
        nodes: id.0 + 1..c.subtree_end[id.0 as usize].0,
        tokens: base + start..base + end,
    })
}

/// Immediate children of a typed node, including interleaved tokens. No
/// contiguous adjacency column is needed to begin or advance this iterator.
#[derive(Debug)]
pub struct DirectChildren<'a> {
    table: &'a NodeTable,
    nodes: Range<u32>,
    tokens: Range<u32>,
    base: u32,
}
impl Iterator for DirectChildren<'_> {
    type Item = Position;
    fn next(&mut self) -> Option<Position> {
        if self.nodes.is_empty() {
            return self.tokens.next().map(Position);
        }
        let id = self.nodes.start;
        let (start, end) = self.table.0.navigation.bounds[id as usize];
        if self.tokens.start < self.base + start {
            return self.tokens.next().map(Position);
        }
        self.tokens.start = self.base + end;
        self.nodes.start = self.table.0.subtree_end[id as usize].0;
        Some(Position(id))
    }
}
/// Lend the child stream when the builder recorded token intervals.
pub fn direct_children(t: &NodeTable, id: Position) -> Option<DirectChildren<'_>> {
    let range = descendant_ranges(t, id)?;
    let base = t.0.navigation.base(id.0).unwrap_or(0);
    Some(DirectChildren {
        table: t,
        nodes: range.nodes,
        tokens: range.tokens,
        base,
    })
}
