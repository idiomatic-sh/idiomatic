//! THROWAWAY (vec 6e-K, item 3): walks along the tree.
//! - An empty domain ends the step (exact: a domain of no node admits no relative).
//! - Over the tree itself (`semantic-child`), whose nodes are laid out in preorder, a domain smaller than the starts (exact
//!   counts) is read top-down: an admitted node `d` holds the starts in its subtree range `(d, end(d))`. Ancestors of a
//!   start are the domain ranges that hold it (nested ranges form a forest, walked from the last domain node before the
//!   start); descendants are the domain nodes inside the start's own range (a binary search).
//! - An axis the planner marked `exists` (`band/scripts/pass_axis.py`: `related` unread, multiplicity unseen) keeps a start
//!   once, at its first admitted relative: going up, the nearest; going down, the first in preorder.
//!
//! No rule id: the shape is the axis's own arguments.

use super::{Dom, Exec, File, Hier, NULL, Provider, R, Run, Table, bucket, bucket_len, hier_step, kind_set, node_table, s, view_holds};
use ryo::languages::Position;
use serde_json::Value as J;

impl Exec<'_> {
    /// The axis `node` over `from` in one of the forms above, or None (the axis runs as before).
    pub(super) fn walk_axis(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, a: &J, from: &Table) -> R<Option<Table>> {
        if sw!("VEC_NO_AXISWALK") {
            return Ok(None);
        }
        let dir = s(&a["direction"])?.to_owned();
        if !matches!(dir.as_str(), "ancestor" | "descendant" | "nearest-ancestor") {
            return Ok(None);
        }
        let exists = self.plan.nodes[node].j.get("exists").is_some();
        let dom = self.dom(f, run, p, node)?;
        let size = match &dom {
            Dom::Set(x) => Some(x.len()),
            Dom::View(v, _) => kind_set(run, &v.filter, 0, v.base, &[]).map(|ks| bucket_len(f, run, &ks)),
            _ => None,
        };
        if size == Some(0) {
            *self.tally.back.entry((node, "axis: empty domain")).or_default() += 1;
            return Ok(Some(self.empty(run, node)?));
        }
        let rel = self.arg_rel(node, "relation")?;
        let hier = self.hier(f, run, p, rel)?;
        let tree = matches!(hier, Hier::Tree);
        let ic = from.col("id")?;
        let mut out = self.empty(run, node)?;
        let mut row = Vec::new();
        if tree && size.is_some_and(|n| n < from.n) {
            // the domain's nodes, sorted (preorder), with their subtree ends
            let mut d: Vec<u32> = match &dom {
                Dom::Set(x) => x.iter().filter(|v| (**v as usize) < f.n).map(|v| *v as u32).collect(),
                Dom::View(v, _) => {
                    let ks = kind_set(run, &v.filter, 0, v.base, &[]).ok_or("a sized view domain with no kinds")?;
                    bucket(f, run, &ks).into_iter().filter(|r| view_holds(f, run, v, *r)).collect()
                }
                _ => unreachable!(),
            };
            d.sort_unstable();
            d.dedup();
            let end: Vec<u32> = d.iter().map(|x| node_table::subtree_end_of(f.table, Position(*x)).0).collect();
            // the domain forest: each domain node's nearest domain ancestor (by range), or none
            let mut up: Vec<u32> = vec![u32::MAX; d.len()];
            let mut stack: Vec<u32> = Vec::new();
            for i in 0..d.len() {
                while let Some(&t) = stack.last() {
                    if d[i] < end[t as usize] {
                        break;
                    }
                    stack.pop();
                }
                up[i] = stack.last().copied().unwrap_or(u32::MAX);
                stack.push(i as u32);
            }
            self.stats.touched[node] += d.len() as u64;
            for r in 0..from.n {
                let x = from.cols[ic][r];
                if x == NULL || x as usize >= f.n {
                    continue;
                }
                let x = x as u32;
                self.stats.touched[node] += 1;
                if dir == "descendant" {
                    let xe = node_table::subtree_end_of(f.table, Position(x)).0;
                    let lo = d.partition_point(|v| *v <= x);
                    let hi = d.partition_point(|v| *v < xe);
                    for q in &d[lo..hi.max(lo)] {
                        from.row(r, &mut row);
                        row.push(u64::from(*q));
                        out.push_row(&row);
                        if exists {
                            break;
                        }
                    }
                    continue;
                }
                // ancestors: from the last domain node before x, up the forest to the first range that holds x
                let at = d.partition_point(|v| *v < x);
                if at == 0 {
                    continue;
                }
                let mut i = (at - 1) as u32;
                while i != u32::MAX && end[i as usize] <= x {
                    i = up[i as usize];
                }
                while i != u32::MAX {
                    from.row(r, &mut row);
                    row.push(u64::from(d[i as usize]));
                    out.push_row(&row);
                    if exists || dir == "nearest-ancestor" {
                        break;
                    }
                    i = up[i as usize];
                }
            }
            *self.tally.back.entry((node, "axis: top-down")).or_default() += 1;
            return Ok(Some(out));
        }
        if !exists || dir == "nearest-ancestor" {
            return Ok(None);
        }
        if dir == "ancestor" {
            // the first admitted ancestor: level by level for all starts at once, the domain asked once a level
            let mut found: Vec<u64> = vec![NULL; from.n];
            let mut frontier: Vec<(usize, u64)> = (0..from.n).map(|i| (i, from.cols[ic][i])).collect();
            let mut nbrs = Vec::new();
            for _ in 0..4096 {
                if frontier.is_empty() {
                    break;
                }
                let mut next: Vec<(usize, u64)> = Vec::new();
                for (i, x) in &frontier {
                    hier_step(f, run, &hier, *x, true, &mut nbrs);
                    next.extend(nbrs.iter().map(|q| (*i, *q)));
                }
                self.stats.touched[node] += next.len() as u64;
                let qs: Vec<u64> = next.iter().map(|x| x.1).collect();
                let ok = self.admit(f, run, p, &dom, &qs)?;
                frontier.clear();
                for (i, q) in next {
                    if found[i] != NULL {
                        continue;
                    }
                    if ok.contains(&q) {
                        found[i] = q;
                    } else {
                        frontier.push((i, q));
                    }
                }
                frontier.retain(|(i, _)| found[*i] == NULL);
            }
            for (i, q) in found.iter().enumerate() {
                if *q != NULL {
                    from.row(i, &mut row);
                    row.push(*q);
                    out.push_row(&row);
                }
            }
            *self.tally.back.entry((node, "axis: first ancestor")).or_default() += 1;
            return Ok(Some(out));
        }
        // the first admitted descendant in preorder, over the tree with a domain read per node
        if !tree {
            return Ok(None);
        }
        for r in 0..from.n {
            let x = from.cols[ic][r];
            if x == NULL || x as usize >= f.n {
                continue;
            }
            let xe = node_table::subtree_end_of(f.table, Position(x as u32)).0;
            for q in x as u32 + 1..xe {
                self.stats.touched[node] += 1;
                if !self.admit(f, run, p, &dom, &[u64::from(q)])?.is_empty() {
                    from.row(r, &mut row);
                    row.push(u64::from(q));
                    out.push_row(&row);
                    break;
                }
            }
        }
        *self.tally.back.entry((node, "axis: first descendant")).or_default() += 1;
        Ok(Some(out))
    }
}
