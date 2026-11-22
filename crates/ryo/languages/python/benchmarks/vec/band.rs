//! THROWAWAY (vec 6e-K, item 1): a two-source select the planner marked `semi` (see `band/scripts/pass_semi.py`): its
//! items read the kept source only and no reader can tell a row from its copies, so it keeps each kept row once when a
//! probed row passes its tests. The probed rows are laid out once, sorted by (equality keys, band column); a kept row
//! finds its key's run by hash and the rows inside its band by binary search, and stops at the first that passes every
//! other test. The pairs are never made. No rule id: the shape is read from the select's own tests.

use super::{E, Exec, bucket_len, File, NULL, Provider, R, Run, Src, Table, Ty, V, bucket, eval, kind_set, key_of, refs_of, truthy, view_rows, view_holds};
use rustc_hash::FxHashMap;

/// A select's tests, split for the semijoin.
pub(super) struct Shape {
    /// Tests of the kept source alone.
    pub(super) kept: Vec<E>,
    /// Tests of the probed source alone.
    pub(super) probed: Vec<E>,
    /// (probed column, kept expression) equalities.
    pub(super) eqs: Vec<(usize, E)>,
    /// The band column of the probed source, and its bounds: (op with the probed column on the left, kept expression).
    pub(super) band: Option<(usize, Vec<(String, E)>)>,
    /// Every other test.
    pub(super) rest: Vec<E>,
}

fn flip(o: &str) -> &'static str {
    match o {
        "<" => ">",
        "<=" => ">=",
        ">" => "<",
        ">=" => "<=",
        _ => "=",
    }
}

/// The tests of a two-source select split by what they read; `pb` is the probed source, `ty` its column types.
pub(super) fn shape(conj: &[E], pb: usize, ty: &[Ty]) -> Shape {
    let (kb, pbit) = (1u64 << (1 - pb), 1u64 << pb);
    let mut s = Shape { kept: Vec::new(), probed: Vec::new(), eqs: Vec::new(), band: None, rest: Vec::new() };
    for c in conj {
        let r = refs_of(c);
        if r & !kb == 0 {
            s.kept.push(c.clone());
            continue;
        }
        if r == pbit {
            s.probed.push(c.clone());
            continue;
        }
        if let E::Cmp(o, x, y) = c {
            let sides = [(&**x, &**y, o.as_str()), (&**y, &**x, flip(o))];
            let mut placed = false;
            for (a, b, op) in sides {
                let E::Col(si, col, _) = *a else { continue };
                if si != pb || refs_of(b) != kb {
                    continue;
                }
                if op == "=" && o == "=" {
                    s.eqs.push((col, b.clone()));
                    placed = true;
                    break;
                }
                if matches!(op, "<" | "<=" | ">" | ">=") && ty[col] == Ty::Num {
                    match &mut s.band {
                        None => {
                            s.band = Some((col, vec![(op.to_owned(), b.clone())]));
                            placed = true;
                        }
                        Some((bc, bs)) if *bc == col => {
                            bs.push((op.to_owned(), b.clone()));
                            placed = true;
                        }
                        Some(_) => {}
                    }
                    if placed {
                        break;
                    }
                }
            }
            if placed {
                continue;
            }
        }
        s.rest.push(c.clone());
    }
    s
}

/// Counts of the sites the executor ran as a semijoin and those it handed back, by reason (printed at the end of a run).
#[derive(Default)]
pub(crate) struct Tally {
    pub(crate) ran: FxHashMap<usize, u64>,
    pub(crate) back: FxHashMap<(usize, &'static str), u64>,
}

impl Exec<'_> {
    /// The semijoin of a select marked `semi`, or None when this file's sources do not allow it (the select runs as it is).
    pub(super) fn semi(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, pb: usize) -> R<Option<Table>> {
        Ok(self.band_rows(f, run, p, node, pb, false)?.map(|x| x.0))
    }

    /// The kept rows of a select marked `semi` (each once, its items) and, when `count`, how many probed rows each pairs
    /// with: a run length by binary search where no other test is left, else a scan of its band. Without `count`, only
    /// kept rows with a partner, the scan stopping at the first.
    pub(super) fn band_rows(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, pb: usize, count: bool) -> R<Option<(Table, Vec<u64>)>> {
        let sp = self.sel(run, node)?;
        if super::filter_order::file_fails(run, f, &sp.conj) {
            return Ok(Some((self.empty(run, node)?, Vec::new())));
        }
        let ka = 1 - pb;
        let (an, bn) = (sp.srcs[ka], sp.srcs[pb]);
        let bty = self.schema(run, bn)?.tys.clone();
        let sh = shape(&sp.conj, pb, &bty);
        if !count && sh.band.is_none() && !sw!("VEC_SEMI_EQ") {
            *self.tally.back.entry((node, "no band")).or_default() += 1;
            return Ok(None);
        }
        let mut srcs: Vec<Src> = vec![Src::Pending(usize::MAX), Src::Pending(usize::MAX)];
        let sa = self.src_of(f, run, p, an)?;
        match &sa {
            Src::Tab(t, _) if t.n == 0 => return Ok(Some((self.empty(run, node)?, Vec::new()))),
            Src::Tab(..) | Src::View(..) => {}
            _ => {
                *self.tally.back.entry((node, "kept side keyed or a closure")).or_default() += 1;
                return Ok(None);
            }
        }
        // the kept rows: a table's, or a view's (entered by the select's kind tests when the view names none)
        let arows: Vec<u32> = match &sa {
            Src::Tab(t, _) => (0..t.n as u32).collect(),
            Src::View(v, vn) => match kind_set(run, &sp.conj, ka, v.base, &v.map) {
                Some(ks) if kind_set(run, &v.filter, 0, v.base, &[]).is_none() => bucket(f, run, &ks).into_iter().filter(|r| view_holds(f, run, v, *r)).collect(),
                _ => (*view_rows(f, run, v, *vn)).clone(),
            },
            _ => unreachable!(),
        };
        srcs[ka] = sa;
        self.stats.touched[node] += arows.len() as u64;
        let mut cur = vec![0u32; 2];
        let arows: Vec<u32> = arows
            .into_iter()
            .filter(|r| {
                cur[ka] = *r;
                sh.kept.iter().all(|c| truthy(eval(run, f, &srcs, &cur, c)))
            })
            .collect();
        if arows.is_empty() {
            return Ok(Some((self.empty(run, node)?, Vec::new())));
        }
        // the probed rows: a table's or a view's whole, or (a step not made yet) probed by the kept rows' first key
        let sb = self.src_of(f, run, p, bn)?;
        let sb = match sb {
            Src::Pending(n) if !sh.eqs.is_empty() => {
                let (c0, e0) = &sh.eqs[0];
                let mut ks = rustc_hash::FxHashSet::default();
                for r in &arows {
                    cur[ka] = *r;
                    ks.insert(eval(run, f, &srcs, &cur, e0).1);
                }
                let ks: Vec<u64> = ks.into_iter().collect();
                Src::Tab(self.probe(f, run, p, n, *c0, &ks)?, usize::MAX - pb)
            }
            Src::Pending(n) => Src::Tab(self.eval(f, run, p, n)?, n),
            Src::Tab(..) => sb,
            Src::View(v, vn) => match kind_set(run, &sp.conj, pb, v.base, &v.map) {
                Some(ks) if kind_set(run, &v.filter, 0, v.base, &[]).is_none() => {
                    let rows: Vec<u32> = bucket(f, run, &ks).into_iter().filter(|r| view_holds(f, run, &v, *r)).collect();
                    let sc = self.schema(run, bn)?;
                    let mut t = Table::with(sc.names.clone(), sc.tys.clone());
                    for c in 0..v.map.len() {
                        t.cols[c] = rows.iter().map(|r| f.native_get(run, v.base, *r, v.map[c])).collect();
                    }
                    t.n = rows.len();
                    Src::Tab(std::rc::Rc::new(t), usize::MAX - pb)
                }
                _ => Src::View(v, vn),
            },
            Src::Lazy(_) | Src::Base(_) => {
                *self.tally.back.entry((node, "probed side a closure")).or_default() += 1;
                return Ok(None);
            }
        };
        let brows: Vec<u32> = match &sb {
            Src::Tab(t, _) => (0..t.n as u32).collect(),
            Src::View(v, vn) => (*view_rows(f, run, v, *vn)).clone(),
            _ => unreachable!(),
        };
        srcs[pb] = sb;
        self.stats.touched[node] += brows.len() as u64;
        // the layout: probed rows that pass their own tests, sorted by (key, band value); runs by key
        let bc = sh.band.as_ref().map(|b| b.0);
        let mut lay: Vec<(u64, i64, u32)> = Vec::with_capacity(brows.len());
        for r in brows {
            cur[pb] = r;
            if !sh.probed.iter().all(|c| truthy(eval(run, f, &srcs, &cur, c))) {
                continue;
            }
            let key = key_of(sh.eqs.iter().map(|(c, _)| super::get(run, f, &srcs, &cur, pb, *c)));
            let o = match bc {
                Some(c) => {
                    let v = super::get(run, f, &srcs, &cur, pb, c);
                    if v == NULL {
                        continue;
                    }
                    v as i64
                }
                None => 0,
            };
            lay.push((key, o, r));
        }
        lay.sort_unstable();
        let mut runs: FxHashMap<u64, (u32, u32)> = FxHashMap::default();
        let mut i = 0;
        while i < lay.len() {
            let mut j = i;
            while j < lay.len() && lay[j].0 == lay[i].0 {
                j += 1;
            }
            runs.insert(lay[i].0, (i as u32, j as u32));
            i = j;
        }
        // multi-column keys are hashed: their equalities are tested again on each candidate
        let recheck: Vec<E> = if sh.eqs.len() > 1 {
            sh.eqs.iter().map(|(c, e)| E::Cmp("=".to_owned(), Box::new(E::Col(pb, *c, bty[*c])), Box::new(e.clone()))).collect()
        } else {
            Vec::new()
        };
        let mut keep: Vec<u32> = Vec::new();
        let mut counts: Vec<u64> = Vec::new();
        let mut scanned = 0u64;
        for &r in &arows {
            cur[ka] = r;
            let key = key_of(sh.eqs.iter().map(|(_, e)| eval(run, f, &srcs, &cur, e).1));
            let Some(&(s0, s1)) = runs.get(&key) else { continue };
            let mut run_rows = &lay[s0 as usize..s1 as usize];
            let mut later: Vec<&E> = Vec::new();
            if let Some((_, bounds)) = &sh.band {
                let (mut lo, mut hi) = (i64::MIN, i64::MAX);
                let mut none = false;
                for (op, e) in bounds {
                    let v = eval(run, f, &srcs, &cur, e);
                    if v.1 == NULL {
                        none = true;
                        break;
                    }
                    if v.ty() != Ty::Num {
                        later.push(e);
                        continue;
                    }
                    let x = v.1 as i64;
                    match op.as_str() {
                        ">" => lo = lo.max(x.saturating_add(1)),
                        ">=" => lo = lo.max(x),
                        "<" => hi = hi.min(x.saturating_sub(1)),
                        _ => hi = hi.min(x),
                    }
                }
                if none || lo > hi {
                    continue;
                }
                let a0 = run_rows.partition_point(|t| t.1 < lo);
                let a1 = run_rows.partition_point(|t| t.1 <= hi);
                run_rows = &run_rows[a0..a1.max(a0)];
            }
            let band_terms: Vec<E> = if later.is_empty() {
                Vec::new()
            } else {
                sh.band.as_ref().map(|(c, bs)| bs.iter().map(|(op, e)| E::Cmp(op.clone(), Box::new(E::Col(pb, *c, Ty::Num)), Box::new(e.clone()))).collect()).unwrap_or_default()
            };
            if count && recheck.is_empty() && band_terms.is_empty() && sh.rest.is_empty() {
                if !run_rows.is_empty() {
                    keep.push(r);
                    counts.push(run_rows.len() as u64);
                }
                scanned += 1;
                continue;
            }
            let mut m = 0u64;
            for t in run_rows {
                scanned += 1;
                cur[pb] = t.2;
                if recheck.iter().chain(&band_terms).chain(&sh.rest).all(|c| truthy(eval(run, f, &srcs, &cur, c))) {
                    m += 1;
                    if !count {
                        break;
                    }
                }
            }
            if m > 0 {
                keep.push(r);
                counts.push(m);
            }
        }
        self.stats.touched[node] += scanned;
        let mut out = self.empty(run, node)?;
        for (c, (_, e)) in sp.items.iter().enumerate() {
            out.cols[c] = keep
                .iter()
                .map(|r| {
                    cur[ka] = *r;
                    eval(run, f, &srcs, &cur, e).1
                })
                .collect();
        }
        out.n = keep.len();
        *self.tally.ran.entry(node).or_default() += 1;
        let _ = V::new;
        Ok(Some((out, counts)))
    }

    /// Whether a select marked `semi`, asked by `asked` keys of its kept rows, is cheaper made whole: its probed source's
    /// entry (a made table's rows or a kind bucket, exact counts) is smaller than the keys asked, so the whole select
    /// drives from it and asks the kept rows by its own keys.
    pub(super) fn semi_drives_whole(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, asked: usize) -> R<bool> {
        let j = &self.plan.nodes[node].j;
        let Some(pb) = j.get("semi").and_then(serde_json::Value::as_u64) else { return Ok(false) };
        if j.get("count").is_some() || sw!("VEC_NO_SEMI") {
            return Ok(false);
        }
        let sp = self.sel(run, node)?;
        let pb = pb as usize;
        let size = match self.src_of(f, run, p, sp.srcs[pb])? {
            Src::Tab(t, _) => Some(t.n),
            Src::View(v, _) => kind_set(run, &v.filter, 0, v.base, &[]).or_else(|| kind_set(run, &sp.conj, pb, v.base, &v.map)).map(|ks| bucket_len(f, run, &ks)),
            _ => None,
        };
        let whole = size.is_some_and(|n| n < asked);
        if whole {
            *self.tally.back.entry((node, "probed by more keys than its probed side: made whole")).or_default() += 1;
        }
        Ok(whole)
    }
}
