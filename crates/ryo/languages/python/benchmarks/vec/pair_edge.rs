//! Local syntax witnesses that describe one element or one tree edge. All
//! signatures anchored in the same file bucket are answered by a single scan.
use super::*;
use rustc_hash::{FxHashMap, FxHashSet};
#[derive(Clone, Copy, Hash, Eq, PartialEq)]
enum Edge {
    Node,
    Normal,
    WithEmbedded,
}
#[derive(Default, Clone, Copy)]
struct Terms {
    kind: Option<u16>,
    field: Option<u16>,
}
impl Terms {
    fn add(&mut self, d: &Domain) -> bool {
        match *d {
            Domain::Kind(k) if self.kind.is_none_or(|v| v == k) => {
                self.kind = Some(k);
                true
            }
            Domain::Field(f) if self.field.is_none_or(|v| v == f) => {
                self.field = Some(f);
                true
            }
            _ => false,
        }
    }
}
fn terms(p: &Program, id: Id, into: &mut Terms) -> bool {
    match p.domains[id] {
        Domain::Both(a, b) => terms(p, a, into) && terms(p, b, into),
        ref d => into.add(d),
    }
}
fn pattern(p: &Program, id: Id) -> Option<(Bucket, Edge, u8, [u16; 4])> {
    let mut here = Terms::default();
    let mut neighbor = Terms::default();
    let mut edge = Edge::Node;
    let mut stack = vec![id];
    while let Some(i) = stack.pop() {
        match p.domains[i] {
            Domain::Both(a, b) => stack.extend([a, b]),
            Domain::Parents(x) | Domain::Children(x) if edge == Edge::Node => {
                edge = if matches!(p.domains[i], Domain::Parents(_)) {
                    Edge::WithEmbedded
                } else {
                    Edge::Normal
                };
                if !terms(p, x, &mut neighbor) {
                    return None;
                }
            }
            ref d => {
                if !here.add(d) {
                    return None;
                }
            }
        }
    }
    let (c, parent) = if edge == Edge::WithEmbedded {
        (neighbor, here)
    } else {
        (here, neighbor)
    };
    let bucket = c
        .kind
        .map(Bucket::Kind)
        .or_else(|| c.field.map(Bucket::Field))?;
    let terms = [c.kind, c.field, parent.kind, parent.field];
    let mut mask = 0u8;
    for (i, t) in terms.iter().enumerate() {
        if t.is_some() {
            mask |= 1 << i;
        }
    }
    if edge == Edge::Node && mask.count_ones() < 2 {
        return None;
    }
    Some((bucket, edge, mask, terms.map(|v| v.unwrap_or_default())))
}
struct Group {
    bucket: Bucket,
    masks: Vec<(Edge, u8, FxHashMap<[u16; 4], Vec<Id>>)>,
}
pub(crate) struct Groups {
    map: Vec<Option<usize>>,
    groups: Vec<Group>,
}
pub(crate) fn compile(p: &Program) -> Groups {
    let mut gs = Groups {
        map: vec![None; p.domains.len()],
        groups: Vec::new(),
    };
    let mut ix = FxHashMap::default();
    for n in &p.needs {
        let Need::Some(id) = *n else { continue };
        if gs.map[id].is_some() {
            continue;
        }
        let Some((bucket, edge, mask, key)) = pattern(p, id) else {
            continue;
        };
        let group = *ix.entry(bucket).or_insert_with(|| {
            let id = gs.groups.len();
            gs.groups.push(Group {
                bucket,
                masks: Vec::new(),
            });
            id
        });
        let masks = &mut gs.groups[group].masks;
        let i = masks
            .iter()
            .position(|v| v.0 == edge && v.1 == mask)
            .unwrap_or_else(|| {
                let i = masks.len();
                masks.push((edge, mask, FxHashMap::default()));
                i
            });
        masks[i].2.entry(key).or_default().push(id);
        gs.map[id] = Some(group);
    }
    gs
}
#[derive(Default)]
pub(super) struct Answers {
    done: FxHashSet<usize>,
    yes: FxHashSet<Id>,
}
impl File<'_> {
    pub(super) fn edge_witness(&mut self, id: Id) -> Option<bool> {
        let groups = self.order?.edges.as_ref()?;
        let g = groups.map[id]?;
        if self.edges.done.insert(g) {
            let group = &groups.groups[g];
            let t = self.t;
            let ids = match group.bucket {
                Bucket::Kind(k) => node_table::ids_of_kind(t, languages::KindBits(k)),
                Bucket::Field(f) => node_table::ids_of_field(t, languages::FieldCode(f)),
                Bucket::Text(_) => unreachable!(),
            };
            let kinds = node_table::kind_codes(t);
            let fields = node_table::field_codes(t);
            let parents = node_table::parent_ids(t);
            let roots: Vec<_> = node_table::embedded_segments(t).map(|s| s.root.0).collect();
            let mut seen = FxHashSet::default();
            for &at in ids {
                let n = at.0 as usize;
                let parent = parents[n] as usize;
                let edge = if parent == n || parent >= kinds.len() {
                    Edge::Node
                } else if !roots.is_empty() && roots.binary_search(&at.0).is_ok() {
                    Edge::WithEmbedded
                } else {
                    Edge::Normal
                };
                let parent = if edge == Edge::Node { n } else { parent };
                let values = [kinds[n], fields[n], kinds[parent], fields[parent]];
                if !seen.insert((edge, values)) {
                    continue;
                }
                for (wanted, mask, lookup) in &group.masks {
                    if *wanted != Edge::Node
                        && (edge == Edge::Node
                            || (*wanted == Edge::Normal && edge == Edge::WithEmbedded))
                    {
                        continue;
                    }
                    let key =
                        std::array::from_fn(|i| if mask & (1 << i) == 0 { 0 } else { values[i] });
                    if let Some(ids) = lookup.get(&key) {
                        self.edges.yes.extend(ids);
                    }
                }
            }
            self.bucket_requests += 1;
        }
        Some(self.edges.yes.contains(&id))
    }
}
