//! A file's observed kind/field signatures and their tree edges. Collisions
//! only enlarge a possible class set. An empty intersection proves there is
//! no real node before the evaluator requests any semantic columns.
use super::*;
#[derive(Default)]
pub(crate) struct Classes {
    keys: Vec<u8>,
    kinds: Vec<u64>,
    fields: Vec<u64>,
    up: Vec<u64>,
    down: Vec<u64>,
    above: Vec<u64>,
    below: Vec<u64>,
    memo: Vec<Option<u64>>,
    parents: u64,
    pub(crate) built: u64,
    pub(crate) tested: u64,
    pub(crate) ruled_out: u64,
}

fn image(mut set: u64, edges: &[u64]) -> u64 {
    if set == u64::MAX {
        return u64::MAX;
    }
    let mut out = 0;
    while set != 0 {
        out |= edges[set.trailing_zeros() as usize];
        set &= set - 1;
    }
    out
}
fn transitive(edges: &[u64], result: &mut Vec<u64>) {
    result.clear();
    result.extend_from_slice(edges);
    for k in 0..u64::BITS as usize {
        let from = result[k];
        let bit = 1u64 << k;
        for row in result.iter_mut() {
            if *row & bit != 0 {
                *row |= from;
            }
        }
    }
}
impl Classes {
    pub(super) fn reset(&mut self) {
        self.keys.clear();
        self.memo.clear();
    }
    fn prepare(&mut self, p: &Program, t: &NodeTable) {
        self.kinds.fill(0);
        self.fields.fill(0);
        self.parents = 0;
        self.up.clear();
        self.up.resize(u64::BITS as usize, 0);
        self.down.clear();
        self.down.resize(u64::BITS as usize, 0);
        self.above.clear();
        self.below.clear();
        self.memo.resize(p.domains.len(), None);
        let ks = node_table::kind_codes(t);
        let fs = node_table::field_codes(t);
        let ps = node_table::parent_ids(t);
        for (&k, &f) in ks.iter().zip(fs) {
            // One machine word represents all possible classes. This hash
            // is independent of file contents and of the language vocabulary.
            let packed = (u32::from(k) << 16) | u32::from(f);
            let code = (packed.wrapping_mul(2654435761) >> (32 - 6)) as u8;
            let bit = 1u64 << code;
            self.keys.push(code);
            let k = usize::from(k);
            let f = usize::from(f);
            if self.kinds.len() <= k {
                self.kinds.resize(k + 1, 0);
            }
            if self.fields.len() <= f {
                self.fields.resize(f + 1, 0);
            }
            self.kinds[k] |= bit;
            self.fields[f] |= bit;
        }
        for (n, &parent) in ps.iter().enumerate() {
            let parent = parent as usize;
            if n == parent || parent >= self.keys.len() {
                continue;
            }
            let a = self.keys[parent];
            let b = self.keys[n];
            self.parents |= 1u64 << b;
            self.up[b as usize] |= 1u64 << a;
            self.down[a as usize] |= 1u64 << b;
        }
        self.built += 1;
    }
    pub(super) fn empty(
        &mut self,
        p: &Program,
        t: &NodeTable,
        texts: Option<&Map<&[u8], Vec<u32>>>,
        id: Id,
    ) -> bool {
        if self.memo.is_empty() {
            self.prepare(p, t);
        }
        let empty = self.mask(p, texts, id) == 0;
        self.ruled_out += u64::from(empty);
        empty
    }
    fn mask(&mut self, p: &Program, texts: Option<&Map<&[u8], Vec<u32>>>, id: Id) -> u64 {
        if let Some(v) = self.memo[id] {
            return v;
        }
        self.tested += 1;
        let set = match &p.domains[id] {
            Domain::Empty => 0,
            Domain::HasParent => self.parents,
            Domain::Kind(k) | Domain::TextMatching(k, _) | Domain::SpanMatching(k, _) => {
                self.kinds.get(usize::from(*k)).copied().unwrap_or(0)
            }
            Domain::Field(f) => self.fields.get(usize::from(*f)).copied().unwrap_or(0),
            Domain::Text(word) => {
                if let Some(texts) = texts {
                    texts
                        .get(word.as_bytes())
                        .into_iter()
                        .flatten()
                        .fold(0, |m, &at| m | (1u64 << self.keys[at as usize]))
                } else {
                    p.text_kinds
                        .iter()
                        .filter_map(|k| self.kinds.get(usize::from(*k)))
                        .fold(0, |a, b| a | b)
                }
            }
            Domain::Both(a, b) => {
                let x = self.mask(p, texts, *a);
                if x == 0 {
                    0
                } else {
                    x & self.mask(p, texts, *b)
                }
            }
            Domain::Either(a, b) => {
                let x = self.mask(p, texts, *a);
                if x == u64::MAX {
                    x
                } else {
                    x | self.mask(p, texts, *b)
                }
            }
            Domain::Except(x, _) | Domain::When(_, x) => self.mask(p, texts, *x),
            Domain::ProvenExcept(a, b, ..) => {
                let x = self.mask(p, texts, *a);
                if x == 0 {
                    0
                } else {
                    x | self.mask(p, texts, *b)
                }
            }
            Domain::Parents(x) => {
                let v = self.mask(p, texts, *x);
                image(v, &self.up)
            }
            Domain::Children(x) => {
                let v = self.mask(p, texts, *x);
                image(v, &self.down)
            }
            Domain::Ancestors(x) => {
                let v = self.mask(p, texts, *x);
                if self.above.is_empty() {
                    transitive(&self.up, &mut self.above);
                }
                image(v, &self.above)
            }
            Domain::Descendants(x) => {
                let v = self.mask(p, texts, *x);
                if self.below.is_empty() {
                    transitive(&self.down, &mut self.below);
                }
                image(v, &self.below)
            }
            Domain::CountUnder(x, d, ..) => {
                let mut v = self.mask(p, texts, *x);
                for _ in 0..*d {
                    v = image(v, &self.up);
                }
                v
            }
            _ => u64::MAX,
        };
        self.memo[id] = Some(set);
        set
    }
}
