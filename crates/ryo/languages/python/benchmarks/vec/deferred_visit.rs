//! redo-binding pass 4 (THROWAWAY, bead ryo-fmx3.1): the `deferred-visit` fact. For every element of a file, whether its
//! evaluation is postponed past its statement (`deferred`): an
//! element of a type expression in a stub (PEP 484: a stub's annotations are never evaluated in order), of an annotation
//! Python stores without evaluating (PEP 563 postponed, PEP 649 deferred, an unevaluated local annotation under the
//! future), of any annotation in a stub, of a type parameter's bound or default, a `type` alias value, a string
//! annotation's embedded expression, or a stub class header. Written from the node table and the two facts it composes
//! (`type-expression`, `annotation-evaluation`); proved against `deferred-type-reads` by the equality probe. Pass 5
//! (PRINCIPLE): what a checker leaves unvisited (Ruff: the unpacked keywords of some typing calls, `bool`'s keywords) is
//! rule text (`deferred-type-reads` over `type-definition-skipped`), not this fact.

use super::finish;
use ryo::languages::{NodeTable, Position, node_table};

const EXPRESSION_KINDS: [&[u8]; 26] = [
    b"name", b"attribute", b"call", b"subscript", b"bin-op", b"unary-op", b"bool-op", b"if-exp", b"named-expr", b"await", b"yield", b"yield-from", b"list",
    b"set", b"tuple", b"dict", b"list-comp", b"set-comp", b"dict-comp", b"generator-exp", b"joined-str", b"template-str", b"formatted-value", b"starred",
    b"constant", b"slice",
];

fn col(t: &finish::Table, n: &str) -> Result<usize, String> {
    t.names.iter().position(|c| c == n).ok_or_else(|| format!("no column {n}"))
}

/// The inputs: the file's two facts, its embedded expressions, whether it is a stub and whether the future is enabled.
pub(super) struct Inputs<'t> {
    pub(super) types: &'t finish::Table,
    pub(super) evaluations: &'t finish::Table,
    pub(super) embedded: &'t finish::Table,
    pub(super) stub: bool,
    pub(super) future: bool,
}

/// redo-fills c2: whether an annotation of the file is stored unevaluated (the `lazy` test of `rows`).
pub(super) fn lazy_any(run: &mut finish::Run, evaluations: &finish::Table, future: bool) -> Result<bool, String> {
    let (s_postponed, s_deferred, s_unevaluated) = (u64::from(run.sym(b"postponed")), u64::from(run.sym(b"deferred")), u64::from(run.sym(b"unevaluated")));
    let a_eval = col(evaluations, "evaluation")?;
    Ok(evaluations.cols[a_eval][..evaluations.rows.0].iter().any(|e| *e == s_postponed || *e == s_deferred || (*e == s_unevaluated && future)))
}

/// redo-fills c2: whether the file has an element of a kind whose children `rows` marks outside a stub (a type parameter or
/// a type alias).
pub(super) fn has_type_parameters(run: &mut finish::Run, table: &NodeTable) -> bool {
    let names: [&[u8]; 4] = [b"type-var", b"type-var-tuple", b"param-spec", b"type-alias"];
    let syms: Vec<u32> = names.iter().map(|k| run.sym(k)).collect();
    run.kind_sym.iter().enumerate().any(|(c, s)| syms.contains(s) && !node_table::ids_of_kind(table, ryo::languages::KindBits(c as u16)).is_empty())
}

/// The fact's rows: (element, deferred = true).
pub(super) fn rows(run: &mut finish::Run, table: &NodeTable, inp: &Inputs<'_>) -> Result<Vec<u32>, String> {
    let n = node_table::element_count(table).0 as u32;
    let expression: Vec<u32> = EXPRESSION_KINDS.iter().map(|k| run.sym(k)).collect();
    let (k_arg, k_ann_assign, k_class, k_arguments, k_alias) = (run.sym(b"arg"), run.sym(b"ann-assign"), run.sym(b"class-def"), run.sym(b"call-arguments"), run.sym(b"type-alias"));
    let k_function = [run.sym(b"function-def"), run.sym(b"async-function-def")];
    let k_params = [run.sym(b"type-var"), run.sym(b"type-var-tuple"), run.sym(b"param-spec")];
    let (f_annotation, f_returns, f_bound, f_default, f_value) = (run.sym(b"annotation"), run.sym(b"returns"), run.sym(b"bound"), run.sym(b"default-value"), run.sym(b"value"));
    let s_type = u64::from(run.sym(b"type"));
    let (s_postponed, s_deferred, s_unevaluated) = (u64::from(run.sym(b"postponed")), u64::from(run.sym(b"deferred")), u64::from(run.sym(b"unevaluated")));
    let kind = |p: u32| run.kind_sym[usize::from(node_table::kind_of(table, Position(p)).0)];
    let field = |p: u32| run.field_sym.get(usize::from(node_table::field_of(table, Position(p)).0)).copied().unwrap_or(run.empty);
    let parent = |p: u32| {
        let q = node_table::parent_of(table, Position(p)).0;
        (p != 0 && q != p).then_some(q)
    };
    let children = |x: u32| node_table::children_of(table, Position(x)).iter().map(|c| c.0).filter(move |c| *c < n);
    let subtree = |x: u32| x..node_table::subtree_end_of(table, Position(x)).0.max(x + 1);
    let mut typed = vec![false; n as usize];
    let (t_id, t_kind) = (col(inp.types, "id")?, col(inp.types, "kind")?);
    for r in 0..inp.types.rows.0 {
        let x = inp.types.cols[t_id][r] as usize;
        if x < n as usize {
            if inp.types.cols[t_kind][r] == s_type {
                typed[x] = true;
            }
        }
    }
    let mut lazy = vec![false; n as usize];
    let (a_id, a_eval) = (col(inp.evaluations, "id")?, col(inp.evaluations, "evaluation")?);
    for r in 0..inp.evaluations.rows.0 {
        let (x, e) = (inp.evaluations.cols[a_id][r] as usize, inp.evaluations.cols[a_eval][r]);
        if x < n as usize && (e == s_postponed || e == s_deferred || (e == s_unevaluated && inp.future)) {
            lazy[x] = true;
        }
    }
    let mut deferred = vec![false; n as usize];
    let mut is_expression = vec![false; n as usize];
    let mark = |deferred: &mut Vec<bool>, root: u32| {
        for q in subtree(root) {
            if q < n {
                deferred[q as usize] = true;
            }
        }
    };
    let mut annotation_roots: Vec<u32> = Vec::new();
    for p in 0..n {
        let k = kind(p);
        is_expression[p as usize] = expression.contains(&k);
        if k == k_arg || k == k_ann_assign {
            annotation_roots.extend(children(p).filter(|c| field(*c) == f_annotation));
        } else if k_function.contains(&k) {
            annotation_roots.extend(children(p).filter(|c| field(*c) == f_returns));
        } else if k_params.contains(&k) {
            for c in children(p).filter(|c| field(*c) == f_bound || field(*c) == f_default) {
                mark(&mut deferred, c);
            }
        } else if k == k_alias {
            for c in children(p).filter(|c| field(*c) == f_value) {
                mark(&mut deferred, c);
            }
        } else if k == k_class && inp.stub {
            // a stub is never executed: its class bases are read as types, in any order (pass 6, GENERALITY: kept in the
            // fact; moving it to rule text would leave these reads without the postponed lookup this fact answers)
            for c in children(p).filter(|c| kind(*c) == k_arguments) {
                mark(&mut deferred, c);
            }
        }
    }
    let e_root = col(inp.embedded, "root")?;
    for r in 0..inp.embedded.rows.0 {
        mark(&mut deferred, inp.embedded.cols[e_root][r] as u32);
    }
    // annotation members: an element in an annotation root's subtree; deferred in a stub, or where Python stores it unevaluated
    for root in annotation_roots {
        for q in subtree(root) {
            if q < n && (inp.stub || lazy[q as usize]) {
                deferred[q as usize] = true;
            }
        }
    }
    // a stub's type expressions: the element (an expression) or an expression above it is typed
    if inp.stub {
        // above[x]: some expression strictly above x is typed (2: not known yet)
        let mut above: Vec<u8> = vec![2; n as usize];
        let typed_expression = |x: u32| is_expression[x as usize] && typed[x as usize];
        let mut chain: Vec<u32> = Vec::new();
        for x in 0..n {
            if above[x as usize] == 2 {
                chain.clear();
                chain.push(x);
                let mut at = x;
                let mut value = loop {
                    let Some(a) = parent(at) else { break false };
                    if above[a as usize] != 2 {
                        break above[a as usize] == 1 || typed_expression(a);
                    }
                    chain.push(a);
                    at = a;
                };
                for c in chain.iter().rev() {
                    above[*c as usize] = u8::from(value);
                    value = value || typed_expression(*c);
                }
            }
            if above[x as usize] == 1 || typed_expression(x) {
                deferred[x as usize] = true;
            }
        }
    }
    let mut out = Vec::new();
    for x in 0..n as usize {
        if deferred[x] {
            out.push(x as u32);
        }
    }
    Ok(out)
}
