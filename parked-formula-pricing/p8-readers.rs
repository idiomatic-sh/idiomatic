//! Auxiliary observation for the price-comparison builds.
use super::super::{pair_summary::Fact, thread_cpu};
use rustc_hash::FxHashMap;
use std::{cell::RefCell, fmt::Write, sync::OnceLock};

#[derive(Default)]
struct Reads {
    files: u64,
    checks: f64,
    relations: FxHashMap<&'static str, (&'static str, u64, f64)>,
}

thread_local! { static READS: RefCell<Reads> = RefCell::default(); }

#[inline]
pub(crate) fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("P8_READERS").is_some())
}

#[inline]
pub(crate) fn start() -> f64 {
    if enabled() { thread_cpu() } else { 0.0 }
}

pub(super) fn provider(spec: &Fact, start: f64) {
    if start != 0.0 {
        let elapsed = thread_cpu() - start;
        READS.with_borrow_mut(|r| {
            let slot = r.relations.entry(spec.name).or_insert((spec.family.name(), 0, 0.0));
            slot.1 += 1;
            slot.2 += elapsed;
        });
    }
}

pub(super) fn file(start: f64) {
    if start != 0.0 {
        let elapsed = thread_cpu() - start;
        READS.with_borrow_mut(|r| {
            r.files += 1;
            r.checks += elapsed;
        });
    }
}

pub(crate) fn report(extra: Option<String>) {
    if !enabled() { return; }
    READS.with_borrow_mut(|r| {
        let mut output = format!("C\t{}\t{:.9}\n", r.files, r.checks);
        let mut names: Vec<_> = r.relations.keys().copied().collect();
        names.sort_unstable();
        for name in names {
            let (family, reads, cpu) = r.relations[name];
            writeln!(output, "T\t{name}\t{family}\t{reads}\t{cpu:.9}").unwrap();
        }
        if let Some(extra)=extra { output.push_str(&extra); }
        std::fs::write(std::env::var_os("P8_CAPTURE").unwrap(), output).unwrap();
        *r = Reads::default();
    });
}


pub(crate) fn elapsed(start:f64)->f64 { if start==0. {0.} else { thread_cpu()-start } }

pub(crate) fn training_clock() -> f64 {
    static ON: OnceLock<bool> = OnceLock::new();
    if *ON.get_or_init(||std::env::var_os("P8_TRAIN_OBSERVER").is_some()) {thread_cpu()} else {0.}
}
