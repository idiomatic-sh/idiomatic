//! What a step needs to return anything: its inputs, the kind sets its rows must have one of, and whether a per-file test decides it.
//! The closed phase reads it to prove a step empty (`session::proven_empty`).

use super::{E, Exec, Extent, File, Native, Provider, Res, Run, filter_order};

/// What a step needs to return anything, prepared once a run (at its first ask).
pub struct Need {
    /// Empty when any of these is empty (or, `all`, when every one is).
    pub inputs: Box<[usize]>,
    pub all: bool,
    /// Kind sets the rows must have one of (kind codes): empty when the file has none of a set.
    pub kinds: Box<[Box<[u16]>]>,
    /// A per-file test to decide.
    pub file_test: bool,
    /// Never known empty (a constant step: made once a run, as before).
    pub never: bool,
}

impl Exec<'_> {

    /// The step's needs, prepared on its first use in this execution.
    #[inline]
    pub fn need(&mut self, run: &mut Run, node: usize) -> std::rc::Rc<Need> {
        if let Some(n) = &self.needs[node] {
            return n.clone();
        }
        let n = std::rc::Rc::new(self.need_of(run, node));
        self.needs[node] = Some(n.clone());
        n
    }

    pub fn need_of(&mut self, run: &mut Run, node: usize) -> Need {
        let mut n = Need { inputs: Box::new([]), all: false, kinds: Box::new([]), file_test: false, never: false };
        if self.is_constant(run, node) {
            n.never = true;
            return n;
        }
        let codes = |run: &Run, syms: &[u32]| -> Box<[u16]> { run.kind_sym.iter().enumerate().filter(|(_, s)| syms.contains(s)).map(|(c, _)| c as u16).collect() };
        let file_test = |conj: &[E]| conj.iter().any(|c| filter_order::file_const(c) && !matches!(c, E::Lit(_)));
        if let Ok(Some(v)) = self.view(run, node) {
            if v.base.is_tree() {
                n.kinds = v.filter.iter().filter_map(|c| super::kind_set(run, std::slice::from_ref(c), 0, v.base, &[])).map(|ks| codes(run, &ks)).collect();
            }
            n.file_test = file_test(&v.filter);
            return n;
        }
        let plan = self.plan;
        let pn = &plan.nodes[node];
        let first = match pn.op() {
            "select" => {
                let Ok(sp) = self.sel(run, node) else {
                    n.never = true;
                    return n;
                };
                let mut kinds = Vec::new();
                for (i, s) in sp.srcs.iter().enumerate() {
                    if let Ok(Some(v)) = self.view(run, *s)
                        && v.base.is_tree()
                    {
                        kinds.extend(sp.conj.iter().filter_map(|c| super::kind_set(run, std::slice::from_ref(c), i, v.base, &v.map)).map(|ks| codes(run, &ks)));
                    }
                }
                n.kinds = kinds.into();
                n.file_test = file_test(&sp.conj);
                n.inputs = Box::from(&*sp.srcs);
                return n;
            }
            "union" => {
                n.inputs = Box::from(&*pn.inputs);
                n.all = true;
                return n;
            }
            // the counted group-count reads its select's kept side with zero counts: never known from its input
            "group-count" if !pn.distinct_count() && !sw!("VEC_NO_SEMI") && self.arg_rel(node, "from").is_ok_and(|x| plan.nodes[x].counted()) => {
                n.never = true;
                return n;
            }
            "antijoin" | "intersection" | "difference" | "ordered-match" | "as-of" => "left",
            "closure" | "follow" => "edges",
            "base" | "rows" | "worklist" => {
                n.never = true;
                return n;
            }
            _ => "from",
        };
        match self.arg_rel(node, first) {
            Ok(i) => n.inputs = Box::new([i]),
            Err(_) => n.never = true,
        }
        n
    }
}
