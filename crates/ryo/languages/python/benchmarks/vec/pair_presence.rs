//! Cheap file-wide prerequisites of a domain or Boolean test. A traversal
//! inherits the presence requirements of its input. Alternatives keep only
//! prerequisites shared by both arms.
use super::*;
use rustc_hash::FxHashMap;
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
enum Atom {
    Kind(u16),
    Field(u16),
    Text(Id),
}
type Mask = Vec<(usize, u64)>;
pub(crate) struct Presence {
    atoms: Vec<Atom>,
    domains: Vec<Mask>,
    needs: Vec<Mask>,
}
fn join(l: &Mask, r: &Mask, both: bool) -> Mask {
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::new();
    while i < l.len() && j < r.len() {
        if l[i].0 < r[j].0 {
            if both {
                out.push(l[i]);
            }
            i += 1;
        } else if r[j].0 < l[i].0 {
            if both {
                out.push(r[j]);
            }
            j += 1;
        } else {
            let bits = if both {
                l[i].1 | r[j].1
            } else {
                l[i].1 & r[j].1
            };
            if bits != 0 {
                out.push((l[i].0, bits));
            }
            i += 1;
            j += 1;
        }
    }
    if both {
        out.extend_from_slice(&l[i..]);
        out.extend_from_slice(&r[j..]);
    }
    out
}
pub(crate) fn compile(p: &Program) -> Presence {
    let mut atoms = Vec::new();
    let mut ids = FxHashMap::default();
    let mut domains: Vec<Mask> = Vec::new();
    for (i, d) in p.domains.iter().enumerate() {
        let atom = match *d {
            Domain::Kind(k) | Domain::TextMatching(k, _) => Some(Atom::Kind(k)),
            Domain::Field(f) => Some(Atom::Field(f)),
            Domain::Text(_) => Some(Atom::Text(i)),
            _ => None,
        };
        let mask = if let Some(atom) = atom {
            let key = *ids.entry(atom).or_insert_with(|| {
                let n = atoms.len();
                atoms.push(atom);
                n
            });
            vec![(key / 64, 1u64 << (key % 64))]
        } else {
            match *d {
                Domain::Both(a, b) | Domain::Either(a, b) => {
                    join(&domains[a], &domains[b], matches!(d, Domain::Both(..)))
                }
                Domain::Parents(x) | Domain::Children(x) | Domain::Ancestors(x) => {
                    domains[x].clone()
                }
                _ => Vec::new(),
            }
        };
        domains.push(mask);
    }
    let mut needs: Vec<Mask> = Vec::new();
    for n in &p.needs {
        let mask = match *n {
            Need::Both(a, b) | Need::Either(a, b) => {
                join(&needs[a], &needs[b], matches!(n, Need::Both(..)))
            }
            Need::Some(d) => domains[d].clone(),
            _ => Vec::new(),
        };
        needs.push(mask);
    }
    Presence {
        atoms,
        domains,
        needs,
    }
}
impl File<'_> {
    pub(super) fn collect_absent(&mut self) {
        let Some(plan) = self.order.and_then(|o| o.presence.as_ref()) else {
            return;
        };
        self.absent.resize(plan.atoms.len().div_ceil(64), 0);
        self.index_texts();
        for (i, &atom) in plan.atoms.iter().enumerate() {
            let exists = match atom {
                Atom::Kind(k) => {
                    !node_table::ids_of_kind(self.t, languages::KindBits(k)).is_empty()
                }
                Atom::Field(f) => node_table::field_count(self.t, languages::FieldCode(f)).0 != 0,
                Atom::Text(d) => {
                    let Domain::Text(ref t) = self.p.domains[d] else {
                        unreachable!()
                    };
                    self.texts.as_ref().unwrap().contains_key(t.as_bytes())
                }
            };
            if !exists {
                self.absent[i / 64] |= 1 << (i % 64);
            }
        }
    }
    pub(super) fn missing(&self, id: Id, domain: bool) -> bool {
        let Some(p) = self.order.and_then(|o| o.presence.as_ref()) else {
            return false;
        };
        let mask = if domain { &p.domains[id] } else { &p.needs[id] };
        mask.iter().any(|&(i, m)| self.absent[i] & m != 0)
    }
}
