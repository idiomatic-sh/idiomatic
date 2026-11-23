//! Whole two-table equijoins. The plan binds both orientations. Row counts and the
//! existing keyed-reader policy still determine which orientation a file executes.
use super::super::{
    E, Exec, File, Index, J, NULL, Op, Provider, R, Res, Run, Schema, Table, index_of, key_of,
    keyed::Form, recycle, table_index,
};
use super::Kernels;
use std::rc::Rc;

#[derive(Clone, Copy)]
struct Inputs {
    nodes: [usize; 2],
    // For each possible deferred side, the first equality it can answer by key.
    deferred: [Option<[usize; 2]>; 2],
    semi_without_band: bool,
}

pub(super) struct Join {
    node: usize,
    inputs: Inputs,
    keys: [Box<[usize]>; 2],
    picks: Box<[(usize, usize)]>,
    pub(super) schema: Rc<Schema>,
}

#[derive(Default)]
pub(super) struct Work {
    keys: Vec<u64>,
    driver: Vec<u32>,
    matches: Vec<u32>,
}

fn equalities_only(expr: &J) -> bool {
    match expr[0].as_str() {
        Some("and") => equalities_only(&expr[1]) && equalities_only(&expr[2]),
        Some("cmp") => {
            expr[1] == "=" && expr[2][0] == "col" && expr[3][0] == "col" && expr[2][2] != expr[3][2]
        }
        _ => false,
    }
}

impl Join {
    fn bind(ex: &mut Exec<'_>, run: &mut Run, node: usize) -> R<Option<Self>> {
        let pn = &ex.plan.nodes[node];
        let j = &pn.j;
        let Some([left, right]) = j["src"].as_array().map(Vec::as_slice) else {
            return Ok(None);
        };
        if pn.op != "select" || !j["count"].is_null() || !equalities_only(&j["cond"]) {
            return Ok(None);
        }
        let semi = j["semi"].is_u64() && !sw!("VEC_NO_SEMI");
        if semi && (sw!("VEC_SEMI_EQ") || sw!("VEC_KSEL_NO_SEMI")) {
            return Ok(None);
        }
        let (Some(a), Some(b)) = (left[0].as_u64(), right[0].as_u64()) else {
            return Ok(None);
        };
        let nodes = [a as usize, b as usize];
        for &source in &nodes {
            let Some(src) = ex.plan.nodes.get(source) else {
                return Ok(None);
            };
            if matches!(src.op.as_str(), "closure" | "base" | "rows")
                || ex.view(run, source)?.is_some()
            {
                return Ok(None);
            }
        }
        let sel = ex.sel(run, node)?;
        let mut keys = [Vec::new(), Vec::new()];
        for expr in &sel.conj {
            let E::Cmp(Op::Eq, a, b) = expr else {
                return Ok(None);
            };
            let (E::Col(sa, ca, ta), E::Col(sb, cb, tb)) = (&**a, &**b) else {
                return Ok(None);
            };
            if sa == sb || ta != tb || keys[*sa].contains(ca) || keys[*sb].contains(cb) {
                return Ok(None);
            }
            keys[*sa].push(*ca);
            keys[*sb].push(*cb);
        }
        let Some(picks) = sel
            .items
            .iter()
            .map(|(_, e)| match e {
                E::Col(s, c, _) => Some((*s, *c)),
                _ => None,
            })
            .collect::<Option<Box<[_]>>>()
        else {
            return Ok(None);
        };
        let mut deferred = [None; 2];
        if nodes[0] != nodes[1] && !sw!("VEC_NO_KEYED_JOIN") && !keys[0].is_empty() {
            for side in 0..2 {
                let form = ex.keyed_form(run, nodes[side])?;
                if let Some(i) = keys[side].iter().position(|&c| match &form {
                    Form::Not => false,
                    Form::Select(cols) => cols.get(c).is_some_and(Option::is_some),
                    Form::Antijoin | Form::Union => true,
                }) {
                    deferred[side] = Some([keys[side][i], keys[1 - side][i]]);
                }
            }
        }
        Ok(Some(Self {
            node,
            inputs: Inputs {
                nodes,
                deferred,
                semi_without_band: semi,
            },
            keys: keys.map(Vec::into_boxed_slice),
            picks,
            schema: ex.schema(run, node)?,
        }))
    }

    fn gather(&self, tables: [&Table; 2], driver: usize, work: &Work) -> Table {
        let n = work.driver.len();
        let mut out = Table::with_rows(self.schema.names.clone(), self.schema.tys.clone(), n);
        for (&(side, column), dst) in self.picks.iter().zip(&mut out.cols) {
            let src = &tables[side].cols[column];
            let rows = if side == driver {
                &work.driver
            } else {
                &work.matches
            };
            dst.extend(rows.iter().map(|&r| src[r as usize]));
        }
        out.n = n;
        out
    }

    // Index buckets preserve insertion order. On a composite hash the old kernel
    // tests every equality on each hit, including its null semantics.
    fn pairs(&self, tables: [&Table; 2], driver: usize, ix: &Index, work: &mut Work) -> u64 {
        let other = 1 - driver;
        let dc = &self.keys[driver];
        let oc = &self.keys[other];
        let dt = tables[driver];
        let ot = tables[other];
        let mut touched = 0;
        if let [col] = dc.as_ref() {
            for (r, &key) in dt.cols[*col].iter().enumerate() {
                if let Some(hits) = ix.get(&key) {
                    touched += hits.len() as u64;
                    work.driver.resize(work.driver.len() + hits.len(), r as u32);
                    work.matches.extend_from_slice(hits);
                }
            }
        } else {
            for r in 0..dt.n {
                let key = key_of(dc.iter().map(|&c| dt.cols[c][r]));
                if let Some(hits) = ix.get(&key) {
                    touched += hits.len() as u64;
                    for &h in hits {
                        if dc.iter().zip(oc.iter()).all(|(&a, &b)| {
                            let value = dt.cols[a][r];
                            value != NULL && value == ot.cols[b][h as usize]
                        }) {
                            work.driver.push(r as u32);
                            work.matches.push(h);
                        }
                    }
                }
            }
        }
        touched
    }
}

pub(super) fn load(ex: &mut Exec<'_>, run: &mut Run, kernels: &mut Kernels) {
    if sw!("VEC_NO_COL_JOIN") {
        return;
    }
    let mut empty_by_names =
        rustc_hash::FxHashMap::<super::super::Names, Vec<Rc<Table>>>::default();
    for node in 0..ex.plan.nodes.len() {
        let Ok(Some(j)) = Join::bind(ex, run, node) else {
            continue;
        };
        // An empty call stores only a reference in the executor memo. A single
        // immutable empty table serves all joins with identical names and types.
        let layouts = empty_by_names.entry(j.schema.names.clone()).or_default();
        if let Some(empty) = layouts.iter().find(|t| t.tys == j.schema.tys) {
            ex.empties[node] = Some(empty.clone());
        } else {
            let Ok(empty) = ex.empty_rc(run, node) else {
                continue;
            };
            layouts.push(empty);
        }
        kernels.by_node[node] = (kernels.projections.len() + kernels.joins.len() + 1) as u32;
        kernels.joins.push(j);
    }
    kernels.join_audit = super::join_audit::Audit::open(kernels.joins.len());
}

impl Exec<'_> {
    #[inline]
    fn bound_table(
        &mut self,
        f: &mut File<'_>,
        run: &mut Run,
        p: &mut dyn Provider,
        node: usize,
    ) -> R<Rc<Table>> {
        if let Some(Res::Tab(t)) = &f.memo[node] {
            return Ok(t.clone());
        }
        self.eval(f, run, p, node)
    }

    pub(crate) fn whole_join(
        &mut self,
        f: &mut File<'_>,
        run: &mut Run,
        p: &mut dyn Provider,
        index: usize,
    ) -> R<Rc<Table>> {
        let node = self.col.joins[index].node;
        let out = if sw!("VEC_COL_JOIN_ORACLE") {
            self.select_entry(f, run, p, node)?
        } else {
            self.bound_join(f, run, p, index, node)?
        };
        if let Some(a) = &mut self.col.join_audit {
            a.record(index, node, f.file, &self.col.joins[index].schema, &out)?;
        }
        Ok(out)
    }

    fn bound_join(
        &mut self,
        f: &mut File<'_>,
        run: &mut Run,
        p: &mut dyn Provider,
        index: usize,
        node: usize,
    ) -> R<Rc<Table>> {
        let inputs = self.col.joins[index].inputs;
        let ids = inputs.nodes;
        if inputs.semi_without_band {
            // A pure equality has no range predicate. The existing band reader
            // declines it without reading a source; preserve its reason counter.
            *self.tally.back.entry((node, "no band")).or_default() += 1;
        }
        let mut deferred = [1, 0].into_iter().find_map(|s| {
            inputs.deferred[s]
                .filter(|_| f.memo[ids[s]].is_none())
                .map(|cols| (s, cols))
        });
        if let Some((s, _)) = deferred {
            if self.keyed_size(f, run, p, ids[s], false)? == 0 {
                return self.empty_rc(run, node);
            }
            if s == 0 && f.memo[ids[1]].is_none() {
                deferred = None;
            }
        }
        let (left, right, temporary) = if let Some((s, [col, driver_col])) = deferred {
            let d = 1 - s;
            let driver = self.bound_table(f, run, p, ids[d])?;
            if driver.n == 0 {
                return self.empty_rc(run, node);
            }
            let size = self.keyed_size(f, run, p, ids[s], true)?;
            let restrict = driver.n < size;
            let other = if restrict {
                self.stats.touched[node] += driver.n as u64;
                let mut keys = std::mem::take(&mut self.col.join_work.keys);
                keys.clear();
                keys.extend_from_slice(&driver.cols[driver_col]);
                keys.sort_unstable();
                keys.dedup();
                let t = self.probe(f, run, p, ids[s], col, &keys);
                self.col.join_work.keys = keys;
                t?
            } else {
                self.bound_table(f, run, p, ids[s])?
            };
            if other.n == 0 {
                return self.empty_rc(run, node);
            }
            let temp = restrict.then_some(s);
            if s == 0 {
                (other, driver, temp)
            } else {
                (driver, other, temp)
            }
        } else {
            let a = self.bound_table(f, run, p, ids[0])?;
            if a.n == 0 {
                return self.empty_rc(run, node);
            }
            let b = self.bound_table(f, run, p, ids[1])?;
            if b.n == 0 {
                return self.empty_rc(run, node);
            }
            (a, b, None)
        };
        let tabs = [left.as_ref(), right.as_ref()];
        let driver = temporary.map_or(usize::from(right.n < left.n), |s| 1 - s);
        let other = 1 - driver;
        let j = &self.col.joins[index];
        let w = &mut self.col.join_work;
        w.driver.clear();
        w.matches.clear();
        if temporary.is_none() {
            self.stats.touched[node] += tabs[driver].n as u64;
        }
        if left.n == 1 && right.n == 1 && j.keys[0].len() == 1 {
            // Both columns are already singleton indexes. A miss needs no buffers.
            if left.cols[j.keys[0][0]][0] == right.cols[j.keys[1][0]][0] {
                self.stats.touched[node] += 1;
                w.driver.push(0);
                w.matches.push(0);
            }
        } else if j.keys[other].is_empty() {
            // Lowering removed all comparisons between file-local file columns.
            for r in 0..tabs[driver].n {
                w.driver.resize(w.driver.len() + tabs[other].n, r as u32);
                w.matches.extend(0..tabs[other].n as u32);
            }
            self.stats.touched[node] += w.matches.len() as u64;
        } else {
            let hits = if temporary.is_some() {
                let ix = index_of(tabs[other], &j.keys[other]);
                j.pairs(tabs, driver, &ix, w)
            } else {
                let ix = table_index(f, ids[other], tabs[other], &j.keys[other]);
                j.pairs(tabs, driver, &ix, w)
            };
            self.stats.touched[node] += hits;
        }
        let result = if w.driver.is_empty() {
            None
        } else {
            Some(recycle::rc(j.gather(tabs, driver, w)))
        };
        if let Some(s) = temporary {
            recycle::recycle(if s == 0 { left } else { right });
        }
        match result {
            Some(t) => Ok(t),
            None => self.empty_rc(run, node),
        }
    }
}
