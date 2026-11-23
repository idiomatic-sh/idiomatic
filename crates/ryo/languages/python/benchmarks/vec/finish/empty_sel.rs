//! THROWAWAY (ryo-fmx3.1, redo-select2): selects that return nothing.
//!
//! `VEC_ESEL_WHY=1`: every select run that returns nothing is classed by where it ended (the stage marked last in
//! `Exec::select`), by the sizes it saw (the driver's rows before and after its own tests, the other side's rows) and
//! by what it built before it knew (an index, a probe of the other side); tallied and printed as `ESEL` lines.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::collections::BTreeMap;

/// Where a select run ended (the last stage it reached).
pub(super) mod stage {
    pub(crate) const FILE_TEST: u8 = 1;
    pub(crate) const SEMI: u8 = 2;
    pub(crate) const SOURCE_EMPTY: u8 = 3;
    pub(crate) const SEED_EMPTY: u8 = 4;
    pub(crate) const BOUND_EMPTY: u8 = 5;
    pub(crate) const PENDSIZE_EMPTY: u8 = 6;
    pub(crate) const BUCKET_EMPTY: u8 = 7;
    pub(crate) const PENDING_EMPTY: u8 = 8;
    pub(crate) const DRIVER_FILTERED: u8 = 9;
    pub(crate) const OTHER_PROBED_EMPTY: u8 = 10;
    pub(crate) const KEYS_MISS: u8 = 11;
    pub(crate) const RESIDUAL: u8 = 12;
    pub(crate) const CROSS: u8 = 13;
    pub(crate) const ONE_SOURCE_FILTERED: u8 = 14;
    pub(crate) const KEYS_ABSENT: u8 = 15;
    pub(crate) const ZERO_ENTRY: u8 = 16;
    pub(crate) const BOUND_ZERO: u8 = 17;
    pub(crate) const NAMES: [&str; 18] = [
        "none marked",
        "a per-file test fails",
        "semi band empty",
        "a source made empty",
        "seed probe empty",
        "bound source made empty",
        "bounded driver made empty",
        "kind-bucket probe empty",
        "pending driver made empty",
        "driver's own tests reject every row",
        "other side probed by keys: no rows",
        "keys do not meet (index built, no hit)",
        "residual rejects every match",
        "cross product: nothing kept",
        "one source: its tests reject every row",
        "other side's keys absent where its probe ends (not probed)",
        "a view source's entry empty (decided before the front)",
        "bounded driver's bound is zero (not made)",
    ];
    /// What was built before the answer was known.
    pub(crate) const B_INDEX_FRESH: u8 = 1;
    pub(crate) const B_INDEX_FILE: u8 = 2;
    pub(crate) const B_PROBE: u8 = 4;
    pub(crate) const B_VIEW_PROBE: u8 = 8;
}

#[derive(Clone, Copy, Default)]
pub(super) struct Mark {
    pub(super) stage: u8,
    pub(super) built: u8,
    /// The driver's rows before its own tests, after them, and the other side's rows (u32::MAX: not known).
    pub(super) d0: u32,
    pub(super) d1: u32,
    pub(super) o: u32,
    /// The source a stage names (the source made empty: its position, then its node).
    pub(super) src: u8,
    pub(super) srcn: u32,
    /// The clock and the nested runs' seconds at the start, and the own seconds at each lap (cumulative; 0 not reached).
    pub(super) c0: f64,
    pub(super) i0: f64,
    pub(super) laps: [f32; 10],
    pub(super) node: u32,
}

thread_local! {
    static CUR: Cell<Mark> = const { Cell::new(Mark { stage: 0, built: 0, d0: u32::MAX, d1: u32::MAX, o: u32::MAX, src: 0, srcn: 0, c0: 0.0, i0: 0.0, laps: [0.0; 10], node: 0 }) };
    static TALLY: RefCell<BTreeMap<(u8, u8, &'static str, &'static str, &'static str, String), (u64, f64)>> = const { RefCell::new(BTreeMap::new()) };
    /// By stage: the runs that reached each lap, and their own seconds at it.
    static COUNTS: RefCell<BTreeMap<&'static str, (u64, f64)>> = const { RefCell::new(BTreeMap::new()) };
    static LAPS: RefCell<BTreeMap<u8, [(u64, f64); 10]>> = const { RefCell::new(BTreeMap::new()) };
}

#[inline]
pub(super) fn on() -> bool {
    sw!("VEC_ESEL_WHY")
}

/// A new select run starts: the enclosing run's mark, kept to restore.
#[inline]
pub(super) fn begin() -> Mark {
    CUR.with(|c| c.replace(Mark { d0: u32::MAX, d1: u32::MAX, o: u32::MAX, ..Mark::default() }))
}

/// The run's clock origin (`inner`: the executor's nested seconds now).
#[inline]
pub(super) fn origin(inner: f64) {
    with(|m| {
        m.c0 = super::step_clock();
        m.i0 = inner;
    });
}

/// A lap: the run's own seconds so far (`inner`: the executor's nested seconds now).
#[inline]
pub(super) fn lap(i: usize, inner: f64) {
    with(|m| m.laps[i] = ((super::step_clock() - m.c0) - (inner - m.i0)) as f32);
}

#[inline]
pub(super) fn at(stage: u8) {
    if on() {
        CUR.with(|c| {
            let mut m = c.get();
            m.stage = stage;
            c.set(m);
        });
    }
}

#[inline]
pub(super) fn with(g: impl FnOnce(&mut Mark)) {
    if on() {
        CUR.with(|c| {
            let mut m = c.get();
            g(&mut m);
            c.set(m);
        });
    }
}

fn bucket(n: u32) -> &'static str {
    match n {
        u32::MAX => "-",
        0 => "0",
        1 => "1",
        2..=4 => "2-4",
        5..=16 => "5-16",
        17..=64 => "17-64",
        65..=256 => "65-256",
        _ => ">256",
    }
}

/// The run ended; tallied when it returned nothing (`secs`: its own seconds, timing on). The enclosing mark restored.
pub(super) fn end(saved: Mark, empty: bool, secs: f64, op_of: impl Fn(u32) -> String) {
    let m = CUR.with(|c| c.replace(saved));
    if sw!("VEC_ESEL_NODES") && (m.node == 7070 || m.node == 7078) {
        count(Box::leak(format!("node {} empty {empty} stage {}", m.node, stage::NAMES[m.stage as usize]).into_boxed_str()), 0.0);
    }
    if !empty {
        return;
    }
    LAPS.with(|l| {
        let mut l = l.borrow_mut();
        let e = l.entry(m.stage).or_insert([(0, 0.0); 10]);
        for (i, x) in m.laps.iter().enumerate() {
            if *x > 0.0 {
                e[i].0 += 1;
                e[i].1 += f64::from(*x);
            }
        }
    });
    TALLY.with(|t| {
        let mut t = t.borrow_mut();
        let e = t.entry((m.stage, m.built, bucket(m.d0), bucket(m.d1), bucket(m.o), if m.stage == stage::SOURCE_EMPTY { format!("{} {}", m.src, op_of(m.srcn)) } else { String::new() })).or_default();
        e.0 += 1;
        e.1 += secs;
    });
}

/// A named count and its seconds (diagnostics).
pub(super) fn count(what: &'static str, secs: f64) {
    COUNTS.with(|c| {
        let mut c = c.borrow_mut();
        let e = c.entry(what).or_default();
        e.0 += 1;
        e.1 += secs;
    });
}

pub(crate) fn print() {
    if !on() {
        return;
    }
    COUNTS.with(|c| {
        for (k, (n, x)) in c.borrow().iter() {
            println!("ESELCOUNT\t{k}\t{n}\t{x:.3}");
        }
    });
    TALLY.with(|t| {
        let t = t.borrow();
        // by stage first
        let mut by: BTreeMap<u8, (u64, f64)> = BTreeMap::new();
        for ((s, ..), (n, x)) in t.iter() {
            let e = by.entry(*s).or_default();
            e.0 += n;
            e.1 += x;
        }
        for (s, (n, x)) in &by {
            println!("ESELSTAGE\t{}\t{n}\t{x:.3}", stage::NAMES[*s as usize]);
        }
        LAPS.with(|l| {
            for (s, e) in l.borrow().iter() {
                let laps: Vec<String> = e.iter().enumerate().filter(|(_, x)| x.0 > 0).map(|(i, x)| format!("L{i} {}k {:.0}ns", x.0 / 1000, x.1 * 1e9 / x.0 as f64)).collect();
                println!("ESELLAPS\t{}\t{}", stage::NAMES[*s as usize], laps.join("  "));
            }
        });
        let mut v: Vec<_> = t.iter().collect();
        v.sort_by_key(|(_, c)| std::cmp::Reverse(c.0));
        for ((s, b, d0, d1, o, x), (n, secs)) in v {
            println!("ESEL\t{}\tbuilt {b}\td0 {d0}\td1 {d1}\to {o}\t{x}\t{n}\t{secs:.3}", stage::NAMES[*s as usize]);
        }
    });
}

impl super::Exec<'_> {
    /// The select's entry from `Exec::body` (whole runs, and the runs a probe's substitution makes): the kernels
    /// (`select_kernel.rs`); `VEC_OLD_EMPTYSEL=1` the interpreter's `select`, as before (the oracle). Classed when it
    /// returns nothing (`VEC_ESEL_WHY`).
    pub(super) fn select_entry(&mut self, f: &mut super::File<'_>, run: &mut super::Run, p: &mut dyn super::Provider, node: usize) -> super::R<Rc<super::Table>> {
        if !on() {
            return self.select_entry_(f, run, p, node);
        }
        let saved = begin();
        with(|m| m.node = node as u32);
        let (t0, i0) = (super::step_clock(), self.inner);
        let out = self.select_entry_(f, run, p, node);
        let secs = super::step_clock() - t0 - (self.inner - i0);
        let empty = matches!(&out, Ok(t) if t.n == 0);
        let plan = self.plan;
        end(saved, empty, secs, |n| plan.nodes[n as usize].op.clone());
        out
    }

    #[inline]
    fn select_entry_(&mut self, f: &mut super::File<'_>, run: &mut super::Run, p: &mut dyn super::Provider, node: usize) -> super::R<Rc<super::Table>> {
        if sw!("VEC_OLD_EMPTYSEL") || sw!("VEC_OLD_SELECT") {
            let t = self.select(f, run, p, node)?;
            return if t.n == 0 { self.empty_rc(run, node) } else { Ok(super::recycle::rc(t)) };
        }
        self.select_kr(f, run, p, node)
    }
}
