//! Candidate-local evaluation for an existential node formula. Enumerate the
//! smallest native bucket in its conjunction. Other terms see only surviving ids.
use super::*;
impl File<'_> {
    pub(super) fn contains(&mut self, id: Id, at: Position) -> bool {
        let shared = self.order.expect("ordered evaluator").shared[id];
        let key = (id, at.0 / 64);
        let bit = 1u64 << (at.0 % 64);
        if shared
            && let Some(&(seen, pass)) = self.local.get(&key)
            && seen & bit != 0
        {
            return pass & bit != 0;
        }
        let yes = self.test_at(id, at);
        if shared {
            let (seen, pass) = self.local.entry(key).or_default();
            *seen |= bit;
            if yes {
                *pass |= bit;
            }
        }
        yes
    }
    fn test_at(&mut self, id: Id, at: Position) -> bool {
        self.node_tests += 1;
        if let Some(s) = &self.sets[id] {
            return holds(self.arena, s, at);
        }
        let order = self.order.expect("ordered evaluator");
        match self.p.domains[id] {
            Domain::Any => true,
            Domain::Empty => false,
            Domain::Kind(k) => node_table::kind_of(self.t, at).0 == k,
            Domain::Field(f) => node_table::field_of(self.t, at).0 == f,
            Domain::TextMatching(k, re) => {
                node_table::kind_of(self.t, at).0 == k && self.p.regexes[re].is_match(self.text(at))
            }
            Domain::Text(ref t) => {
                order.texts.get(node_table::kind_of(self.t, at).0 as usize) == Some(&true)
                    && self.text(at) == t.as_bytes()
            }
            Domain::Both(a, b) | Domain::Either(a, b) => {
                let (a, b) = if order.costs[a] <= order.costs[b] {
                    (a, b)
                } else {
                    (b, a)
                };
                if matches!(self.p.domains[id], Domain::Both(..)) {
                    self.contains(a, at) && self.contains(b, at)
                } else {
                    self.contains(a, at) || self.contains(b, at)
                }
            }
            Domain::Children(x) if order.concrete[x] => {
                let t = self.t;
                let parent = node_table::parent_of(t, at);
                parent != at
                    && !node_table::embedded_segments(t).any(|s| s.root == at)
                    && self.contains(x, parent)
            }
            Domain::Parents(x) if order.concrete[x] => {
                let t = self.t;
                node_table::children_of(t, at)
                    .iter()
                    .copied()
                    .any(|c| self.contains(x, c))
                    || node_table::embedded_segments(t)
                        .any(|s| s.host == at && self.contains(x, s.root))
            }
            Domain::Fact(slot, _) | Domain::Matching(slot, _, _)
                if !self.facts[slot].read && order.keys[slot].is_some() =>
            {
                self.at_fact(id, slot, at)
            }
            _ => {
                let s = self.nodes(id);
                holds(self.arena, &s, at)
            }
        }
    }
    fn text(&self, at: Position) -> &[u8] {
        node_table::decoded_of(self.t, at).map_or_else(
            || &self.src[super::super::span_bytes(self.t, at.0 as usize)],
            |s| s.0.as_bytes(),
        )
    }

    // A declared node key's fill is the corresponding slice of a whole fill.
    // Retain one answer per demanded key. Partial or refused answers admit it.
    fn at_fact(&mut self, id: Id, slot: usize, at: Position) -> bool {
        if matches!(self.p.domains[id], Domain::Fact(_, None))
            && let Some(answer) = (self.read)(
                self.p.facts[slot].relation,
                Request::Has(Some(at)),
                self.words,
                &mut FilledColumns::default(),
            )
        {
            return answer;
        }
        let key = self.order.unwrap().keys[slot].expect("declared node key");
        let table = &mut self.facts[slot];
        let spec = &self.p.facts[slot];
        let rows = table.by_node.entry(at.0).or_insert_with(|| {
            let mut rows = FilledColumns::default();
            let wanted = languages::Wanted::Key(key, languages::CellWord(u64::from(at.0)));
            let ok = (self.read)(spec.relation, Request::Rows(wanted), self.words, &mut rows)
                .expect("row fill answered");
            ok.then_some(rows)
        });
        let Some(rows) = rows else { return true };
        match &self.p.domains[id] {
            Domain::Fact(_, None) => rows.0.first().is_some_and(|c| !c.0.is_empty()),
            Domain::Fact(_, Some((col, values))) => rows.0.get(*col).is_some_and(|c| {
                c.0.iter().any(|&v| {
                    self.p
                        .symbols
                        .get(languages::facts::text(self.words, v).0)
                        .is_some_and(|s| values.contains(s))
                })
            }),
            Domain::Matching(_, col, re) => rows.0.get(*col).is_some_and(|c| {
                c.0.iter().any(|&v| {
                    if spec.kinds[*col] == languages::CellKind::Span {
                        self.src
                            .get((v.0 >> 32) as usize..(v.0 as u32) as usize)
                            .is_some_and(|s| self.p.regexes[*re].is_match(s))
                    } else {
                        self.p.regexes[*re]
                            .is_match(languages::facts::text(self.words, v).0.as_bytes())
                    }
                })
            }),
            _ => unreachable!(),
        }
    }

    // Below one candidate per machine word, testing the candidates is cheaper
    // in iterations than touching every word of even one file-wide bitset.
    pub(super) fn sparse_witness(&mut self, id: Id) -> Option<bool> {
        let t = self.t;
        let order = self.order?;
        let mut limit = node_table::element_count(t).0.div_ceil(u64::BITS as usize);
        let mut best = None;
        for &key in &order.buckets[id] {
            let n = match key {
                Bucket::Kind(k) => node_table::ids_of_kind(t, languages::KindBits(k)).len(),
                Bucket::Field(f) => node_table::field_count(t, languages::FieldCode(f)).0,
                Bucket::Text(d) => {
                    self.index_texts();
                    let Domain::Text(ref text) = self.p.domains[d] else {
                        unreachable!()
                    };
                    self.texts
                        .as_ref()
                        .unwrap()
                        .get(text.as_bytes())
                        .map_or(0, Vec::len)
                }
            };
            if n == 0 {
                return Some(false);
            }
            if n <= limit {
                best = Some(key);
                limit = n;
            }
        }
        let key = best?;
        self.bucket_requests += 1;
        if order.measure {
            self.visited_buckets.insert(key);
        }
        let ids = match key {
            Bucket::Kind(k) => node_table::ids_of_kind(t, languages::KindBits(k)),
            Bucket::Field(f) => node_table::ids_of_field(t, languages::FieldCode(f)),
            Bucket::Text(d) => {
                let set = self.nodes(d).expect("finite literal domain");
                return Some((0..set.len()).any(|w| {
                    let mut bits = self.arena.word(set, w);
                    while bits != 0 {
                        let at = Position(w as u32 * 64 + bits.trailing_zeros());
                        self.candidates += 1;
                        if self.contains(id, at) {
                            return true;
                        }
                        bits &= bits - 1;
                    }
                    false
                }));
            }
        };
        Some(ids.iter().copied().any(|at| {
            self.candidates += 1;
            self.contains(id, at)
        }))
    }
}
fn holds(arena: &Arena, s: &Nodes, at: Position) -> bool {
    s.is_none_or(|b| {
        let i = (at.0 / 64) as usize;
        i < b.len() && arena.word(b, i) & (1 << (at.0 % 64)) != 0
    })
}
