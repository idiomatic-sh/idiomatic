//! Spike (ryo-fmx3) instrumentation: per plan node, the time its kernels took less the nodes they made first, how often it ran,
//! the rows it held and how often it answered empty without running. On when `RYO_STATS` names a file; written by `dump`.
use std::cell::RefCell;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

/// What one node cost across the run.
#[derive(Clone, Copy, Debug, Default)]
pub struct NodeStat {
    /// Nanoseconds in the node's own kernels.
    pub nanos: u64,
    /// How many files ran it.
    pub runs: u64,
    /// Rows it held.
    pub rows: u64,
    /// Files in which it answered empty without running.
    pub skips: u64,
    /// Runs that held no row.
    pub empty: u64,
    /// Nanoseconds spent answering empty without running.
    pub skip_nanos: u64,
}

struct Local {
    stats: Vec<NodeStat>,
    stack: Vec<(u32, Instant, u64, bool)>,
    touched: Vec<u32>,
}

thread_local! {
    static LOCAL: RefCell<Local> = const { RefCell::new(Local { stats: Vec::new(), stack: Vec::new(), touched: Vec::new() }) };
}

static GLOBAL: Mutex<Vec<NodeStat>> = Mutex::new(Vec::new());
static DESCRIPTIONS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static OWNERS: Mutex<Vec<Vec<u32>>> = Mutex::new(Vec::new());

/// Whether evaluation is demand-driven, set once from `RYO_LAZY`.
pub fn lazy() -> bool {
    static LAZY: OnceLock<bool> = OnceLock::new();
    *LAZY.get_or_init(|| std::env::var_os("RYO_LAZY").is_some_and(|value| value != "0"))
}

/// Whether the entity-query answers are switched off, set once from `RYO_NO_ENTITY`.
pub fn entity_off() -> bool {
    static OFF: OnceLock<bool> = OnceLock::new();
    *OFF.get_or_init(|| std::env::var_os("RYO_NO_ENTITY").is_some())
}

/// Spike (ryo-fmx3): whether the leaf guards are switched off, set once from `RYO_NO_GUARDS` (read once, not per node run).
pub fn guards_off() -> bool {
    static OFF: OnceLock<bool> = OnceLock::new();
    *OFF.get_or_init(|| std::env::var_os("RYO_NO_GUARDS").is_some())
}

/// Whether the recorder is on.
pub fn on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("RYO_STATS").is_some())
}

/// Each node's kernel, as text, by node.
pub fn describe(descriptions: Vec<String>) {
    if on() {
        *DESCRIPTIONS.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = descriptions;
    }
}

/// The rule ordinals that read each node, by node.
pub fn owners(owners: Vec<Vec<u32>>) {
    if on() {
        *OWNERS.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = owners;
    }
}

fn slot(local: &mut Local, node: u32) -> &mut NodeStat {
    let at = node as usize;
    if local.stats.len() <= at {
        local.stats.resize(at + 1, NodeStat::default());
    }
    &mut local.stats[at]
}

/// A node's kernels start.
pub fn enter(node: u32) {
    if on() {
        LOCAL.with(|local| local.borrow_mut().stack.push((node, Instant::now(), 0, false)));
    }
}

/// A node's kernels end, holding `rows`.
pub fn leave(rows: usize) {
    if !on() {
        return;
    }
    LOCAL.with(|local| {
        let mut local = local.borrow_mut();
        let Some((node, started, inner, skipped)) = local.stack.pop() else {
            return;
        };
        let whole = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
        if let Some(parent) = local.stack.last_mut() {
            parent.2 = parent.2.saturating_add(whole);
        }
        let stat = slot(&mut local, node);
        let fresh = stat.runs == 0 && stat.skips == 0;
        let own = whole.saturating_sub(inner);
        if skipped {
            stat.skips += 1;
            stat.skip_nanos = stat.skip_nanos.saturating_add(own);
        } else {
            stat.nanos = stat.nanos.saturating_add(own);
            stat.runs += 1;
            stat.rows += rows as u64;
            if rows == 0 {
                stat.empty += 1;
            }
        }
        if fresh {
            local.touched.push(node);
        }
    });
}

/// The node whose kernels are running answered empty without running them.
pub fn skip(node: u32) {
    if !on() {
        return;
    }
    LOCAL.with(|local| {
        let mut local = local.borrow_mut();
        if let Some(top) = local.stack.last_mut()
            && top.0 == node
        {
            top.3 = true;
            return;
        }
        let stat = slot(&mut local, node);
        let fresh = stat.runs == 0 && stat.skips == 0;
        stat.skips += 1;
        if fresh {
            local.touched.push(node);
        }
    });
}

/// Gives this thread's counts to the run's totals; called at each file's end.
pub fn flush() {
    if !on() {
        return;
    }
    LOCAL.with(|local| {
        let mut local = local.borrow_mut();
        if local.touched.is_empty() {
            return;
        }
        let mut global = GLOBAL.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let touched = std::mem::take(&mut local.touched);
        for node in &touched {
            let at = *node as usize;
            if global.len() <= at {
                global.resize(at + 1, NodeStat::default());
            }
            let from = std::mem::take(&mut local.stats[at]);
            let into = &mut global[at];
            into.nanos += from.nanos;
            into.runs += from.runs;
            into.rows += from.rows;
            into.skips += from.skips;
            into.empty += from.empty;
            into.skip_nanos += from.skip_nanos;
        }
        local.touched = touched;
        local.touched.clear();
    });
}

/// Writes the totals to the file `RYO_STATS` names, one JSON object per node that ran or was skipped.
pub fn dump() {
    let Some(path) = std::env::var_os("RYO_STATS") else {
        return;
    };
    let global = GLOBAL.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let descriptions = DESCRIPTIONS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let owners = OWNERS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut text = String::new();
    for (node, stat) in global.iter().enumerate() {
        if stat.runs == 0 && stat.skips == 0 {
            continue;
        }
        let description = descriptions.get(node).map_or("", String::as_str);
        let rules = owners.get(node).map_or(&[][..], Vec::as_slice);
        text.push_str(&format!(
            "{{\"node\":{node},\"nanos\":{},\"runs\":{},\"rows\":{},\"skips\":{},\"empty\":{},\"skip_nanos\":{},\"rules\":{rules:?},\"desc\":{:?}}}\n",
            stat.nanos, stat.runs, stat.rows, stat.skips, stat.empty, stat.skip_nanos, description
        ));
    }
    drop(std::fs::write(path, text));
}
