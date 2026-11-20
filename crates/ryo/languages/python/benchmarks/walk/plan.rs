//! Measured planning (bead ryo-k8gz, throwaway): statistics per plan point from a run over a file sample, and the pass that
//! orders each program level by them.
//!
//! A plan point is an instruction of the pack's programs after the structural passes (specialise, fusions, detached
//! components, folding, first-element marks, memo specialisations) and before hoisting, numbered in one canonical walk over the
//! groups, made tables, shared entries and aux programs. A statistics run (`NAV_PLAN_STAGE=stats`, `NAV_PLAN_STATS=1`) executes
//! exactly those programs and writes, per point, how often it ran and, per program, the steps spent below it. The planner
//! (`NAV_PLAN=<statistics>`) derives per instruction its pass rate or fan-out (runs of the next instruction over its own) and
//! its cost (its own step plus the steps of its sub-programs, per run), and orders each level by rank (selectivity - 1) / cost
//! under the register dependencies. Nothing here reads a rule id, a rule's text or a relation's name.

use crate::nav::{Ins, NavProgram, Prog, R, free_regs, ins_free_reads, reads_defs, reads_defs_top, subs_mut, subs_ref};
use crate::sema::FxMap;

/// What owns a root program: the unit a plan is kept or dropped by.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Unit {
    Group(usize),
    Made(usize),
    Shared(usize),
    Aux(usize),
}

/// Every root program of the pack in the canonical order (the order plan points are numbered in).
fn roots<'a>(prog: &'a NavProgram, f: &mut dyn FnMut(&'a [Ins], Unit)) {
    for (g, grp) in prog.groups.iter().enumerate() {
        f(&grp.prog, Unit::Group(g));
    }
    for (i, fr) in prog.facts.iter().enumerate() {
        if let Some((progs, ..)) = &fr.made {
            progs.iter().for_each(|q| f(q, Unit::Made(i)));
        }
        if let Some(om) = &fr.made_om {
            om.left.iter().chain(om.right.iter()).for_each(|q| f(q, Unit::Made(i)));
        }
    }
    for (k, q) in prog.shared.iter().enumerate() {
        f(q, Unit::Shared(k));
    }
    for (ri, r) in prog.rules.iter().enumerate() {
        for (_, ps, _) in &r.aux {
            ps.iter().for_each(|q| f(q, Unit::Aux(ri)));
        }
    }
}

/// Plan point of every instruction (by address): its index in the canonical pre-order walk.
pub(crate) fn site_ids(prog: &NavProgram) -> FxMap<usize, u32> {
    fn walk(p: &[Ins], ids: &mut FxMap<usize, u32>, next: &mut u32) {
        for i in p {
            ids.insert(i as *const Ins as usize, *next);
            *next += 1;
            subs_ref(i, &mut |q| walk(q, ids, next));
        }
    }
    let mut ids = FxMap::default();
    let mut next = 0u32;
    roots(prog, &mut |q, _| walk(q, &mut ids, &mut next));
    ids
}

/// One worker's statistics as a JSON line: runs per plan point, (runs, inclusive steps) per program (its first point).
pub(crate) fn stats_json(prog: &NavProgram, prof: &FxMap<(usize, u32), u64>, pincl: &FxMap<usize, (u64, u64)>, extra: &FxMap<usize, u64>) -> String {
    let ids = site_ids(prog);
    let mut ins: FxMap<u32, u64> = FxMap::default();
    for (&(addr, _), &n) in prof {
        if let Some(&id) = ids.get(&addr) {
            *ins.entry(id).or_default() += n;
        }
    }
    let mut a: Vec<(u32, u64)> = ins.into_iter().collect();
    a.sort_unstable();
    let mut b: Vec<(u32, u64, u64)> = pincl.iter().filter_map(|(addr, &(n, st))| ids.get(addr).map(|&id| (id, n, st))).collect();
    b.sort_unstable();
    let mut x: Vec<(u32, u64)> = extra.iter().filter(|(_, n)| **n > 0).filter_map(|(addr, &n)| ids.get(addr).map(|&id| (id, n))).collect();
    x.sort_unstable();
    format!(
        "{{\"plan_stats\":{{\"points\":{},\"ins\":[{}],\"prog\":[{}],\"extra\":[{}]}}}}",
        ids.len(),
        a.iter().map(|(i, n)| format!("[{i},{n}]")).collect::<Vec<_>>().join(","),
        b.iter().map(|(i, n, st)| format!("[{i},{n},{st}]")).collect::<Vec<_>>().join(","),
        x.iter().map(|(i, n)| format!("[{i},{n}]")).collect::<Vec<_>>().join(",")
    )
}

/// The statistics of a sample run, summed over its workers' lines.
#[derive(Default)]
struct Stats {
    points: usize,
    ins: FxMap<u32, u64>,
    prog: FxMap<u32, (u64, u64)>,
    /// made-table build steps a point's demands stand for
    extra: FxMap<u32, u64>,
}

fn load_stats(path: &str) -> Stats {
    let mut s = Stats::default();
    for line in std::fs::read_to_string(path).expect("plan statistics").lines() {
        if !line.starts_with("{\"plan_stats\"") {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(line).expect("plan statistics json");
        let v = &v["plan_stats"];
        s.points = v["points"].as_u64().unwrap() as usize;
        for e in v["ins"].as_array().unwrap() {
            *s.ins.entry(e[0].as_u64().unwrap() as u32).or_default() += e[1].as_u64().unwrap();
        }
        for e in v["extra"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
            *s.extra.entry(e[0].as_u64().unwrap() as u32).or_default() += e[1].as_u64().unwrap();
        }
        for e in v["prog"].as_array().unwrap() {
            let x = s.prog.entry(e[0].as_u64().unwrap() as u32).or_default();
            x.0 += e[1].as_u64().unwrap();
            x.1 += e[2].as_u64().unwrap();
        }
    }
    s
}

/// Units whose learned order is dropped (the sample ran no cheaper with it): `{"groups":[..],"made":[..],"shared":bool}`.
#[derive(Default)]
struct Deny {
    groups: Vec<usize>,
    made: Vec<usize>,
    /// rules (their index in the pack): the trie branches only they run keep the unlearned order
    rules: Vec<usize>,
    shared: bool,
}

fn load_deny() -> Deny {
    let Ok(path) = std::env::var("NAV_PLAN_DENY") else { return Deny::default() };
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).expect("plan deny")).expect("plan deny json");
    let list = |k: &str| v[k].as_array().map(|a| a.iter().map(|x| x.as_u64().unwrap() as usize).collect()).unwrap_or_default();
    Deny { groups: list("groups"), made: list("made"), rules: list("rules"), shared: v["shared"].as_bool().unwrap_or(false) }
}

thread_local! {
    /// every register defined anywhere inside each shared entry
    static SHARED_DEFS: std::cell::RefCell<Vec<Vec<R>>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Every register defined anywhere inside a program (nested sub-programs and shared entries included).
fn all_defs(p: &[Ins], out: &mut Vec<R>) {
    for i in p {
        if let Ins::Shared { idx } = i {
            SHARED_DEFS.with(|t| out.extend(t.borrow()[*idx as usize].iter().copied()));
            continue;
        }
        let mut r = Vec::new();
        reads_defs_top(i, &mut r, out);
        subs_ref(i, &mut |q| all_defs(q, out));
    }
}

/// What the planner knows about one instruction of a level.
struct Item {
    /// registers it needs from outside, registers it sets for what follows, registers its sub-programs use as scratch
    reads: Vec<R>,
    defs: Vec<R>,
    scratch: Vec<R>,
    /// continuations per run (pass rate of a test or check, fan-out of an iteration), smoothed
    sel: f64,
    /// steps per run (its own and its sub-programs')
    cost: f64,
}

fn overlap(a: &[R], b: &[R]) -> bool {
    a.iter().any(|x| b.contains(x))
}

/// `b` (later in the given order) must stay after `a`: a flow, anti or output dependence on a register, a sub-program's
/// scratch register included when the other step reads or sets it.
fn depends(a: &Item, b: &Item) -> bool {
    overlap(&a.defs, &b.reads)
        || overlap(&a.reads, &b.defs)
        || overlap(&a.defs, &b.defs)
        || overlap(&a.scratch, &b.reads)
        || overlap(&a.scratch, &b.defs)
        || overlap(&a.reads, &b.scratch)
        || overlap(&a.defs, &b.scratch)
}

/// how many steps of an unlocked chain a step's rank looks through
const LOOKAHEAD: usize = 8;

fn rank(sel: f64, cost: f64) -> f64 {
    (sel - 1.0) / cost.max(1e-9)
}

/// Modelled steps of an order per entry of the level: each step's cost times the product of the selectivities before it.
fn modelled(order: &[usize], items: &[Item]) -> f64 {
    let (mut reach, mut total) = (1.0f64, 0.0f64);
    for &i in order {
        total += reach * items[i].cost;
        reach *= items[i].sel;
    }
    total
}

/// Rank ordering under precedence: of the steps whose predecessors are placed, the one with the lowest rank goes next, where a
/// step's rank is that of the best chain it unlocks (a walk followed by the selective test it enables counts as one module).
fn order_level(items: &[Item]) -> Vec<usize> {
    let n = items.len();
    let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
    for b in 0..n {
        for a in 0..b {
            if depends(&items[a], &items[b]) {
                preds[b].push(a);
            }
        }
    }
    let mut placed = vec![false; n];
    let mut order = Vec::with_capacity(n);
    let mut in_chain = vec![false; n];
    while order.len() < n {
        let mut best: Option<(f64, usize)> = None;
        for i in 0..n {
            if placed[i] || !preds[i].iter().all(|&p| placed[p]) {
                continue;
            }
            let (mut c, mut s) = (items[i].cost, items[i].sel);
            in_chain.iter_mut().for_each(|x| *x = false);
            in_chain[i] = true;
            // the best prefix of the chain this step unlocks: the chain grows by its lowest-ranked unlocked step (a read is
            // followed through to the test it feeds), and the step is ranked by the lowest rank any prefix reaches
            let mut r = rank(s, c);
            for _ in 0..LOOKAHEAD {
                // steps this chain unlocks: every predecessor placed or in the chain, at least one in the chain
                let mut next: Option<(f64, usize)> = None;
                for j in 0..n {
                    if placed[j] || in_chain[j] {
                        continue;
                    }
                    if preds[j].iter().all(|&p| placed[p] || in_chain[p]) && preds[j].iter().any(|&p| in_chain[p]) {
                        let rj = rank(items[j].sel, items[j].cost);
                        if next.is_none_or(|(br, _)| rj < br) {
                            next = Some((rj, j));
                        }
                    }
                }
                let Some((_, j)) = next else { break };
                c += s * items[j].cost;
                s *= items[j].sel;
                in_chain[j] = true;
                r = r.min(rank(s, c));
            }
            if best.is_none_or(|(br, _)| r < br) {
                best = Some((r, i));
            }
        }
        let (_, i) = best.expect("a level's dependence graph is acyclic");
        placed[i] = true;
        order.push(i);
    }
    order
}

#[derive(Default)]
struct Report {
    levels: u64,
    measured: u64,
    reordered: u64,
    before: f64,
    after: f64,
}

/// Plans one program level and everything below it.
fn plan_prog(p: &mut Prog, ids: &FxMap<usize, u32>, st: &Stats, min_gain: f64, denied: &[bool], rep: &mut Report) {
    let n = p.len();
    // this level's measurements, read before anything below moves
    let runs: Vec<u64> = p.iter().map(|i| ids.get(&(i as *const Ins as usize)).and_then(|id| st.ins.get(id)).copied().unwrap_or(0)).collect();
    let below: Vec<u64> = p
        .iter()
        .map(|i| {
            let mut s = ids.get(&(i as *const Ins as usize)).and_then(|id| st.extra.get(id)).copied().unwrap_or(0);
            subs_ref(i, &mut |q| {
                if let Some(first) = q.first() {
                    s += ids.get(&(first as *const Ins as usize)).and_then(|id| st.prog.get(id)).map_or(0, |x| x.1);
                }
            });
            s
        })
        .collect();
    for i in p.iter_mut() {
        if let Ins::Fork { subs } = i {
            // a trie branch run only by rules whose learned order was dropped keeps the unlearned order
            for (slots, q) in subs.iter_mut() {
                if !slots.iter().all(|&s| denied[s as usize]) {
                    plan_prog(q, ids, st, min_gain, denied, rep);
                }
            }
            continue;
        }
        subs_mut(i, &mut |q| plan_prog(q, ids, st, min_gain, denied, rep));
    }
    rep.levels += 1;
    if n < 3 || p.iter().any(|i| matches!(i, Ins::SubFirst { .. })) {
        return;
    }
    // a group's program is entered after its kind walk: the walk stays first
    let start = usize::from(runs[0] == 0 && matches!(p[0], Ins::IterKind { .. }));
    // the measured steps: a prefix (a step never reached on the sample keeps its place, as does the level's last step)
    let mut end = start;
    while end < n - 1 && runs[end] > 0 {
        end += 1;
    }
    if end - start < 2 {
        return;
    }
    rep.measured += 1;
    let items: Vec<Item> = (start..end)
        .map(|k| {
            let i = &p[k];
            let mut has_subs = false;
            subs_ref(i, &mut |_| has_subs = true);
            let (mut reads, mut defs, mut scratch) = (Vec::new(), Vec::new(), Vec::new());
            if has_subs {
                reads = ins_free_reads(i);
                reads_defs_top(i, &mut Vec::new(), &mut defs);
                subs_ref(i, &mut |q| all_defs(q, &mut scratch));
            } else {
                reads_defs(i, &mut reads, &mut defs);
            }
            let (r, next) = (runs[k] as f64, runs[k + 1] as f64);
            // a step nothing passed on the sample is not taken as never passing: half a continuation
            let sel = if std::env::var_os("NAV_PLAN_SMOOTH").is_some() { (next + 0.5) / (r + 1.0) } else if runs[k + 1] == 0 { 0.5 / (r + 1.0) } else { next / r };
            Item { reads, defs, scratch, sel, cost: (runs[k] + below[k]) as f64 / r }
        })
        .collect();
    let given: Vec<usize> = (0..items.len()).collect();
    let order = order_level(&items);
    let (c0, c1) = (modelled(&given, &items), modelled(&order, &items));
    let entries = runs[start] as f64;
    if std::env::var("NAV_PLAN_DEBUG").ok().and_then(|v| v.parse::<f64>().ok()).is_some_and(|min| c0 * entries >= min) {
        eprintln!("LEVEL entries {entries} modelled {:.0} -> {:.0}", c0 * entries, c1 * entries);
        for (pos, &k) in order.iter().enumerate() {
            let text: String = format!("{:?}", p[start + k]).chars().take(110).collect();
            eprintln!("  {pos:>3} was {k:>3} runs {:>9} sel {:>9.4} cost {:>9.2} {text}", runs[start + k], items[k].sel, items[k].cost);
        }
    }
    rep.before += c0 * entries;
    if order == given || c1 > c0 * (1.0 - min_gain) {
        rep.after += c0 * entries;
        return;
    }
    rep.after += c1 * entries;
    rep.reordered += 1;
    let mut v: Vec<Option<Ins>> = std::mem::take(p).into_vec().into_iter().map(Some).collect();
    let mut out: Vec<Ins> = Vec::with_capacity(n);
    for slot in v.iter_mut().take(start) {
        out.push(slot.take().unwrap());
    }
    for &k in &order {
        out.push(v[start + k].take().unwrap());
    }
    for slot in v.iter_mut().skip(end) {
        out.push(slot.take().unwrap());
    }
    *p = out.into();
}

/// The planning pass (`NAV_PLAN=<statistics file>`): orders every level of every unit by the sample's measurements. Returns,
/// per group, made table and for the shared entries, whether the unit was planned (the others keep the unlearned order).
pub(crate) fn plan(prog: &mut NavProgram) -> Option<(Vec<bool>, Vec<bool>, bool, Vec<bool>)> {
    let path = std::env::var("NAV_PLAN").ok()?;
    let st = load_stats(&path);
    let ids = site_ids(prog);
    assert_eq!(st.points, ids.len(), "the statistics were taken on another program (plan points differ)");
    let deny = load_deny();
    let min_gain: f64 = std::env::var("NAV_PLAN_GAIN").ok().and_then(|v| v.parse().ok()).unwrap_or(0.02);
    SHARED_DEFS.with(|t| t.borrow_mut().clear());
    for q in &prog.shared {
        let mut d = Vec::new();
        all_defs(q, &mut d);
        d.sort_unstable();
        d.dedup();
        SHARED_DEFS.with(|t| t.borrow_mut().push(d));
    }
    let mut rep = Report::default();
    // per (rule, branch) slot: its rule's learned order was dropped
    let denied: Vec<bool> = prog.slot_rule.iter().map(|&r| deny.rules.contains(&(r as usize))).collect();
    let none: Vec<bool> = Vec::new();
    let groups: Vec<bool> = (0..prog.groups.len()).map(|g| !deny.groups.contains(&g) && !prog.groups[g].slots.iter().all(|&s| denied[s as usize])).collect();
    let made: Vec<bool> = (0..prog.facts.len()).map(|i| !deny.made.contains(&i)).collect();
    for (g, grp) in prog.groups.iter_mut().enumerate() {
        if groups[g] {
            plan_prog(&mut grp.prog, &ids, &st, min_gain, &denied, &mut rep);
        }
    }
    for (i, fr) in prog.facts.iter_mut().enumerate() {
        if !made[i] {
            continue;
        }
        if let Some((progs, ..)) = &mut fr.made {
            progs.iter_mut().for_each(|q| plan_prog(q, &ids, &st, min_gain, &none, &mut rep));
        }
        if let Some(om) = &mut fr.made_om {
            om.left.iter_mut().chain(om.right.iter_mut()).for_each(|q| plan_prog(q, &ids, &st, min_gain, &none, &mut rep));
        }
    }
    if !deny.shared {
        for q in &mut prog.shared {
            plan_prog(q, &ids, &st, min_gain, &none, &mut rep);
        }
    }
    // a shared entry's free registers do not depend on its order; the tables the later passes read stay valid
    let _ = free_regs;
    eprintln!(
        "{{\"plan\":{{\"levels\":{},\"measured\":{},\"reordered\":{},\"modelled_sample_steps_before\":{:.0},\"modelled_sample_steps_after\":{:.0},\"denied_groups\":{},\"denied_made\":{},\"denied_rules\":{},\"denied_shared\":{}}}}}",
        rep.levels, rep.measured, rep.reordered, rep.before, rep.after, deny.groups.len(), deny.made.len(), deny.rules.len(), deny.shared
    );
    Some((groups, made, !deny.shared, denied))
}

/// The prior (the sink) for the trie branches of a planned group whose rules' learned order was dropped.
pub(crate) fn sink_denied(p: &mut Prog, denied: &[bool]) {
    for i in p.iter_mut() {
        if let Ins::Fork { subs } = i {
            for (slots, q) in subs.iter_mut() {
                if slots.iter().all(|&s| denied[s as usize]) {
                    *q = crate::nav::sink(std::mem::take(q));
                } else {
                    sink_denied(q, denied);
                }
            }
        }
    }
}

/// The unit lists of a run (`NAV_PLAN_STATS`): each group's rules, so a sample comparison can name the units a rule runs in.
pub(crate) fn units_json(prog: &NavProgram) -> String {
    let groups: Vec<String> = prog
        .groups
        .iter()
        .map(|g| {
            let mut ids: Vec<&str> = g.slots.iter().map(|&s| prog.rules[prog.slot_rule[s as usize] as usize].id.as_str()).collect();
            ids.dedup();
            format!("[{}]", ids.iter().map(|i| format!("\"{i}\"")).collect::<Vec<_>>().join(","))
        })
        .collect();
    let rules: Vec<String> = prog.rules.iter().map(|r| format!("\"{}\"", r.id)).collect();
    format!("{{\"plan_units\":{{\"groups\":[{}],\"rules\":[{}]}}}}", groups.join(","), rules.join(","))
}

/// NAV_PROFILE report: per (instruction site, rule charged) its steps, with the chain of constructs that enclose the site
/// (a shared entry takes the chain of its first use), its path and its text.
pub(crate) fn profile_report(prog: &NavProgram, prof: &FxMap<(usize, u32), u64>) -> String {
    struct W<'a> {
        prog: &'a NavProgram,
        sites: FxMap<usize, String>,
        shared_done: Vec<bool>,
    }
    fn name(i: &Ins) -> &'static str {
        crate::nav::INS_NAMES[i.code()]
    }
    impl W<'_> {
        fn walk(&mut self, p: &[Ins], path: String, chain: &str) {
            for (pc, ins) in p.iter().enumerate() {
                let here = format!("{path}.{pc}");
                let text: String = format!("{ins:?}").chars().take(140).collect();
                self.sites.insert(ins as *const Ins as usize, format!("{} {here} {text}", if chain.is_empty() { "top" } else { chain }));
                if let Ins::Shared { idx } = ins {
                    if !self.shared_done[*idx as usize] {
                        self.shared_done[*idx as usize] = true;
                        let prog = self.prog;
                        self.walk(&prog.shared[*idx as usize], format!("s{idx}"), chain);
                    }
                    continue;
                }
                let inner = if matches!(ins, Ins::Fork { .. }) { chain.to_owned() } else if chain.is_empty() { name(ins).to_owned() } else { format!("{chain}>{}", name(ins)) };
                let mut k = 0;
                subs_ref(ins, &mut |q| {
                    self.walk(q, format!("{here}/{k}"), &inner);
                    k += 1;
                });
            }
        }
    }
    let mut w = W { prog, sites: FxMap::default(), shared_done: vec![false; prog.shared.len()] };
    for (g, grp) in prog.groups.iter().enumerate() {
        w.walk(&grp.prog, format!("g{g}"), "");
    }
    for (i, f) in prog.facts.iter().enumerate() {
        if let Some((progs, ..)) = &f.made {
            for (k, q) in progs.iter().enumerate() {
                w.walk(q, format!("m{i}p{k}"), "made");
            }
        }
        if let Some(om) = &f.made_om {
            for (k, q) in om.left.iter().chain(om.right.iter()).enumerate() {
                w.walk(q, format!("m{i}o{k}"), "made");
            }
        }
    }
    for (ri, r) in prog.rules.iter().enumerate() {
        for (k, (_, ps, _)) in r.aux.iter().enumerate() {
            for q in ps {
                w.walk(q, format!("a{ri}x{k}"), "aux");
            }
        }
    }
    let mut v: Vec<((usize, u32), u64)> = prof.iter().map(|(&k, &n)| (k, n)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    let rule = |r: u32| prog.rules.get(r as usize).map_or("_shared", |x| x.id.as_str());
    v.iter().map(|((addr, r), n)| format!("{n:>12} {} {}", rule(*r), w.sites.get(addr).map_or("? ? ?", String::as_str))).collect::<Vec<_>>().join("\n")
}
