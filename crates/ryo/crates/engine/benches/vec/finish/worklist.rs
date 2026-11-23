//! FIFO fixpoint over a finite join-semilattice supplied in the query.
use super::{J, R, Schema, Table, Ty, pos, s, strs};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(super) fn schema(
    a: &J,
    roots: &Schema,
    edges: &Schema,
    lattice: &Schema,
) -> R<(Vec<String>, Vec<Ty>)> {
    let keys = strs(&a["keys"])?;
    let ty = |schema: &Schema, name: &str| -> R<Ty> { Ok(schema.tys[pos(&schema.names, name)?]) };
    let mut names: Vec<_> = keys.clone().map(str::to_owned).collect();
    let mut types = Vec::new();
    for k in keys {
        let t = ty(roots, k)?;
        if t != ty(edges, k)? {
            return Err(format!("worklist partition type of {k}"));
        }
        types.push(t);
    }
    let root = ty(roots, s(&a["root"])?)?;
    let state = ty(roots, s(&a["state"])?)?;
    for (input, col, expected) in [
        (edges, "source", root),
        (edges, "destination", root),
        (edges, "order", Ty::Num),
        (edges, "before", state),
        (edges, "after", state),
        (lattice, "lower", state),
        (lattice, "upper", state),
    ] {
        if ty(input, s(&a[col])?)? != expected {
            return Err(format!("worklist column type of {col}"));
        }
    }
    for col in ["root", "source", "destination", "order", "before", "output"] {
        names.push(s(&a[col])?.to_owned());
    }
    types.extend([root, root, root, Ty::Num, state, state]);
    Ok((names, types))
}

struct Lattice {
    bottom: u64,
    joins: BTreeMap<(u64, u64), u64>,
}

impl Lattice {
    fn read(t: &Table, lower: usize, upper: usize) -> R<Option<Self>> {
        let mut above = BTreeMap::<u64, BTreeSet<u64>>::new();
        for (&l, &u) in t.cols[lower].iter().zip(&t.cols[upper]) {
            above.entry(l).or_default().extend([l, u]);
            above.entry(u).or_default().insert(u);
        }
        if above.is_empty() {
            return Ok(None);
        }
        let states: Vec<_> = above.keys().copied().collect();
        for via in &states {
            let rest = above[via].clone();
            for supers in above.values_mut() {
                if supers.contains(via) {
                    supers.extend(&rest);
                }
            }
        }
        let least = |set: &BTreeSet<u64>| -> R<u64> {
            let mut candidates = set.iter().copied().filter(|v| set.is_subset(&above[v]));
            match (candidates.next(), candidates.next()) {
                (Some(v), None) => Ok(v),
                _ => Err("worklist lattice has no unique least upper bound".into()),
            }
        };
        let bottom = least(&states.iter().copied().collect())?;
        let mut joins = BTreeMap::new();
        for &left in &states {
            for &right in &states {
                let common = above[&left].intersection(&above[&right]).copied().collect();
                joins.insert((left, right), least(&common)?);
            }
        }
        Ok(Some(Self { bottom, joins }))
    }

    fn join(&self, left: u64, right: u64) -> R<u64> {
        self.joins
            .get(&(left, right))
            .copied()
            .ok_or_else(|| "worklist state outside lattice".into())
    }
}

pub(super) fn trace(
    a: &J,
    roots: &Table,
    edges: &Table,
    order: &Table,
    output: &mut Table,
) -> R<()> {
    let Some(lattice) = Lattice::read(
        order,
        order.col(s(&a["lower"])?)?,
        order.col(s(&a["upper"])?)?,
    )?
    else {
        return if roots.n == 0 && edges.n == 0 {
            Ok(())
        } else {
            Err("worklist states in empty lattice".into())
        };
    };
    let keys = strs(&a["keys"])?;
    let indices = |t: &Table| keys.clone().map(|k| t.col(k)).collect::<R<Vec<_>>>();
    let rk = indices(roots)?;
    let ek = indices(edges)?;
    let partition = |t: &Table, cols: &[usize], row: usize| {
        cols.iter().map(|&i| t.cols[i][row]).collect::<Vec<_>>()
    };
    let (entity, state) = (roots.col(s(&a["root"])?)?, roots.col(s(&a["state"])?)?);
    let col = |name| edges.col(s(&a[name])?);
    let (source, target, before, after, ordinal) = (
        col("source")?,
        col("destination")?,
        col("before")?,
        col("after")?,
        col("order")?,
    );
    let mut moves = BTreeMap::new();
    for i in 0..edges.n {
        let a = edges.cols[after][i];
        let b = edges.cols[before][i];
        lattice.join(a, b)?;
        let leaving = moves
            .entry((partition(edges, &ek, i), edges.cols[source][i], b))
            .or_insert_with(BTreeMap::new);
        if leaving
            .insert(edges.cols[ordinal][i] as i64, (edges.cols[target][i], a))
            .is_some()
        {
            return Err("duplicate worklist edge order".into());
        }
    }
    let mut starts = BTreeMap::new();
    for i in 0..roots.n {
        let held = starts
            .entry((partition(roots, &rk, i), roots.cols[entity][i]))
            .or_insert(lattice.bottom);
        *held = lattice.join(*held, roots.cols[state][i])?;
    }
    let mut seen = BTreeSet::new();
    for ((key, root), initial) in starts {
        let mut held = BTreeMap::from([(root, initial)]);
        let mut queue = VecDeque::from([root]);
        while let Some(src) = queue.pop_front() {
            let state = held[&src];
            if let Some(out) = moves.get(&(key.clone(), src, state)) {
                for (&rank, &(dst, after)) in out {
                    let old = held.entry(dst).or_insert(lattice.bottom);
                    let reached = lattice.join(*old, after)?;
                    if reached != *old {
                        *old = reached;
                        queue.push_back(dst);
                    }
                    let mut row = key.clone();
                    row.extend([root, src, dst, rank as u64, state, reached]);
                    if seen.insert(row.clone()) {
                        output.push_row(&row);
                    }
                }
            }
        }
    }
    Ok(())
}
