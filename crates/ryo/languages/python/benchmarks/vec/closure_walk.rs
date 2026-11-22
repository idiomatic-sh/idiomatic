//! THROWAWAY (vec 6e-K, item 2): a closure asked from one end is walked from that end, its edges probed by the frontier's
//! keys a level at a time (through whatever steps make them, each (step, column, key) once a file) and never made whole.
//! Where the edges are functional on the walked end (a tree child has one parent), each probe answers at most one row and
//! the walk is a chain. The reach answered is the closure's own: every node one or more edges away, the start included
//! only through a cycle. No rule id: the shape is the closure's arguments.

use super::{Exec, File, Provider, R, Run, Schema, Table, Ty, pos, s, strs};
use rustc_hash::{FxHashMap, FxHashSet};

impl Exec<'_> {
    /// The closure `node`'s rows whose column `col` (its source when `col` is the first after the keys, else its
    /// destination) holds one of `keys`, walked from those keys with the edges probed; None when its keys are not the
    /// file (the closure is then made as before).
    #[expect(clippy::too_many_arguments, reason = "a throwaway")]
    pub(super) fn walk_probed(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, col: usize, keys: &[u64], sc: &Schema) -> R<Option<Table>> {
        let a = &self.plan.nodes[node].j["args"];
        let nk = strs(&a["keys"])?.len();
        if col < nk || sc.tys[..nk].iter().any(|t| *t != Ty::File) {
            return Ok(None);
        }
        let forward = col == nk;
        let en = self.arg_rel(node, "edges")?;
        let esc = self.schema(run, en)?;
        let (si, di) = (pos(&esc.names, s(&a["source"])?)?, pos(&esc.names, s(&a["destination"])?)?);
        let (fc, tc) = if forward { (si, di) } else { (di, si) };
        let mut adj = f.walks.remove(&(node, forward)).unwrap_or_default();
        let mut frontier: Vec<u64> = {
            let mut seen = FxHashSet::default();
            keys.iter().copied().filter(|k| !adj.contains_key(k) && seen.insert(*k)).collect()
        };
        let mut read = 0u64;
        while !frontier.is_empty() {
            let t = self.probe(f, run, p, en, fc, &frontier)?;
            for k in &frontier {
                adj.entry(*k).or_default();
            }
            for r in 0..t.n {
                adj.entry(t.cols[fc][r]).or_default().push(t.cols[tc][r]);
            }
            read += t.n as u64;
            let mut seen = FxHashSet::default();
            frontier = t.cols[tc].iter().copied().filter(|x| !adj.contains_key(x) && seen.insert(*x)).collect();
        }
        let mut out = Table::with(sc.names.clone(), sc.tys.clone());
        let file = u64::from(f.file);
        let mut row: Vec<u64> = Vec::with_capacity(nk + 2);
        // redo-alloc: one walk's set and stack, emptied a key (not made a key)
        let mut seen: FxHashSet<u64> = FxHashSet::default();
        let mut stack: Vec<u64> = Vec::new();
        for k in keys {
            // a chain walk where every node on the way has one neighbour; else a visit with a set
            seen.clear();
            stack.clear();
            stack.push(*k);
            while let Some(x) = stack.pop() {
                let Some(ns) = adj.get(&x) else { continue };
                for y in ns.iter() {
                    if seen.insert(*y) {
                        row.clear();
                        row.extend(std::iter::repeat_n(file, nk));
                        row.extend(if forward { [*k, *y] } else { [*y, *k] });
                        out.push_row(&row);
                        stack.push(*y);
                    }
                }
            }
        }
        self.stats.touched[node] += read;
        f.walks.insert((node, forward), adj);
        let _: Option<FxHashMap<u8, u8>> = None;
        Ok(Some(out))
    }
}
