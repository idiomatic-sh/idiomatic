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
    /// Nanoseconds of its runs in files where no rule that owns it emitted a row.
    pub dead_nanos: u64,
    /// Rows it held in files where no rule that owns it emitted a row.
    pub dead_rows: u64,
}

struct Local {
    stats: Vec<NodeStat>,
    stack: Vec<(u32, Instant, u64, bool)>,
    touched: Vec<u32>,
    /// What ran in the open file: node, its own nanoseconds, its rows.
    file: Vec<(u32, u64, u64)>,
}

thread_local! {
    static LOCAL: RefCell<Local> = const { RefCell::new(Local { stats: Vec::new(), stack: Vec::new(), touched: Vec::new(), file: Vec::new() }) };
}

static GLOBAL: Mutex<Vec<NodeStat>> = Mutex::new(Vec::new());
static DESCRIPTIONS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static OWNERS: Mutex<Vec<Vec<u32>>> = Mutex::new(Vec::new());
static EMITS: Mutex<Vec<u32>> = Mutex::new(Vec::new());

/// The node each rule emits, by rule ordinal (`u32::MAX` for none).
pub fn emits(emits: Vec<u32>) {
    if on() {
        *EMITS.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = emits;
    }
}

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

/// Whether the learned kill order is switched off, set once from `RYO_NO_KILLERS`.
pub fn killers_off() -> bool {
    static OFF: OnceLock<bool> = OnceLock::new();
    *OFF.get_or_init(|| std::env::var_os("RYO_NO_KILLERS").is_some())
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
        let own = whole.saturating_sub(inner);
        if !skipped {
            local.file.push((node, own, rows as u64));
        }
        let stat = slot(&mut local, node);
        let fresh = stat.runs == 0 && stat.skips == 0;
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
    flush_probes();
    LOCAL.with(|local| {
        let mut local = local.borrow_mut();
        // Which rules emitted a row in this file, and so which of the file's runs served a rule that found something.
        let ran = std::mem::take(&mut local.file);
        if !ran.is_empty() {
            let emits = EMITS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let owners = OWNERS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let mut held: std::collections::HashMap<u32, u64> = std::collections::HashMap::new();
            for (node, _, rows) in &ran {
                *held.entry(*node).or_default() += rows;
            }
            let live: Vec<bool> = emits
                .iter()
                .map(|emit| held.get(emit).copied().unwrap_or(0) > 0)
                .collect();
            for (node, nanos, rows) in &ran {
                let alive = owners
                    .get(*node as usize)
                    .is_some_and(|rules| rules.iter().any(|rule| live.get(*rule as usize).copied().unwrap_or(false)));
                if !alive {
                    let stat = slot(&mut local, *node);
                    stat.dead_nanos = stat.dead_nanos.saturating_add(*nanos);
                    stat.dead_rows += rows;
                }
            }
        }
        let mut ran = ran;
        ran.clear();
        local.file = ran;
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
            into.dead_nanos += from.dead_nanos;
            into.dead_rows += from.dead_rows;
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
    dump_keys();
    let Some(path) = std::env::var_os("RYO_STATS") else {
        return;
    };
    flush_probes();
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
            "{{\"node\":{node},\"nanos\":{},\"runs\":{},\"rows\":{},\"skips\":{},\"empty\":{},\"skip_nanos\":{},\"dead_nanos\":{},\"dead_rows\":{},\"rules\":{rules:?},\"desc\":{:?}}}\n",
            stat.nanos, stat.runs, stat.rows, stat.skips, stat.empty, stat.skip_nanos, stat.dead_nanos, stat.dead_rows, description
        ));
    }
    drop(std::fs::write(path, text));
}

/// Spike (ryo-jenl): whether the key census is on, set once from `RYO_KEYSTATS` (a file the census is written to).
pub fn keystats() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("RYO_KEYSTATS").is_some())
}

/// Spike (ryo-jenl): census sites: 0 index build, 1 index probe, 2 repeat check, 3 row-set build, 4 row-set ask, 5 text index,
/// 6 index rows keyed by words, 7 index probes by words.
/// Classes: 0 one node column, 1 two node columns, 2 one other column, 3 two columns otherwise, 4 wider.
static KEY_ROWS: [std::sync::atomic::AtomicU64; 40] = [const { std::sync::atomic::AtomicU64::new(0) }; 40];
/// Spike (ryo-jenl): single-node-column builds: rows by log2 of the id range, rows by log2 of range/rows, rows of builds
/// whose nodes span files, builds.
static KEY_RANGE: [std::sync::atomic::AtomicU64; 72] = [const { std::sync::atomic::AtomicU64::new(0) }; 72];

/// Spike (ryo-jenl): two-column index build rows by their kinds' pair (first x 6 + second, kinds in `CellKind` order).
static KEY_PAIRS: [std::sync::atomic::AtomicU64; 36] = [const { std::sync::atomic::AtomicU64::new(0) }; 36];

/// Spike (ryo-jenl): `rows` index build rows keyed by two columns of kinds `pair`.
pub fn key_pair(pair: [ryo_relations::domain::relations::types::CellKind; 2], rows: u64) {
    let at = (pair[0] as usize) * 6 + pair[1] as usize;
    if let Some(slot) = KEY_PAIRS.get(at) {
        slot.fetch_add(rows, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Spike (ryo-jenl): the census class of a key of `kinds`.
pub fn key_class(kinds: impl ExactSizeIterator<Item = ryo_relations::domain::relations::types::CellKind>) -> usize {
    use ryo_relations::domain::relations::types::CellKind;
    let width = kinds.len();
    let nodes = kinds.filter(|kind| *kind == CellKind::Node).count();
    match (width, nodes) {
        (1, 1) => 0,
        (2, 2) => 1,
        (1, _) => 2,
        (2, _) => 3,
        _ => 4,
    }
}

/// Spike (ryo-jenl): `rows` keyed rows at `site` of `class`.
pub fn key_rows(site: usize, class: usize, rows: u64) {
    if let Some(slot) = KEY_ROWS.get(site * 5 + class) {
        slot.fetch_add(rows, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Spike (ryo-jenl): one single-node-column build of `rows` rows whose positions span `range`, in one file or several.
pub fn key_range(range: u64, rows: u64, files: bool) {
    use std::sync::atomic::Ordering::Relaxed;
    let log = |value: u64| (64 - value.max(1).leading_zeros()) as usize;
    KEY_RANGE[log(range).min(31)].fetch_add(rows, Relaxed);
    KEY_RANGE[32 + log(range.div_ceil(rows.max(1))).min(31)].fetch_add(rows, Relaxed);
    if files {
        KEY_RANGE[64].fetch_add(rows, Relaxed);
    }
    KEY_RANGE[65].fetch_add(1, Relaxed);
    KEY_RANGE[66].fetch_add(rows, Relaxed);
}

/// Spike (ryo-jenl): writes the key census to the file `RYO_KEYSTATS` names.
pub fn dump_keys() {
    let Some(path) = std::env::var_os("RYO_KEYSTATS") else {
        return;
    };
    let rows: Vec<u64> = KEY_ROWS.iter().map(|slot| slot.load(std::sync::atomic::Ordering::Relaxed)).collect();
    let range: Vec<u64> = KEY_RANGE.iter().map(|slot| slot.load(std::sync::atomic::Ordering::Relaxed)).collect();
    let pairs: Vec<u64> = KEY_PAIRS.iter().map(|slot| slot.load(std::sync::atomic::Ordering::Relaxed)).collect();
    drop(std::fs::write(path, format!("{{\"rows\":{rows:?},\"range\":{range:?},\"pairs\":{pairs:?}}}\n")));
}

/// Spike (ryo-jenl): whether the dense-key mechanism named `name` (`RYO_DENSE_OFF` lists the ones switched off,
/// comma-separated, or `all`) is on: `words`, index keys that are one word hashed exactly; `repeats`, a repeat check of a
/// one-word row hashed by its word; `cellword`, a key's cells hashed one word each; `slots`, a one-word index whose words
/// span few places addressed by position; `pairs`, a two-cell index addressed by position on its first cell; `walks`, the
/// indexes of walks, closures, components and groups keyed the same way.
pub fn dense_lever(name: &str) -> bool {
    let at = match name {
        "words" => 0,
        "repeats" => 1,
        "cellword" => 2,
        "slots" => 3,
        "pairs" => 4,
        "walks" => 5,
        _ => return true,
    };
    dense_levers()[at]
}

/// Spike (ryo-jenl): the dense-key switches, read once from `RYO_DENSE_OFF`.
#[inline]
pub fn dense_levers() -> &'static [bool; 6] {
    static ON: OnceLock<[bool; 6]> = OnceLock::new();
    ON.get_or_init(|| {
        let off = std::env::var("RYO_DENSE_OFF").unwrap_or_default();
        let on = |name: &str| !off.split(',').any(|item| item == name || item == "all");
        [on("words"), on("repeats"), on("cellword"), on("slots"), on("pairs"), on("walks")]
    })
}
