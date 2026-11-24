//! redo-binding pass 4 (THROWAWAY, bead ryo-fmx3.1): the `annotation-evaluation` fact. For every element of a file inside an
//! annotation (a parameter or return annotation, an annotated assignment's annotation), when Python evaluates it: `eager`
//! (with its definition or statement, through 3.13 without the annotations future), `postponed` (stored as a string under
//! the future), `deferred` (stored lazily, 3.14 on), `unevaluated` (a function-local variable annotation, or a non-simple
//! target's under the lazy modes); `mode` is `lazy` or `eager`, `root` the annotation. An element inside a lambda body, a
//! generator's element or later clauses, or a string annotation's embedded expression has its own evaluation and no row.
//! The language reference's rules (§7.2.2 annotated assignment statements, PEP 563, PEP 649 / 749), written from the node
//! table, `compiler-feature` and `language-target`; proved against `annotation-evaluation-context` by the equality probe.

use super::finish;
use ryo::languages::{NodeTable, Position, node_table};

/// redo-fills pass 2: whether any element of the file is an annotation or a return annotation (its field), read from the
/// node table's field lists: a file with none has no element inside an annotation, so no row (`rows`), and no annotation
/// root of a type expression.
pub(super) fn has_annotation(run: &mut finish::Run, table: &NodeTable) -> bool {
    let (a, r) = (run.sym(b"annotation"), run.sym(b"returns"));
    run.field_sym.iter().enumerate().any(|(c, s)| (*s == a || *s == r) && u16::try_from(c).is_ok_and(|c| !node_table::ids_of_field(table, ryo::languages::FieldCode(c)).is_empty()))
}

/// One row: element, root, evaluation, mode.
pub(super) type Row = (u32, u32, usize);
pub(super) const NAMES: [(&[u8], &[u8]); 5] = [(b"eager", b"eager"), (b"postponed", b"lazy"), (b"deferred", b"lazy"), (b"unevaluated", b"lazy"), (b"unevaluated", b"eager")];

/// The file's compiler features and target, as `compiler-feature` and `language-target` state them.
pub(super) struct Target {
    /// `python.future-annotations` has a row (enabled or not).
    pub(super) known: bool,
    pub(super) enabled: bool,
    pub(super) disabled: bool,
    pub(super) native: bool,
    pub(super) immediate: bool,
}

fn col(t: &finish::Table, n: &str) -> Result<usize, String> {
    t.names.iter().position(|c| c == n).ok_or_else(|| format!("no column {n}"))
}

pub(super) fn target(run: &mut finish::Run, features: &finish::Table, targets: &finish::Table) -> Result<Target, String> {
    let (f_feature, f_enabled) = (col(features, "feature")?, col(features, "enabled")?);
    let future = u64::from(run.sym(b"python.future-annotations"));
    let truth = u64::from(run.sym(b"true"));
    let flag = features.tys[f_enabled] == finish::Ty::Flag;
    let mut t = Target { known: false, enabled: false, disabled: false, native: false, immediate: false };
    for r in 0..features.n {
        if features.cols[f_feature][r] != future {
            continue;
        }
        t.known = true;
        let v = features.cols[f_enabled][r];
        if (flag && v == 1) || (!flag && v == truth) {
            t.enabled = true;
        } else {
            t.disabled = true;
        }
    }
    let (t_major, t_minor) = (col(targets, "major")?, col(targets, "minor")?);
    for r in 0..targets.n {
        let (major, minor) = (targets.cols[t_major][r] as i64, targets.cols[t_minor][r] as i64);
        if major == 3 && minor >= 14 {
            t.native = true;
        }
        if major == 3 && minor < 14 {
            t.immediate = true;
        }
    }
    Ok(t)
}

pub(super) fn rows(run: &mut finish::Run, table: &NodeTable, target: &Target, embedded: &finish::Table) -> Result<Vec<Row>, String> {
    let mut out: Vec<Row> = Vec::new();
    if !target.known && !target.enabled {
        // no class but `unevaluated` local (needs a known feature row), postponed (enabled), eager / deferred (disabled)
        return Ok(out);
    }
    let n = node_table::element_count(table).0 as u32;
    let sym = |run: &mut finish::Run, t: &[u8]| run.sym(t);
    let k_function = [sym(run, b"function-def"), sym(run, b"async-function-def")];
    let k_arg = sym(run, b"arg");
    let k_ann_assign = sym(run, b"ann-assign");
    let k_name = sym(run, b"name");
    let k_lambda = sym(run, b"lambda");
    let k_class = sym(run, b"class-def");
    let k_module = sym(run, b"module");
    let k_generator = sym(run, b"generator-exp");
    let f_annotation = sym(run, b"annotation");
    let f_returns = sym(run, b"returns");
    let f_target = sym(run, b"target");
    let f_body = sym(run, b"body");
    let f_elt = sym(run, b"elt");
    let f_generators = sym(run, b"generators");
    let f_iter = sym(run, b"iter");
    let kind = |run: &finish::Run, p: u32| run.kind_sym[usize::from(node_table::kind_of(table, Position(p)).0)];
    let field = |run: &finish::Run, p: u32| run.field_sym.get(usize::from(node_table::field_of(table, Position(p)).0)).copied().unwrap_or(run.empty);
    let parent = |p: u32| {
        let q = node_table::parent_of(table, Position(p)).0;
        (p != 0 && q != p).then_some(q)
    };
    let children = |x: u32| node_table::children_of(table, Position(x)).iter().map(|c| c.0).filter(move |c| *c < n);
    let start = |p: u32| node_table::span_of(table, Position(p)).start.0;
    let subtree = |x: u32| x..node_table::subtree_end_of(table, Position(x)).0.max(x + 1);
    // root classes: bit 0 eager, 1 postponed, 2 deferred, 3 unevaluated lazy, 4 unevaluated eager
    let mut class: Vec<Option<u8>> = vec![None; n as usize];
    let mut bodies: Vec<u32> = Vec::new();
    let mut annotations = Vec::new();
    let relevant = [k_arg, k_function[0], k_function[1], k_ann_assign, k_lambda, k_generator];
    let roots = run.kind_sym.iter().enumerate().filter(|(_, k)| relevant.contains(k))
        .flat_map(|(code, _)| node_table::ids_of_kind(table, ryo::languages::KindBits(code as u16)));
    for &Position(p) in roots {
        let k = kind(run, p);
        if k == k_arg || k_function.contains(&k) {
            for c in children(p) {
                let f = field(run, c);
                let signature = if k == k_arg {
                    // a parameter annotation whose parameter is inside a function
                    f == f_annotation && {
                        let mut at = parent(p);
                        while let Some(a) = at {
                            if k_function.contains(&kind(run, a)) {
                                break;
                            }
                            at = parent(a);
                        }
                        at.is_some()
                    }
                } else {
                    f == f_returns
                };
                if signature {
                    let mut bits = 0;
                    if target.disabled && target.immediate {
                        bits |= 1;
                    }
                    if target.enabled {
                        bits |= 2;
                    }
                    if target.disabled && target.native {
                        bits |= 4;
                    }
                    if class[c as usize].is_none() { annotations.push(c); }
                    *class[c as usize].get_or_insert(0) |= bits;
                }
            }
        } else if k == k_ann_assign {
            let mut scope = parent(p);
            while let Some(s) = scope {
                let sk = kind(run, s);
                if sk == k_module || sk == k_class || sk == k_lambda || k_function.contains(&sk) {
                    break;
                }
                scope = parent(s);
            }
            let scope_kind = scope.map(|s| kind(run, s));
            let local = scope_kind.is_some_and(|sk| sk == k_lambda || k_function.contains(&sk));
            let outer = scope_kind.is_some_and(|sk| sk == k_module || sk == k_class);
            let simple = children(p).any(|t| field(run, t) == f_target && kind(run, t) == k_name && start(t) == start(p));
            for c in children(p) {
                if field(run, c) != f_annotation {
                    continue;
                }
                let mut bits = 0;
                if outer {
                    if target.disabled && target.immediate {
                        bits |= 1;
                    }
                    if simple {
                        if target.enabled {
                            bits |= 2;
                        }
                        if target.disabled && target.native {
                            bits |= 4;
                        }
                    } else if target.enabled || (target.disabled && target.native) {
                        bits |= 8;
                    }
                }
                if local && target.known {
                    if target.enabled || (target.disabled && target.native) {
                        bits |= 8;
                    } else {
                        bits |= 16;
                    }
                }
                // a root with no class still holds its members (they take no row)
                if class[c as usize].is_none() { annotations.push(c); }
                *class[c as usize].get_or_insert(0) |= bits;
            }
        } else if k == k_lambda {
            bodies.extend(children(p).filter(|c| field(run, *c) == f_body));
        } else if k == k_generator {
            bodies.extend(children(p).filter(|c| field(run, *c) == f_elt));
            let clauses: Vec<u32> = children(p).filter(|c| field(run, *c) == f_generators).collect();
            let first = clauses.iter().map(|c| start(*c)).min();
            for c in &clauses {
                for cc in children(*c) {
                    if !(Some(start(*c)) == first && field(run, cc) == f_iter) {
                        bodies.push(cc);
                    }
                }
            }
        }
    }
    let e_root = col(embedded, "root")?;
    bodies.extend((0..embedded.n).map(|r| embedded.cols[e_root][r] as u32));
    if annotations.iter().all(|r| class[*r as usize] == Some(0)) { return Ok(out); }
    // These intervals are the same body exclusions used by the lexical model.
    // Tokens can be numbered after their parent subtree, so follow child edges
    // for membership, including when traversing an excluded syntax node.
    let mut excluded = Vec::new();
    if !bodies.is_empty() {
        excluded.resize(n as usize, false);
        for b in bodies {
            let range = subtree(b);
            if b < n { excluded[b as usize..range.end.min(n) as usize].fill(true); }
        }
    }
    // The host is the embedded root's parent, but the host's child list
    // contains only its source syntax. Include those additional parent edges.
    let mut embedded_edges: Vec<_> = node_table::embedded_segments(table)
        .map(|segment| (segment.host.0, segment.root.0)).collect();
    embedded_edges.sort_unstable();
    let mut stack = Vec::new();
    for root in annotations {
        let bits = class[root as usize].unwrap();
        if bits == 0 { continue; }
        stack.push(root);
        while let Some(x) = stack.pop() {
            if !excluded.get(x as usize).copied().unwrap_or(false) {
                let mut pending = bits;
                while pending != 0 {
                    out.push((x, root, pending.trailing_zeros() as usize));
                    pending &= pending - 1;
                }
            }
            // The nested root itself belongs to both annotations. Its children
            // belong to the nearer root alone, even if that root emits no class.
            if x == root || class[x as usize].is_none() {
                stack.extend(children(x));
                let first = embedded_edges.partition_point(|&(host, _)| host < x);
                for &(host, child) in &embedded_edges[first..] {
                    if host != x { break; }
                    stack.push(child);
                }
            }
        }
    }
    out.sort_unstable();
    Ok(out)
}
