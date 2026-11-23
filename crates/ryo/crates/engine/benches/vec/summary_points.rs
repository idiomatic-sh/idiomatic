//! Refine a candidate only after its native kind, field and fixed-parent
//! instructions have passed. A declared keyed fill is shared across all
//! predicates reading that node. Whole-only families retain their normal read.
use super::*;
impl File<'_> {
    fn bit_at(&self, set: Nodes, at: Position) -> bool {
        set.is_none_or(|s| self.arena.word(s, (at.0 / 64) as usize) & (1 << (at.0 % 64)) != 0)
    }
    fn point(&mut self, id: Id, at: Position) -> bool {
        *self.points += 1;
        if let Some(set) = self.sets.get(id) {
            return self.bit_at(set, at);
        }
        if let Domain::Fact(slot, ref test) = self.p.domains[id] {
            let spec = &self.p.facts[slot];
            if test.is_none()
                && let Some(yes) = (self.read)(
                    spec.source,
                    Query::HasAt(at),
                    self.words,
                    &mut FilledColumns::default(),
                )
            {
                return yes;
            }
            if !self.facts[slot].read
                && let Some(key) = self.order.and_then(|o| o.keys.get(slot)).copied().flatten()
            {
                let table = &mut self.facts[slot];
                let rows = table.at.entry(at.0).or_insert_with(|| {
                    let mut out = FilledColumns::default();
                    let complete =
                        (self.read)(spec.source, Query::Node(key, at), self.words, &mut out)
                            == Some(true);
                    let valid = out.0.first().is_none_or(|c| c.0.is_empty())
                        || (out.0.len() == spec.columns.len()
                            && out.0.iter().all(|c| c.0.len() == out.0[0].0.len()));
                    (complete && valid).then_some(out)
                });
                if let Some(rows) = rows
                    && let Some(keys) = spec.key.and_then(|k| rows.0.get(k))
                {
                    return keys.0.iter().enumerate().any(|(row, key)| {
                        key.0 == u64::from(at.0)
                            && test.as_ref().is_none_or(|(col, values)| {
                                super::super::summary_values::symbol(
                                    self.order,
                                    self.p,
                                    self.words,
                                    rows.0[*col].0[row],
                                )
                                .is_some_and(|i| values.binary_search(&i).is_ok())
                            })
                    });
                }
            }
        }
        let set = self.nodes(id);
        self.bit_at(set, at)
    }
    pub(super) fn local_answer(&mut self, id: Id) -> Option<bool> {
        let path = self.order?.paths.get(id)?.as_ref()?;
        let mut visited = 0;
        let out = path.visit(self.t, &mut visited, |at| {
            path.tests.iter().all(|&test| self.point(test, at))
        });
        *self.visits += visited;
        out
    }
    pub(super) fn local_set(&mut self, id: Id) -> Option<Nodes> {
        let order = self.order?;
        if !order.select {
            return None;
        }
        let path = order.paths.get(id)?.as_ref()?;
        let mut visited = 0;
        let mut hits = Vec::new();
        path.visit(self.t, &mut visited, |at| {
            if path.tests.iter().all(|&test| self.point(test, at)) {
                hits.push(at.0);
            }
            false
        })?;
        *self.visits += visited;
        Some(Some(self.arena.bits(hits)))
    }
}
