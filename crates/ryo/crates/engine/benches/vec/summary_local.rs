//! Exact existential tests that walk a fixed chain of native parent links.
//! Each instruction checks a kind and/or field at one distance from the
//! enumerated node. Other expressions continue through their shared registers.
use super::pair_summary::{Domain, Id};
use ryo::languages::{self, NodeTable, Position, node_table};

#[derive(Default, Clone)]
struct Place {
    kind: Option<u16>,
    field: Option<u16>,
    parent: bool,
    ordinary: bool,
}
#[derive(Default)]
pub(super) struct Path {
    places: Vec<Place>,
}
impl Path {
    fn at(&mut self, d: &[Domain], id: Id, level: usize) -> bool {
        if self.places.len() <= level {
            self.places.resize_with(level + 1, Place::default);
        }
        let p = &mut self.places[level];
        match d[id] {
            Domain::Any => true,
            Domain::HasParent => {
                p.parent = true;
                true
            }
            Domain::Kind(k) => p.kind.replace(k).is_none_or(|old| old == k),
            Domain::Field(f) => p.field.replace(f).is_none_or(|old| old == f),
            Domain::Both(a, b) => self.at(d, a, level) && self.at(d, b, level),
            Domain::Children(a) => {
                p.parent = true;
                p.ordinary = true;
                self.at(d, a, level + 1)
            }
            _ => false,
        }
    }
    pub(super) fn new(domains: &[Domain], id: Id) -> Option<Self> {
        if !matches!(
            domains[id],
            Domain::Both(..) | Domain::Children(..) | Domain::Parents(..)
        ) {
            return None;
        }
        let mut stack = vec![id];
        let mut ancestors = Vec::new();
        let mut child = None;
        while let Some(x) = stack.pop() {
            match domains[x] {
                Domain::Both(a, b) => stack.extend([a, b]),
                Domain::Parents(a) => {
                    if child.replace(a).is_some() {
                        return None;
                    }
                }
                _ => ancestors.push(x),
            }
        }
        let mut path = Self::default();
        let level = usize::from(child.is_some());
        if let Some(c) = child {
            if !path.at(domains, c, 0) {
                return None;
            }
            path.places[0].parent = true;
        }
        for a in ancestors {
            if !path.at(domains, a, level) {
                return None;
            }
        }
        let p = path.places.first()?;
        (p.kind.is_some() || p.field.is_some()).then_some(path)
    }
    pub(super) fn witness(&self, t: &NodeTable, visited: &mut u64) -> Option<bool> {
        let first = &self.places[0];
        let kind = first
            .kind
            .map(|k| node_table::ids_of_kind(t, languages::KindBits(k)));
        let field = first.field.map(|f| languages::FieldCode(f));
        let ids = match (kind, field) {
            (Some(ids), Some(f)) if node_table::field_count(t, f).0 < ids.len() => {
                node_table::ids_of_field(t, f)
            }
            (Some(ids), _) => ids,
            (None, Some(f)) => node_table::ids_of_field(t, f),
            _ => return None,
        };
        if ids.len() > node_table::element_count(t).0.div_ceil(u64::BITS as usize) {
            return None;
        }
        Some(ids.iter().copied().any(|mut node: Position| {
            *visited += 1;
            for p in &self.places {
                if p.kind.is_some_and(|k| node_table::kind_of(t, node).0 != k)
                    || p.field
                        .is_some_and(|k| node_table::field_of(t, node).0 != k)
                {
                    return false;
                }
                if p.parent {
                    if p.ordinary && node_table::embedded_segments(t).any(|s| s.root == node) {
                        return false;
                    }
                    let up = node_table::parent_of(t, node);
                    if up == node {
                        return false;
                    }
                    node = up;
                }
            }
            true
        }))
    }
}
