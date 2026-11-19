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

/// Whether every node is made on its own, never fused into a chain, set once from `RYO_HELD`.
pub fn held_only() -> bool {
    static HELD: OnceLock<bool> = OnceLock::new();
    *HELD.get_or_init(|| std::env::var_os("RYO_HELD").is_some())
}

/// Whether the probe lever named `name` (`RYO_PROBE_OFF` lists the ones switched off, comma-separated, or `all`) is on:
/// `fastkey`, probe index builds and lookups reading each side by its own kind's reader.
pub fn probe_lever(name: &str) -> bool {
    static OFF: OnceLock<String> = OnceLock::new();
    !OFF.get_or_init(|| std::env::var("RYO_PROBE_OFF").unwrap_or_default())
        .split(',')
        .any(|off| off == name || off == "all")
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
            add_held(rows as u64);
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
    flush_probes();
    flush_census();
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

/// Spike (ryo-fmx3): what one probe stage cost across the run: its own time in its chunk calls, its runs, the left rows it
/// read, the right rows of its runs, and its time split by the shape of the call (right >= 4 x left; left x right < 64).
#[derive(Clone, Copy, Debug, Default)]
pub struct ProbeStat {
    /// Nanoseconds in the stage's chunk calls.
    pub nanos: u64,
    /// Runs (a run readies the right side once).
    pub runs: u64,
    /// Calls.
    pub calls: u64,
    /// Left rows read.
    pub left: u64,
    /// Right rows over its runs.
    pub right: u64,
    /// Rows written.
    pub out: u64,
    /// Nanoseconds in calls whose right was at least four times its left.
    pub big_right: u64,
    /// Nanoseconds in calls whose left times right was under 64.
    pub tiny: u64,
    /// Nanoseconds in the right side's preparation (filter, index build).
    pub prep: u64,
    /// Right rows indexed by a build.
    pub built: u64,
    /// Builds.
    pub builds: u64,
    /// Index kind: 0 own, 1 shared, 2 tree; 4 is added when the probe filters its right rows.
    pub kind: u64,
}

thread_local! {
    static PROBES: RefCell<(Vec<ProbeStat>, Vec<u32>)> = const { RefCell::new((Vec::new(), Vec::new())) };
}
static GLOBAL_PROBES: Mutex<Vec<ProbeStat>> = Mutex::new(Vec::new());

/// Spike (ryo-fmx3): one chunk call of probe stage `node`.
pub fn probe_prep(node: u32, nanos: u64, built: u64, kind: u64) {
    PROBES.with(|probes| {
        let (stats, _) = &mut *probes.borrow_mut();
        let at = node as usize;
        if stats.len() <= at {
            stats.resize(at + 1, ProbeStat::default());
        }
        let stat = &mut stats[at];
        stat.prep += nanos;
        if built > 0 {
            stat.builds += 1;
            stat.built += built;
        }
        stat.kind = kind;
    });
}

/// Spike (ryo-fmx3): one chunk call of probe stage `node`.
pub fn probe_call(node: u32, nanos: u64, first: bool, left: u64, right: u64, out: u64) {
    PROBES.with(|probes| {
        let (stats, touched) = &mut *probes.borrow_mut();
        let at = node as usize;
        if stats.len() <= at {
            stats.resize(at + 1, ProbeStat::default());
        }
        let stat = &mut stats[at];
        if stat.calls == 0 {
            touched.push(node);
        }
        stat.calls += 1;
        stat.nanos += nanos;
        stat.left += left;
        stat.out += out;
        if first {
            stat.runs += 1;
            stat.right += right;
        }
        if right >= left.saturating_mul(4) {
            stat.big_right += nanos;
        }
        if left.saturating_mul(right) < 64 {
            stat.tiny += nanos;
        }
    });
}

fn flush_probes() {
    PROBES.with(|probes| {
        let (stats, touched) = &mut *probes.borrow_mut();
        if touched.is_empty() {
            return;
        }
        let mut global = GLOBAL_PROBES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        for node in touched.drain(..) {
            let at = node as usize;
            if global.len() <= at {
                global.resize(at + 1, ProbeStat::default());
            }
            let from = std::mem::take(&mut stats[at]);
            let into = &mut global[at];
            into.nanos += from.nanos;
            into.runs += from.runs;
            into.calls += from.calls;
            into.left += from.left;
            into.right += from.right;
            into.out += from.out;
            into.big_right += from.big_right;
            into.tiny += from.tiny;
            into.prep += from.prep;
            into.built += from.built;
            into.builds += from.builds;
            into.kind = from.kind;
        }
    });
}

/// Writes the totals to the file `RYO_STATS` names, one JSON object per node that ran or was skipped.
pub fn dump() {
    bound_report();
    let Some(path) = std::env::var_os("RYO_STATS") else {
        return;
    };
    flush_probes();
    flush_census();
    dump_census();
    {
        let probes = GLOBAL_PROBES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut text = String::new();
        for (node, stat) in probes.iter().enumerate() {
            if stat.calls == 0 {
                continue;
            }
            text.push_str(&format!(
                "{{\"node\":{node},\"nanos\":{},\"runs\":{},\"calls\":{},\"left\":{},\"right\":{},\"out\":{},\"big_right\":{},\"tiny\":{},\"prep\":{},\"built\":{},\"builds\":{},\"kind\":{}}}\n",
                stat.nanos, stat.runs, stat.calls, stat.left, stat.right, stat.out, stat.big_right, stat.tiny, stat.prep, stat.built, stat.builds, stat.kind
            ));
        }
        let mut probe_path = path.clone();
        probe_path.push(".probes");
        drop(std::fs::write(probe_path, text));
    }
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

// Spike (ryo-22c7): rows held by the node runs of this thread, for the bound census (recorder only).
thread_local! {
    static HELD_ROWS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static CENSUS_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    static CENSUS: RefCell<Vec<[u64; 8]>> = const { RefCell::new(Vec::new()) };
}
static GLOBAL_CENSUS: Mutex<Vec<[u64; 8]>> = Mutex::new(Vec::new());

/// Spike (ryo-22c7): rows held so far by this thread's node runs (recorder only).
pub fn held_rows() -> u64 {
    HELD_ROWS.with(std::cell::Cell::get)
}

/// Spike (ryo-22c7): counts `rows` held by a node run.
pub fn add_held(rows: u64) {
    HELD_ROWS.with(|held| held.set(held.get() + rows));
}

/// Spike (ryo-22c7): a census span starts; answers whether it is the outermost one.
pub fn census_enter() -> bool {
    CENSUS_DEPTH.with(|depth| {
        let outer = depth.get() == 0;
        if outer {
            depth.set(1);
        }
        outer
    })
}

/// Spike (ryo-22c7): a census span ends.
pub fn census_leave() {
    CENSUS_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
}

/// Spike (ryo-22c7): one probe readied an unmade right side: [requests, left rows, left distinct keys, nanos, rows held,
/// requests with at most 64 distinct keys, their nanos, their rows held].
pub fn census(node: u32, left: u64, keys: u64, nanos: u64, rows: u64) {
    CENSUS.with(|census| {
        let mut census = census.borrow_mut();
        let at = node as usize;
        if census.len() <= at {
            census.resize(at + 1, [0; 8]);
        }
        let slot = &mut census[at];
        slot[0] += 1;
        slot[1] += left;
        slot[2] += keys;
        slot[3] += nanos;
        slot[4] += rows;
        if keys <= 64 {
            slot[5] += 1;
            slot[6] += nanos;
            slot[7] += rows;
        }
    });
}

/// Spike (ryo-22c7): writes the census beside the recorder's file.
pub fn dump_census() {
    let Some(path) = std::env::var_os("RYO_STATS") else {
        return;
    };
    let mut global = GLOBAL_CENSUS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut text = String::new();
    for (node, slot) in global.iter().enumerate() {
        if slot[0] == 0 {
            continue;
        }
        text.push_str(&format!(
            "{{\"node\":{node},\"requests\":{},\"left\":{},\"keys\":{},\"nanos\":{},\"rows\":{},\"small\":{},\"small_nanos\":{},\"small_rows\":{}}}\n",
            slot[0], slot[1], slot[2], slot[3], slot[4], slot[5], slot[6], slot[7]
        ));
    }
    global.clear();
    let mut census_path = path;
    census_path.push(".bound");
    drop(std::fs::write(census_path, text));
}

fn flush_census() {
    CENSUS.with(|census| {
        let mut census = census.borrow_mut();
        if census.is_empty() {
            return;
        }
        let mut global = GLOBAL_CENSUS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if global.len() < census.len() {
            global.resize(census.len(), [0; 8]);
        }
        for (into, from) in global.iter_mut().zip(census.iter_mut()) {
            for (a, b) in into.iter_mut().zip(from.iter_mut()) {
                *a += *b;
                *b = 0;
            }
        }
    });
}

/// Spike (ryo-22c7): whether restricted evaluation is on (`RYO_BOUND=1`; off by default, as it measured no net gain; needs
/// `RYO_LAZY`).
pub fn bound_on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| lazy() && std::env::var_os("RYO_BOUND").is_some_and(|value| value != "0"))
}

/// Spike (ryo-22c7): whether every restricted request is checked against the node made in full (`RYO_BOUND_CHECK`).
pub fn bound_check() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("RYO_BOUND_CHECK").is_some_and(|value| value != "0"))
}

/// Spike (ryo-22c7): whether restricted evaluation reaches shape `level` (`RYO_BOUND_SHAPES`, default all): 0 row-wise
/// nodes over relations, 1 probes, 2 distinct and group.
pub fn bound_shape(level: u32) -> bool {
    static MOST: OnceLock<u32> = OnceLock::new();
    level <= *MOST.get_or_init(|| std::env::var("RYO_BOUND_SHAPES").ok().and_then(|value| value.parse().ok()).unwrap_or(9))
}

static BOUND_NANOS: [std::sync::atomic::AtomicU64; 2] = [std::sync::atomic::AtomicU64::new(0), std::sync::atomic::AtomicU64::new(0)];

/// Spike (ryo-22c7): adds the time since `started` to counter `at`: 0 restricted requests, 1 nodes made in full under them.
pub fn bound_time(at: usize, started: std::time::Instant) {
    if let Some(counter) = BOUND_NANOS.get(at) {
        counter.fetch_add(u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX), std::sync::atomic::Ordering::Relaxed);
    }
}

static BOUND_SERVED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static BOUND_ROWS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Spike (ryo-22c7): rows the restricted requests answered (the recorder does not count them).
pub fn bound_rows(rows: u64) {
    BOUND_ROWS.fetch_add(rows, std::sync::atomic::Ordering::Relaxed);
}
static BOUND_FELL: Mutex<Vec<(&'static str, u64)>> = Mutex::new(Vec::new());

/// Spike (ryo-22c7): a restricted request was served.
pub fn bound_served() {
    BOUND_SERVED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// Spike (ryo-22c7): a node under a restricted request was made in full, by its lowering.
pub fn bound_fell(lowering: &'static str) {
    let mut fell = BOUND_FELL.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    match fell.iter_mut().find(|(name, _)| *name == lowering) {
        Some((_, count)) => *count += 1,
        None => fell.push((lowering, 1)),
    }
}

/// Spike (ryo-22c7): appends the restricted-evaluation counters to the file `RYO_BOUND_REPORT` names.
pub fn bound_report() {
    let Some(path) = std::env::var_os("RYO_BOUND_REPORT") else {
        return;
    };
    let fell = BOUND_FELL.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let fell: Vec<String> = fell.iter().map(|(name, count)| format!("\"{name}\":{count}")).collect();
    let line = format!(
        "{{\"restricted_rows\":{},\"restricted_requests\":{},\"restricted_s\":{:.2},\"full_under_s\":{:.2},\"fallbacks_by_lowering\":{{{}}}}}\n",
        BOUND_ROWS.load(std::sync::atomic::Ordering::Relaxed),
        BOUND_SERVED.load(std::sync::atomic::Ordering::Relaxed),
        BOUND_NANOS[0].load(std::sync::atomic::Ordering::Relaxed) as f64 / 1e9,
        BOUND_NANOS[1].load(std::sync::atomic::Ordering::Relaxed) as f64 / 1e9,
        fell.join(",")
    );
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        drop(file.write_all(line.as_bytes()));
    }
    let mut costs_path = path;
    costs_path.push(".costs");
    drop(std::fs::write(costs_path, crate::domain::physical_plan::service::run::bound::service::costs_text()));
}

/// Spike (ryo-22c7): a restricted-evaluation knob read once from the environment: `RYO_BOUND_KEYS` (most distinct keys,
/// default 64), `RYO_BOUND_FULL_AT` (the request at which a node is made in full, default 3), `RYO_BOUND_READERS` (most
/// readers of a node restricted below the top, default unbounded), `RYO_BOUND_TOP_READERS` (most readers of a top node).
pub fn bound_knob(at: usize) -> usize {
    static KNOBS: OnceLock<[usize; 8]> = OnceLock::new();
    let knobs = KNOBS.get_or_init(|| {
        let read = |name: &str, default: usize| std::env::var(name).ok().and_then(|value| value.parse().ok()).unwrap_or(default);
        [
            read("RYO_BOUND_KEYS", 64),
            read("RYO_BOUND_FULL_AT", 3),
            read("RYO_BOUND_READERS", usize::MAX),
            read("RYO_BOUND_TOP_READERS", usize::MAX),
            read("RYO_BOUND_LEARN", 2),
            read("RYO_BOUND_RATIO", 80),
            read("RYO_BOUND_WASTE", 34),
            read("RYO_BOUND_MODES", 7),
        ]
    });
    knobs.get(at).copied().unwrap_or(usize::MAX)
}
