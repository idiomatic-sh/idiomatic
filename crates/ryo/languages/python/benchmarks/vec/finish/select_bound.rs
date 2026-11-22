//! THROWAWAY (ryo-fmx3.1, redo-select2): `Exec::bound_size` for the select kernels, the same answer at a fraction of the
//! cost. The interpreter's bound walks the plan below a pending source (to depth 6) on every select run, matching
//! operator names, looking up arguments and computing each view's bound (its kind bucket, else its literal rows)
//! anew. Here the walk's shape is prepared once a node (by operator code, inputs resolved), and a view's bound is kept
//! once a file (a view's rows are fixed by the file; a probe's substitution never puts a view in place). A made table's
//! rows are read live from the file's memo, as before.

use super::{Exec, File, Provider, R, Res, Run, bucket_len, kind_set};
use std::cell::RefCell;
use std::rc::Rc;

enum Shape {
    /// A view: the kinds its filter requires (else its literal rows bound it), and whether its filter fixes a column to
    /// literals at all (if not, it has no bound, and its fill is not read for one).
    View(Option<Vec<u32>>, bool),
    Select(Box<[u32]>),
    Union(u32, u32),
    Unary(u32),
    Other,
}

thread_local! {
    static SHAPES: RefCell<(usize, Vec<Option<Rc<Shape>>>)> = const { RefCell::new((0, Vec::new())) };
    /// The views' bounds in this file: (file, node table) and by node 0 not known, 1 none, n + 2 a bound of n.
    static VIEWS: RefCell<((u32, usize), Vec<u32>)> = const { RefCell::new(((u32::MAX, 0), Vec::new())) };
    /// The bounds found since `bound_fresh` (nothing is made between, so each (node, depth) has one answer): by
    /// node * 8 + depth, (generation, answer + 1; 0 none).
    static FOUND: RefCell<(u64, Vec<(u64, u32)>)> = const { RefCell::new((1, Vec::new())) };
}

/// A new set of bound questions: nothing made since the last answers may be assumed.
pub(super) fn bound_fresh() {
    FOUND.with(|c| c.borrow_mut().0 += 1);
}

const LIT_FILTER: usize = 1 << 56;

/// The columns of source 0 an expression names, as `lit_rows` walks them.
fn cols(e: &super::E, out: &mut Vec<usize>) {
    use super::E;
    match e {
        E::Col(0, c, _) => out.push(*c),
        E::Cmp(_, a, b) | E::And(a, b) | E::Or(a, b) | E::In(a, b, _) => {
            cols(a, out);
            cols(b, out);
        }
        _ => {}
    }
}

impl Exec<'_> {
    fn bound_shape(&mut self, run: &mut Run, node: usize) -> R<Rc<Shape>> {
        let addr = std::ptr::from_ref(self.plan) as usize;
        if let Some(s) = SHAPES.with(|c| {
            let c = c.borrow();
            if c.0 == addr { c.1.get(node).cloned().flatten() } else { None }
        }) {
            return Ok(s);
        }
        let s = Rc::new(if let Some(v) = self.view(run, node)? {
            // the literal test `lit_rows` makes (a column fixed to text literals), decided once
            let lit = matches!(v.base, super::Native::Fact(_)) && v.filter.iter().any(|e| {
                let mut cs = Vec::new();
                cols(e, &mut cs);
                cs.iter().any(|c| super::kinds_required(e, 0, *c).is_some())
            });
            Shape::View(kind_set(run, &v.filter, 0, v.base, &[]), lit)
        } else {
            let pn = &self.plan.nodes[node];
            match pn.op.as_str() {
                "select" => Shape::Select(pn.inputs.iter().map(|i| *i as u32).collect()),
                "union" => Shape::Union(self.arg_rel(node, "left")? as u32, self.arg_rel(node, "right")? as u32),
                "antijoin" => Shape::Unary(self.arg_rel(node, "left")? as u32),
                "extend" | "project" => Shape::Unary(self.arg_rel(node, "from")? as u32),
                _ => Shape::Other,
            }
        });
        SHAPES.with(|c| {
            let mut c = c.borrow_mut();
            if c.0 != addr {
                *c = (addr, Vec::new());
            }
            if c.1.len() <= node {
                c.1.resize(node + 1, None);
            }
            c.1[node] = Some(s.clone());
        });
        Ok(s)
    }

    /// `bound_size`, exactly: a made table's rows, a view's bucket or literal rows, a select's smallest bounded input, a
    /// union's sum, an antijoin's left, a pass-through step's input; None: unknown.
    pub(super) fn bound_k(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, depth: u32) -> R<Option<usize>> {
        if sw!("VEC_OLD_BOUND") || sw!("VEC_OLD_EMPTYSEL") {
            return self.bound_size(f, run, p, node, depth);
        }
        if let Some(Res::Tab(t)) = &f.memo[node] {
            return Ok(Some(t.n));
        }
        let at = node * 8 + depth.min(7) as usize;
        if let Some(x) = FOUND.with(|c| {
            let c = c.borrow();
            c.1.get(at).filter(|e| e.0 == c.0).map(|e| e.1)
        }) {
            return Ok(x.checked_sub(1).map(|x| x as usize));
        }
        let b = self.bound_k_(f, run, p, node, depth)?;
        FOUND.with(|c| {
            let mut c = c.borrow_mut();
            if c.1.len() <= at {
                c.1.resize(at + 8, (0, 0));
            }
            let g = c.0;
            c.1[at] = (g, b.map_or(0, |x| x as u32 + 1));
        });
        Ok(b)
    }

    fn bound_k_(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, depth: u32) -> R<Option<usize>> {
        let shape = self.bound_shape(run, node)?;
        match &*shape {
            Shape::View(None, false) => Ok(None),
            Shape::View(ks, _) => {
                let key = (f.file, std::ptr::from_ref(f.table) as usize);
                let known = VIEWS.with(|c| {
                    let mut c = c.borrow_mut();
                    if c.0 != key {
                        c.0 = key;
                        c.1.clear();
                        if super::empty_sel::on() {
                            super::empty_sel::count("bound: view cache reset", 0.0);
                        }
                    }
                    c.1.get(node).copied().unwrap_or(0)
                });
                if known != 0 {
                    return Ok((known >= 2).then(|| (known - 2) as usize));
                }
                let Some(v) = self.view(run, node)? else { return Err("a view's bound with no view".to_owned()) };
                let t0 = super::step_clock();
                self.load_fact(f, run, p, v.base)?;
                let t1 = super::step_clock();
                let b = match ks {
                    Some(ks) => Some(bucket_len(f, run, ks)),
                    None => super::select_kernel::lit_file(f, LIT_FILTER | node, &v.filter, 0, v.base, &[]).map(|r| r.len()),
                };
                VIEWS.with(|c| {
                    let mut c = c.borrow_mut();
                    if c.1.len() <= node {
                        c.1.resize(node + 1, 0);
                    }
                    c.1[node] = b.map_or(1, |n| n as u32 + 2);
                });
                if super::empty_sel::on() {
                    super::empty_sel::count("bound: view load_fact", t1 - t0);
                    super::empty_sel::count("bound: view bound", super::step_clock() - t1);
                }
                Ok(b)
            }
            _ if depth == 0 => Ok(None),
            Shape::Select(ins) => {
                let mut m: Option<usize> = None;
                for i in ins.iter() {
                    if let Some(b) = self.bound_k(f, run, p, *i as usize, depth - 1)? {
                        m = Some(m.map_or(b, |x| x.min(b)));
                    }
                }
                Ok(m)
            }
            Shape::Union(l, r) => Ok(match (self.bound_k(f, run, p, *l as usize, depth - 1)?, self.bound_k(f, run, p, *r as usize, depth - 1)?) {
                (Some(a), Some(b)) => Some(a + b),
                _ => None,
            }),
            Shape::Unary(i) => self.bound_k(f, run, p, *i as usize, depth - 1),
            Shape::Other => Ok(None),
        }
    }
}
