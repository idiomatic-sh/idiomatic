//! Candidate-local evaluation for an existential node formula. Enumerate the
//! smallest native bucket in its conjunction. Other terms see only surviving ids.
use super::*;
impl File<'_> {
    pub(super) fn contains(&mut self, id: Id, at: Position) -> bool {
        self.node_tests += 1;
        if let Some(s) = &self.sets[id] {
            return holds(s, at);
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
            Domain::Fact(slot, _) | Domain::Matching(slot, _, _)
                if !self.facts[slot].read && order.keys[slot].is_some() =>
            {
                self.at_fact(id, slot, at)
            }
            _ => holds(&self.nodes(id), at),
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

    pub(super) fn native_witness(&mut self, id: Id) -> Option<bool> {
        // The node table is borrowed independently of the mutable per-file memo.
        let t = self.t;
        let key = self.order?.buckets[id].iter().min_by_key(|b| match b {
            Bucket::Kind(k) => node_table::ids_of_kind(t, languages::KindBits(*k)).len(),
            Bucket::Field(f) => node_table::field_count(t, languages::FieldCode(*f)).0,
        })?;
        self.bucket_requests += 1;
        if self.order.unwrap().measure {
            self.visited_buckets.insert(*key);
        }
        let ids = match key {
            Bucket::Kind(k) => node_table::ids_of_kind(t, languages::KindBits(*k)),
            Bucket::Field(f) => node_table::ids_of_field(t, languages::FieldCode(*f)),
        };
        Some(ids.iter().copied().any(|at| {
            self.candidates += 1;
            self.contains(id, at)
        }))
    }
}
fn holds(s: &Nodes, at: Position) -> bool {
    s.as_ref().is_none_or(|bits| {
        bits.get((at.0 / 64) as usize)
            .is_some_and(|w| w & (1 << (at.0 % 64)) != 0)
    })
}
