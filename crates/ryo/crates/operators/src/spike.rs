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

/// Spike (ryo-fmx3, host3): a host lever, each a mechanism of host work that scales with the file, not the rule pack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostLever {
    /// `roots`: a node's tree roots listed per root kind for the kinds the file holds.
    Roots = 0,
    /// `quiet`: a rule whose emitted and observed nodes hold no row in the file answers the shared clean outcome.
    Quiet = 1,
    /// `record`: a rule's recorded frontier digest reused when its words are the ones last digested.
    Record = 2,
    /// `facts`: provider facts produced on the first read of a relation in the file.
    Facts = 3,
    /// `buffers`: buffer reset proportional to the nodes touched.
    Buffers = 4,
    /// `startup`: startup work made cheaper or parallel.
    Startup = 5,
    /// `q-room`: part of `quiet`: rule records written into a room kept per worker thread.
    QuietRoom = 6,
    /// `q-outputs`: part of `quiet`: named outputs written into last file's room, readiness without a copy, whole files
    /// admitted without a policy.
    QuietOutputs = 7,
    /// `q-frontier`: part of `quiet`: a frontier that is a named output read from the answer made in the file.
    QuietFrontier = 8,
    /// `q-whole`: part of `quiet`: a node of a file read whole run with no completion policy.
    QuietWhole = 9,
    /// `q-emit`: part of `quiet`: an empty complete emitted root withheld without a copy.
    QuietEmit = 10,
    /// `frames`: a rule that found nothing in a file is kept as one clean file of its buckets, not one frame per file.
    Frames = 11,
}

/// Spike (ryo-fmx3, host3): whether host lever `lever` is on; `RYO_HOST_OFF` names the ones off (comma-separated, `roots`,
/// `quiet`, `record`, `facts`, `buffers`, `startup`, or `all`), read once.
pub fn host_lever(lever: HostLever) -> bool {
    static ON: OnceLock<[bool; 12]> = OnceLock::new();
    let on = ON.get_or_init(|| {
        let off = std::env::var("RYO_HOST_OFF").unwrap_or_default();
        let names = [
            "roots", "quiet", "record", "facts", "buffers", "startup", "q-room", "q-outputs", "q-frontier", "q-whole",
            "q-emit", "frames",
        ];
        let mut on = names.map(|name| !off.split(',').any(|each| each == name || each == "all"));
        // A part of `quiet` is on only while `quiet` is.
        for part in 6..=10 {
            on[part] = on[part] && on[1];
        }
        on
    });
    on.get(lever as usize).copied().unwrap_or(true)
}

/// Whether the learned kill order is switched off, set once from `RYO_NO_KILLERS`.
pub fn killers_off() -> bool {
    static OFF: OnceLock<bool> = OnceLock::new();
    *OFF.get_or_init(|| std::env::var_os("RYO_NO_KILLERS").is_some())
}

/// Spike (ryo-fmx3): whether sibling constant filters read one partition of their source (`RYO_NO_FANOUT` switches it off),
/// set once.
pub fn fanout() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("RYO_NO_FANOUT").is_none())
}

/// Spike (ryo-fmx3): whether the filters `keyed_select` would make lookups of one shared index (each build answering every
/// lookup of it) are left filters that read one partition instead, made once and read only by the siblings asked for
/// (`RYO_NO_FANOUT_LOOKUPS` keeps the lookups), set once. Only demand-driven runs make partitions, so only they drop the lookups.
pub fn fanout_lookups() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| fanout() && lazy() && std::env::var_os("RYO_NO_FANOUT_LOOKUPS").is_none())
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

/// Spike (ryo-8y7q): whether the memory counters are on, set once from `RYO_MEMSTATS`.
pub fn memstats() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("RYO_MEMSTATS").is_some())
}

/// Spike (ryo-8y7q): over the run, the files ended, the table bytes still held at their ends (sum and largest), and of
/// those the bytes of tables no reader and no engine use was still waiting for.
static END_HELD: Mutex<(u64, u64, u64, u64)> = Mutex::new((0, 0, 0, 0));

/// Spike (ryo-8y7q): one file ended holding `held` table bytes, `idle` of them in tables no consumer still waited for.
pub fn end_held(held: u64, idle: u64) {
    let mut totals = END_HELD.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    totals.0 += 1;
    totals.1 += held;
    totals.2 = totals.2.max(held);
    totals.3 += idle;
}

/// Spike (ryo-8y7q): writes the memory counters to the file `RYO_MEMSTATS` names.
pub fn memstats_dump() {
    if !memstats() {
        return;
    }
    let totals = *END_HELD.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(path) = std::env::var_os("RYO_MEMSTATS") else {
        return;
    };
    let line = format!(
        "MEMSTATS files={} end_held_sum_mib={:.1} end_held_max_mib={:.1} end_idle_sum_mib={:.1} tables_peak_mib={:.1}\n",
        totals.0,
        totals.1 as f64 / 1048576.0,
        totals.2 as f64 / 1048576.0,
        totals.3 as f64 / 1048576.0,
        tables_peak_mib()
    );
    let remade = REMADE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let line = format!("{line}REMADE {} {:?}\n", remade.0, remade.1);
    drop(std::fs::write(path, line));
}

/// Spike (ryo-8y7q): the bytes of source the files read at once may hold, from `RYO_ADMISSION` when set, read once.
pub fn admission_bytes() -> Option<u64> {
    static BYTES: OnceLock<Option<u64>> = OnceLock::new();
    *BYTES.get_or_init(|| std::env::var("RYO_ADMISSION").ok().and_then(|value| value.parse().ok()))
}

/// Spike (ryo-8y7q): the node table bytes all workers hold now, and the most they held at once.
static TABLES_NOW: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
static TABLES_PEAK: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

/// Spike (ryo-8y7q): a node table of `bytes` was held (positive) or let go (negative).
pub fn tables_held(bytes: i64) {
    use std::sync::atomic::Ordering::Relaxed;
    let now = TABLES_NOW.fetch_add(bytes, Relaxed) + bytes;
    TABLES_PEAK.fetch_max(now, Relaxed);
}

fn tables_peak_mib() -> f64 {
    TABLES_PEAK.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1048576.0
}

/// Spike (ryo-8y7q): whether the session keeps its compiled plans past a run's preparation, set once from `RYO_KEEP_PLANS`.
pub fn keep_plans() -> bool {
    static KEEP: OnceLock<bool> = OnceLock::new();
    *KEEP.get_or_init(|| std::env::var_os("RYO_KEEP_PLANS").is_some())
}

/// Spike (ryo-8y7q): whether the release of nodes at their last rule turn is off, set once from `RYO_NO_TURNS`.
pub fn turns_off() -> bool {
    static OFF: OnceLock<bool> = OnceLock::new();
    *OFF.get_or_init(|| std::env::var_os("RYO_NO_TURNS").is_some())
}

/// Spike (ryo-8y7q): nodes made again in a file after a release: how many, and the first few (node, turn, last turn).
static REMADE: Mutex<(u64, Vec<(u32, u32, u32)>)> = Mutex::new((0, Vec::new()));

/// Spike (ryo-8y7q): `node` was made again in the open file, in rule turn `turn`; its last turn is `last`.
pub fn remade(node: u32, turn: u32, last: u32) {
    let mut remade = REMADE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    remade.0 += 1;
    if remade.1.len() < 40 {
        remade.1.push((node, turn, last));
    }
}

/// Spike (ryo-jenl): whether the key census is on, set once from `RYO_KEYSTATS` (a file the census is written to).
#[inline]
pub fn keystats() -> bool {
    static ON: Switch = Switch(std::sync::atomic::AtomicU8::new(0));
    switch(&ON.0, || std::env::var_os("RYO_KEYSTATS").is_some())
}

/// Spike (ryo-jenl): a switch's byte on a cache line of its own, so no counter written beside it makes its read a miss.
#[repr(align(64))]
pub struct Switch(std::sync::atomic::AtomicU8);

/// Spike (ryo-jenl): a switch read once into a byte (0 not read yet, 1 off, 2 on), so a hot path's check is one plain load.
#[inline]
fn switch(cell: &std::sync::atomic::AtomicU8, read: impl FnOnce() -> bool) -> bool {
    use std::sync::atomic::Ordering::Relaxed;
    match cell.load(Relaxed) {
        2 => true,
        1 => false,
        _ => {
            let on = read();
            cell.store(if on { 2 } else { 1 }, Relaxed);
            on
        }
    }
}

/// Spike (ryo-jenl): whether a key's cells hash one word each (`cellword` in `RYO_DENSE_OFF`), as one plain load.
#[inline]
pub fn cellword() -> bool {
    static ON: Switch = Switch(std::sync::atomic::AtomicU8::new(0));
    switch(&ON.0, || dense_levers()[2])
}

/// Spike (ryo-jenl): whether a walk meets nodes by position (`visited` in `RYO_DENSE_OFF`), as one plain load.
#[inline]
pub fn visited_on() -> bool {
    static ON: Switch = Switch(std::sync::atomic::AtomicU8::new(0));
    switch(&ON.0, || dense_levers()[5])
}

/// Spike (ryo-jenl): census sites: 0 index build, 1 index probe, 2 repeat check, 3 row-set build, 4 row-set ask, 5 text index,
/// 6 index rows keyed by words, 7 index probes by words. The per-probe and per-row sites (1, 2, 4, 7) were counted on
/// commit 01ba804f4 and e6014fa6d and are no longer counted: a check per probe cost more than it told.
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
/// span few places addressed by position; `walks`, the
/// indexes of walks, closures, components and groups keyed the same way; `visited`, a walk's met nodes by position.
pub fn dense_lever(name: &str) -> bool {
    let at = match name {
        "words" => 0,
        "repeats" => 1,
        "cellword" => 2,
        "slots" => 3,
        "walks" => 4,
        "visited" => 5,
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
        [on("words"), on("repeats"), on("cellword"), on("slots"), on("walks"), on("visited")]
    })
}

/// Spike (ryo-fmx3.1): which engine runs the file phase, from `RYO_ENGINE`, for the harness only (scope-engine2 2.9): the old
/// engine (unset or `old`), the new engine alone (`new`), or both, comparing the rules the new engine's `load` accepts (`both`).
///
/// # Examples
///
/// ```
/// use ryo_operators::spike::EngineMode;
///
/// assert_ne!(EngineMode::Old, EngineMode::Both);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineMode {
    /// Today's engine alone.
    Old,
    /// The new engine alone, on the rules its `load` accepts.
    New,
    /// Both engines; the run fails on any difference of the rules the new engine accepts.
    Both,
}

/// Spike (ryo-fmx3.1): the engine `RYO_ENGINE` selects, read once; an unknown value is the old engine.
///
/// # Examples
///
/// ```
/// let _mode = ryo_operators::spike::engine_mode();
/// ```
pub fn engine_mode() -> EngineMode {
    static MODE: OnceLock<EngineMode> = OnceLock::new();
    *MODE.get_or_init(|| match std::env::var("RYO_ENGINE").as_deref() {
        Ok("new") => EngineMode::New,
        Ok("both") => EngineMode::Both,
        _ => EngineMode::Old,
    })
}

/// Spike (ryo-fmx3.1): the harness's mode as `RYO_ENGINE` spells it.
fn engine_name(mode: EngineMode) -> &'static str {
    match mode {
        EngineMode::Old => "old",
        EngineMode::New => "new",
        EngineMode::Both => "both",
    }
}

/// Spike (ryo-fmx3.1): writes to stderr what the new engine's `load` accepted of the selected rules, the refused ones by why.
///
/// # Examples
///
/// ```
/// use ryo_operators::spike::{EngineMode, engine_loaded};
///
/// engine_loaded(EngineMode::Both, 0, 3, 3, 0);
/// ```
pub fn engine_loaded(mode: EngineMode, accepted: usize, selected: usize, unlowerable: usize, over_budget: usize) {
    let refused = selected.saturating_sub(accepted);
    eprintln!(
        "ryo engine {}: load accepted {accepted} of {selected} rules; refused {refused} ({unlowerable} unlowerable, {over_budget} over budget)",
        engine_name(mode)
    );
}

/// Spike (ryo-fmx3.1): writes to stderr what the store's pack of the selected rules reads back: nothing was lowered.
///
/// # Examples
///
/// ```
/// use ryo_operators::spike::{EngineMode, engine_unpacked};
///
/// engine_unpacked(EngineMode::Both, 0, 3);
/// ```
pub fn engine_unpacked(mode: EngineMode, accepted: usize, selected: usize) {
    let refused = selected.saturating_sub(accepted);
    eprintln!(
        "ryo engine {}: pack of the store accepted {accepted} of {selected} rules; refused {refused} (nothing lowered)",
        engine_name(mode)
    );
}

/// Spike (ryo-fmx3.1): writes to stderr how many accepted rules `both` compared, all equal.
///
/// # Examples
///
/// ```
/// ryo_operators::spike::engine_compared(0);
/// ```
pub fn engine_compared(rules: usize) {
    eprintln!("ryo engine both: compared {rules} rules, 0 differences");
}

/// Spike (ryo-fmx3.1): why the harness stops a run whose files a language linked without its walk would have to be scanned of.
///
/// # Examples
///
/// ```
/// use ryo_operators::spike::{EngineMode, engine_unwalked};
///
/// assert!(engine_unwalked(EngineMode::New, &"words").starts_with("RYO_ENGINE=new"));
/// ```
pub fn engine_unwalked(mode: EngineMode, language: &dyn std::fmt::Display) -> String {
    format!(
        "RYO_ENGINE={}: the language `{language}` is linked without its walk, so the new engine scans none of its files",
        engine_name(mode)
    )
}

/// Spike (ryo-fmx3.1): why the harness stops a run whose engines' rows differ.
///
/// # Examples
///
/// ```
/// assert!(ryo_operators::spike::engine_difference(&3).ends_with('3'));
/// ```
pub fn engine_difference(difference: &dyn std::fmt::Debug) -> String {
    format!("rows differ: {difference:?}")
}
