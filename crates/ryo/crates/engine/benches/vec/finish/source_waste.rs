//! Diagnostic (`VEC_SRC_WASTE=1`, off costs one cached switch test): the thread CPU a select spends making each of its
//! sources, split by how the run ended, and what `keyed_size` spends. Printed at the end of a session.
use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};

#[derive(Default)]
struct Frame {
    sources: Vec<(usize, f64)>,
    sized: f64,
    proving: Option<usize>,
}

#[derive(Default)]
struct State {
    stack: Vec<Frame>,
    // class -> [runs, source secs before proving, proving source secs, all source secs, keyed_size secs]
    classes: BTreeMap<&'static str, [f64; 5]>,
    // (make, outcome) -> [calls, secs]
    sizes: BTreeMap<(bool, &'static str), [f64; 2]>,
    seen: HashSet<(usize, u32)>,
    repeats: f64,
    repeat_secs: f64,
    sized_calls: f64,
}

thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }

#[inline]
pub(super) fn on() -> bool {
    sw!("VEC_SRC_WASTE")
}

pub(super) fn enter() {
    STATE.with_borrow_mut(|s| s.stack.push(Frame::default()));
}

pub(super) fn source(index: usize, secs: f64) {
    STATE.with_borrow_mut(|s| {
        if let Some(f) = s.stack.last_mut() {
            f.sources.push((index, secs));
        }
    });
}

pub(super) fn prove(index: usize) {
    STATE.with_borrow_mut(|s| {
        if let Some(f) = s.stack.last_mut() {
            f.proving = Some(index);
        }
    });
}

pub(super) fn sized(node: usize, file: u32, make: bool, outcome: &'static str, secs: f64) {
    STATE.with_borrow_mut(|s| {
        s.sized_calls += 1.0;
        if !s.seen.insert((node, file)) {
            s.repeats += 1.0;
            s.repeat_secs += secs;
        }
        let e = s.sizes.entry((make, outcome)).or_default();
        e[0] += 1.0;
        e[1] += secs;
        if let Some(f) = s.stack.last_mut() {
            f.sized += secs;
        }
    });
}

pub(super) fn leave(rows: usize) {
    STATE.with_borrow_mut(|s| {
        let Some(f) = s.stack.pop() else { return };
        let all: f64 = f.sources.iter().map(|x| x.1).sum();
        let (class, before, proof) = if rows > 0 {
            ("answered rows", 0.0, 0.0)
        } else if let Some(p) = f.proving {
            let before: f64 = f.sources.iter().filter(|x| x.0 != p).map(|x| x.1).sum();
            let proof: f64 = f.sources.iter().filter(|x| x.0 == p).map(|x| x.1).sum();
            ("no row: a made source was empty", before, proof)
        } else {
            ("no row: other end", 0.0, 0.0)
        };
        let e = s.classes.entry(class).or_default();
        e[0] += 1.0;
        e[1] += before;
        e[2] += proof;
        e[3] += all;
        e[4] += f.sized;
    });
}

pub(crate) fn print() {
    if !on() {
        return;
    }
    STATE.with_borrow(|s| {
        for (k, v) in &s.classes {
            println!("SRCWASTE\t{k}\truns {}\tbefore_proving_ms {:.1}\tproving_ms {:.1}\tall_sources_ms {:.1}\tkeyed_size_ms {:.1}", v[0], v[1] * 1e3, v[2] * 1e3, v[3] * 1e3, v[4] * 1e3);
        }
        for ((make, outcome), v) in &s.sizes {
            println!("SIZED\tmake {make}\t{outcome}\tcalls {}\tms {:.1}", v[0], v[1] * 1e3);
        }
        println!("SIZEDTOTAL\tcalls {}\trepeats_same_step_file {}\trepeat_ms {:.1}", s.sized_calls, s.repeats, s.repeat_secs * 1e3);
    });
}
