//! Each demanded input column or kind bucket is scanned once for the regular
//! expressions that share it. The set has the same byte-regex semantics as its
//! individually compiled members.
use super::*;
use regex::bytes::RegexSet;
use rustc_hash::FxHashMap;

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
enum Source {
    Kind(u16),
    Column(usize, usize),
}
struct Group {
    source: Source,
    set: RegexSet,
    members: Vec<Id>,
}
pub(crate) struct Groups {
    by_domain: Vec<Option<usize>>,
    groups: Vec<Group>,
    pub(crate) ungrouped: usize,
}
pub(crate) fn compile(p: &Program) -> Groups {
    let mut sources = FxHashMap::<Source, Vec<(Id, usize)>>::default();
    for (id, d) in p.domains.iter().enumerate() {
        let (source, re) = match *d {
            Domain::TextMatching(k, re) => (Source::Kind(k), re),
            Domain::Matching(slot, col, re) => (Source::Column(slot, col), re),
            _ => continue,
        };
        sources.entry(source).or_default().push((id, re));
    }
    let mut out = Groups {
        by_domain: vec![None; p.domains.len()],
        groups: Vec::new(),
        ungrouped: 0,
    };
    for (source, ds) in sources {
        if ds.len() < 2 {
            continue;
        }
        let Ok(set) = RegexSet::new(ds.iter().map(|x| p.regexes[x.1].as_str())) else {
            out.ungrouped += ds.len();
            continue;
        };
        let members: Vec<_> = ds.iter().map(|x| x.0).collect();
        for &id in &members {
            out.by_domain[id] = Some(out.groups.len());
        }
        out.groups.push(Group {
            source,
            set,
            members,
        });
    }
    out
}
impl File<'_> {
    pub(super) fn grouped_matches(&mut self, id: Id) -> Option<Nodes> {
        let o = self.order?;
        let gs = o.regex.as_ref()?;
        let g = &gs.groups[gs.by_domain[id]?];
        let mut hits = vec![Vec::new(); g.members.len()];
        match g.source {
            Source::Kind(k) => {
                let entries = node_table::ids_of_kind(self.t, languages::KindBits(k));
                for &at in entries {
                    let text = node_table::decoded_of(self.t, at).map_or_else(
                        || &self.src[super::super::span_bytes(self.t, at.0 as usize)],
                        |t| t.0.as_bytes(),
                    );
                    for member in g.set.matches(text).iter() {
                        hits[member].push(at.0);
                    }
                }
            }
            Source::Column(slot, col) => {
                let spec = &self.p.facts[slot];
                let table = &mut self.facts[slot];
                let n = node_table::element_count(self.t).0 as u32;
                table.load(spec, n, self.words, self.read);
                if !table.complete {
                    for &member in &g.members {
                        self.sets[member] = Some(None);
                    }
                    return Some(None);
                }
                let keys = spec.key.and_then(|k| table.cells.0.get(k));
                if let Some(input) = table.cells.0.get(col) {
                    for (row, &v) in input.0.iter().enumerate() {
                        let text = if spec.kinds[col] == languages::CellKind::Span {
                            let Some(bytes) =
                                self.src.get((v.0 >> 32) as usize..(v.0 as u32) as usize)
                            else {
                                continue;
                            };
                            bytes
                        } else {
                            languages::facts::text(self.words, v).0.as_bytes()
                        };
                        let at = keys.map_or(0, |keys| keys.0[row].0 as u32);
                        for member in g.set.matches(text).iter() {
                            hits[member].push(at);
                        }
                    }
                }
            }
        }
        for (&member, ids) in g.members.iter().zip(hits) {
            self.sets[member] = Some(Some(self.arena.bits(ids)));
        }
        self.sets[id]
    }
}
