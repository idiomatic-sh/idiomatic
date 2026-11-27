//! THROWAWAY (redo-honest): the `ordered-dfs` operation, with the shipping engine's meaning (operators `ordered_dfs` and the
//! `ordered_walk` kernel): from each distinct (keys, root) of `from`, an ordered depth-first visit of `edges`: a source's
//! edges in ascending order of their order column, each edge visited once a root (a destination already met is not
//! entered again), one output row an edge with its ordinal among the root's visits. Output columns: the keys, the root,
//! the edge's source, destination and edge columns, then the ordinal. Two edges of one source with one order are refused,
//! as the shipping engine refuses them (`DuplicateOrder`).

use super::{Args, NULL, R, Table, Ty, s, strs};
use rustc_hash::{FxHashMap, FxHashSet};

/// The output's column names and types from the two inputs' schemas.
pub(crate) fn schema(a: &Args, from: (&[String], &[Ty]), edges: (&[String], &[Ty])) -> R<(Vec<String>, Vec<Ty>)> {
    let col = |names: &[String], c: &str| names.iter().position(|n| n == c).ok_or_else(|| format!("ordered-dfs: no column {c}"));
    let mut names = Vec::new();
    let mut tys = Vec::new();
    for k in strs(&a["keys"])? {
        names.push(k.to_owned());
        tys.push(from.1[col(from.0, k)?]);
    }
    let root = s(&a["root"])?;
    names.push(root.to_owned());
    tys.push(from.1[col(from.0, root)?]);
    for c in [s(&a["source"])?, s(&a["destination"])?, s(&a["edge"])?] {
        names.push(c.to_owned());
        tys.push(edges.1[col(edges.0, c)?]);
    }
    names.push(s(&a["output"])?.to_owned());
    tys.push(Ty::Num);
    Ok((names, tys))
}

/// The visit rows into `out` (made with `schema`'s columns).
pub(crate) fn walked(a: &Args, from: &Table, edges: &Table, out: &mut Table) -> R<()> {
    let keys = strs(&a["keys"])?;
    let fk: Vec<usize> = keys.clone().map(|k| from.col(k)).collect::<R<_>>()?;
    let ek: Vec<usize> = keys.map(|k| edges.col(k)).collect::<R<_>>()?;
    let root = from.col(s(&a["root"])?)?;
    let (src, dst, ord, edge) =
        (edges.col(s(&a["source"])?)?, edges.col(s(&a["destination"])?)?, edges.col(s(&a["order"])?)?, edges.col(s(&a["edge"])?)?);
    // each (keys, source)'s edges, ascending by order (a number cell: its bits as a signed integer)
    let mut runs: FxHashMap<Vec<u64>, Vec<usize>> = FxHashMap::default();
    for r in 0..edges.n {
        if edges.cols[ord][r] == NULL {
            return Err("ordered-dfs: an edge with no order".to_owned().into());
        }
        let mut k: Vec<u64> = ek.iter().map(|c| edges.cols[*c][r]).collect();
        k.push(edges.cols[src][r]);
        runs.entry(k).or_default().push(r);
    }
    for v in runs.values_mut() {
        v.sort_unstable_by_key(|r| edges.cols[ord][*r] as i64);
        if v.windows(2).any(|w| edges.cols[ord][w[0]] == edges.cols[ord][w[1]]) {
            return Err("ordered-dfs: two edges of one source with one order".to_owned().into());
        }
    }
    let mut done: FxHashSet<Vec<u64>> = FxHashSet::default();
    let mut row = Vec::with_capacity(out.names.len());
    for r in 0..from.n {
        let mut k: Vec<u64> = fk.iter().map(|c| from.cols[*c][r]).collect();
        let start = from.cols[root][r];
        k.push(start);
        if !done.insert(k.clone()) {
            continue;
        }
        let kp = k[..k.len() - 1].to_vec();
        let mut met: FxHashSet<u64> = FxHashSet::default();
        met.insert(start);
        // frames: (the run's edges, the next place in it)
        let mut frames: Vec<(&[usize], usize)> = Vec::new();
        if let Some(v) = runs.get(&k) {
            frames.push((v, 0));
        }
        let mut ordinal = 0i64;
        let mut key = kp.clone();
        while let Some((v, next)) = frames.last_mut() {
            if *next >= v.len() {
                frames.pop();
                continue;
            }
            let e = v[*next];
            *next += 1;
            row.clear();
            row.extend_from_slice(&k);
            row.extend([edges.cols[src][e], edges.cols[dst][e], edges.cols[edge][e], ordinal as u64]);
            out.push_row(&row);
            ordinal += 1;
            let d = edges.cols[dst][e];
            if met.insert(d) {
                key.truncate(kp.len());
                key.push(d);
                if let Some(w) = runs.get(&key) {
                    frames.push((w, 0));
                }
            }
        }
    }
    Ok(())
}
