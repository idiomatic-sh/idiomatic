//! THROWAWAY (redo-binding, ryo-fmx3.1): `subtree-class`, the structural class of the subtree under each row's node.
//!
//! `subtree-class(from=R, relation=E, labels=L, leaves=S, id="id", parent="parent", child="child", label=[..], leaf-label=[..],
//! step=[..], output="class")`: the rows of `R` (node column `id`, which names the node column of `L` and `S` too) with a number column `output`; two rows of one call have equal numbers exactly when their subtrees are
//! equal as trees. A node of `S` is a leaf: its `leaf-label` columns are its label and its children are not visited.
//! Every other node has its `label` columns from its row of `L`, and its children are the `child` column of the rows of
//! `E` whose `parent` column is the node, each named by its `step` columns. Two subtrees are equal when their roots have equal
//! labels (a leaf never equals a node) and their children pair one to one by step, each pair equal. A node with no row in
//! `L` or `S`, or with a child that has none, has no class, so its row is dropped. The classes are interned once a call
//! (hash-consing, never a hash compare): equal numbers are equal trees, with no collision. No rule id: the shape is the
//! arguments. Each subtree node is visited once a call, its edges and labels asked a level at a time by key.

use super::{Exec, File, Provider, R, Run, Table, s, strs};
use rustc_hash::FxHashMap;

impl Exec<'_> {
    pub fn subtree_class(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Table> {
        let a = self.plan.nodes[node].args();
        let from = self.eval(f, run, p, self.arg_rel(node, "from")?)?;
        let mut out = self.empty(run, node)?;
        if from.n == 0 {
            return Ok(out);
        }
        let ic = from.col(s(&a["id"])?)?;
        let (en, ln) = (self.arg_rel(node, "relation")?, self.arg_rel(node, "labels")?);
        let sn = self.arg_rel(node, "leaves").ok();
        let label: Vec<String> = strs(&a["label"])?.map(str::to_owned).collect();
        let leaf_label: Vec<String> = if sn.is_some() { strs(&a["leaf-label"])?.map(str::to_owned).collect() } else { Vec::new() };
        let step: Vec<String> = strs(&a["step"])?.map(str::to_owned).collect();
        let _ = s(&a["output"])?;
        let (esc, lsc) = (self.schema(run, en)?, self.schema(run, ln)?);
        let pos = |names: &[String], c: &str| names.iter().position(|x| x == c).ok_or_else(|| format!("subtree-class: no column {c}"));
        // redo-binding pass 6: the edge columns and the label tables' node column are the step's arguments
        let (parent_col, child_col, id_col) = (s(&a["parent"])?, s(&a["child"])?, s(&a["id"])?);
        let (epc, ecc) = (pos(&esc.names, parent_col)?, pos(&esc.names, child_col)?);
        let esteps: Vec<usize> = step.iter().map(|c| pos(&esc.names, c)).collect::<R<_>>()?;
        let lic = pos(&lsc.names, id_col)?;
        let lcols: Vec<usize> = label.iter().map(|c| pos(&lsc.names, c)).collect::<R<_>>()?;
        let (sic, scols) = match sn {
            Some(sn) => {
                let ssc = self.schema(run, sn)?;
                (pos(&ssc.names, id_col)?, leaf_label.iter().map(|c| pos(&ssc.names, c)).collect::<R<Vec<usize>>>()?)
            }
            None => (0, Vec::new()),
        };
        // Visit the subtrees a level at a time: each node's key (label, then its children by step) once a call.
        // keys[n] = Some((leaf?, label, children (step.., child))) ; None = no label.
        let mut info: FxHashMap<u64, Option<(bool, Vec<u64>, Vec<(Vec<u64>, u64)>)>> = FxHashMap::default();
        let mut order: Vec<u64> = Vec::new();
        let mut frontier: Vec<u64> = from.cols[ic].clone();
        frontier.sort_unstable();
        frontier.dedup();
        while !frontier.is_empty() {
            frontier.retain(|x| !info.contains_key(x));
            if frontier.is_empty() {
                break;
            }
            let mut leaf_rows: FxHashMap<u64, Vec<u64>> = FxHashMap::default();
            if let Some(sn) = sn {
                let t = self.probe(f, run, p, sn, sic, &frontier)?;
                for r in 0..t.n {
                    leaf_rows.entry(t.cols[sic][r]).or_insert_with(|| scols.iter().map(|c| t.cols[*c][r]).collect());
                }
            }
            let inner: Vec<u64> = frontier.iter().copied().filter(|x| !leaf_rows.contains_key(x)).collect();
            let lt = self.probe(f, run, p, ln, lic, &inner)?;
            let mut labels: FxHashMap<u64, Vec<u64>> = FxHashMap::default();
            for r in 0..lt.n {
                labels.entry(lt.cols[lic][r]).or_insert_with(|| lcols.iter().map(|c| lt.cols[*c][r]).collect());
            }
            let has_label: Vec<u64> = inner.iter().copied().filter(|x| labels.contains_key(x)).collect();
            let et = self.probe(f, run, p, en, epc, &has_label)?;
            let mut kids: FxHashMap<u64, Vec<(Vec<u64>, u64)>> = FxHashMap::default();
            for r in 0..et.n {
                kids.entry(et.cols[epc][r]).or_default().push((esteps.iter().map(|c| et.cols[*c][r]).collect(), et.cols[ecc][r]));
            }
            let mut next = Vec::new();
            for x in frontier.drain(..) {
                order.push(x);
                if let Some(l) = leaf_rows.remove(&x) {
                    info.insert(x, Some((true, l, Vec::new())));
                } else if let Some(l) = labels.remove(&x) {
                    let mut ks = kids.remove(&x).unwrap_or_default();
                    ks.sort_unstable();
                    next.extend(ks.iter().map(|k| k.1));
                    info.insert(x, Some((false, l, ks)));
                } else {
                    info.insert(x, None);
                }
            }
            next.sort_unstable();
            next.dedup();
            frontier = next;
        }
        // Classes bottom-up, children before their parent (an explicit post-order: a root may lie under another root).
        let mut intern: FxHashMap<Vec<u64>, u64> = FxHashMap::default();
        let mut class: FxHashMap<u64, Option<u64>> = FxHashMap::default();
        let mut key = Vec::new();
        for start in &order {
            let mut stack: Vec<(u64, bool)> = vec![(*start, false)];
            while let Some((x, expanded)) = stack.pop() {
                if class.contains_key(&x) {
                    continue;
                }
                let Some(Some((leaf, l, ks))) = info.get(&x) else {
                    class.insert(x, None);
                    continue;
                };
                if !expanded {
                    stack.push((x, true));
                    stack.extend(ks.iter().filter(|k| !class.contains_key(&k.1)).map(|k| (k.1, false)));
                    continue;
                }
                key.clear();
                key.push(u64::from(*leaf));
                key.push(l.len() as u64);
                key.extend_from_slice(l);
                let mut whole = true;
                for (st, c) in ks {
                    match class.get(c).copied().flatten() {
                        Some(cc) => {
                            key.extend_from_slice(st);
                            key.push(cc);
                        }
                        None => {
                            whole = false;
                            break;
                        }
                    }
                }
                let id = if whole {
                    let n = intern.len() as u64;
                    Some(*intern.entry(key.clone()).or_insert(n))
                } else {
                    None
                };
                class.insert(x, id);
            }
        }
        let mut row = Vec::new();
        for r in 0..from.n {
            if let Some(Some(c)) = class.get(&from.cols[ic][r]) {
                from.row(r, &mut row);
                row.push(*c);
                out.push_row(&row);
            }
        }
        Ok(out)
    }
}
