//! The minimal per-file host the generated rules run on: one file's fact columns read in place (one `u64` per cell, row order,
//! the engine's own bits), per-file indexes built on first use and shared by every rule, materialised node rows shared by
//! every rule that reads them, and literal texts resolved to the file's symbol codes once.

use ryo_language::domain::symbols::service::{find_symbol, text_of};
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::FactColumn;
use ryo_relations::domain::terms::types::Symbol;
use rustc_hash::{FxHashMap, FxHasher};
use std::cell::{Cell, OnceCell};
use std::hash::Hasher;

/// No symbol of the file holds the literal: a code no cell holds.
pub const ABSENT: u64 = u64::MAX;
const UNRESOLVED: u64 = u64::MAX - 1;

/// Rows by key: a permutation of the rows grouped by key, and each key's stretch of it.
#[derive(Default)]
pub struct KeyIndex {
    dense: Vec<u32>,
    hi: u64,
    map: FxHashMap<u64, (u32, u32)>,
    perm: Vec<u32>,
}

impl KeyIndex {
    /// Builds the index of `n` rows whose key `key(row)` returns. `node` says the keys are node ids of one file: indexed by
    /// position in a dense offsets array.
    pub fn build(n: usize, node: bool, mut key: impl FnMut(usize) -> u64) -> Self {
        let mut keys: Vec<(u64, u32)> = (0..n).map(|row| (key(row), row as u32)).collect();
        if node && n > 0 {
            let hi = keys[0].0 >> 32;
            if keys.iter().all(|(k, _)| k >> 32 == hi) {
                let max = keys.iter().map(|(k, _)| (k & 0xffff_ffff) as usize).max().unwrap_or(0);
                if max <= 8 * n + 4096 {
                    let mut counts = vec![0u32; max + 2];
                    for (k, _) in &keys {
                        counts[(k & 0xffff_ffff) as usize + 1] += 1;
                    }
                    for i in 1..counts.len() {
                        counts[i] += counts[i - 1];
                    }
                    let mut fill = counts.clone();
                    let mut perm = vec![0u32; n];
                    for (k, row) in &keys {
                        let at = &mut fill[(k & 0xffff_ffff) as usize];
                        perm[*at as usize] = *row;
                        *at += 1;
                    }
                    return Self { dense: counts, hi, map: FxHashMap::default(), perm };
                }
            }
        }
        keys.sort_unstable();
        let mut map = FxHashMap::default();
        map.reserve(n);
        let mut perm = Vec::with_capacity(n);
        let mut start = 0usize;
        while start < keys.len() {
            let k = keys[start].0;
            let mut end = start;
            while end < keys.len() && keys[end].0 == k {
                perm.push(keys[end].1);
                end += 1;
            }
            let _ = map.insert(k, (start as u32, (end - start) as u32));
            start = end;
        }
        Self { dense: Vec::new(), hi: 0, map, perm }
    }

    /// The rows under `key`.
    #[inline]
    pub fn get(&self, key: u64) -> &[u32] {
        if !self.dense.is_empty() {
            if key >> 32 != self.hi {
                return &[];
            }
            let at = (key & 0xffff_ffff) as usize;
            if at + 1 >= self.dense.len() {
                return &[];
            }
            return &self.perm[self.dense[at] as usize..self.dense[at + 1] as usize];
        }
        match self.map.get(&key) {
            Some(&(start, len)) => &self.perm[start as usize..(start + len) as usize],
            None => &[],
        }
    }
}

/// The hash of a multi-column key.
#[inline]
pub fn key_of(cells: &[u64]) -> u64 {
    let mut hasher = FxHasher::default();
    for cell in cells {
        hasher.write_u64(*cell);
    }
    hasher.finish()
}

/// One file as the generated rules read it.
pub struct Cx<'a> {
    /// Per relation slot of the pack: its columns, or none when the file has no rows of it.
    pub rels: Vec<&'a [FactColumn]>,
    /// Per relation slot: its row count.
    pub lens: Vec<usize>,
    symbols: &'a Symbols,
    source: &'a [u8],
    lits: Vec<Cell<u64>>,
    bidx: Vec<OnceCell<KeyIndex>>,
    mats: Vec<OnceCell<Vec<u64>>>,
    kidx: Vec<OnceCell<KeyIndex>>,
    bridged: Vec<Vec<u64>>,
    /// Base rows visited that passed their view's tests: the candidates.
    pub visits: Cell<u64>,
}

impl<'a> Cx<'a> {
    pub fn new(
        rels: Vec<&'a [FactColumn]>,
        bridged: Vec<Vec<u64>>,
        symbols: &'a Symbols,
        source: &'a [u8],
        sizes: [usize; 4],
    ) -> Self {
        let lens = rels.iter().map(|columns| columns.first().map_or(0, |c| c.0.len())).collect();
        Self {
            rels,
            lens,
            symbols,
            source,
            lits: (0..sizes[0]).map(|_| Cell::new(UNRESOLVED)).collect(),
            bidx: (0..sizes[1]).map(|_| OnceCell::new()).collect(),
            mats: (0..sizes[2]).map(|_| OnceCell::new()).collect(),
            kidx: (0..sizes[3]).map(|_| OnceCell::new()).collect(),
            visits: Cell::new(0),
            bridged,
        }
    }

    /// The rows of a bridged node, as the table plan answered them.
    #[inline]
    pub fn bridged(&self, slot: usize) -> &[u64] {
        &self.bridged[slot]
    }

    /// The cell of a relation's row.
    #[inline(always)]
    pub fn c(&self, rel: usize, col: usize, row: u32) -> u64 {
        self.rels[rel][col].0[row as usize]
    }

    /// The file's code of the literal text `k` of the pack, or `ABSENT`.
    #[inline]
    pub fn lit(&self, k: usize, text: &str) -> u64 {
        let held = self.lits[k].get();
        if held != UNRESOLVED {
            return held;
        }
        let code = find_symbol(self.symbols, ByteSlice(self.source), ByteSlice(text.as_bytes()))
            .map_or(ABSENT, |symbol| u64::from(symbol.0));
        self.lits[k].set(code);
        code
    }

    /// The text of a symbol cell.
    #[inline]
    pub fn text(&self, cell: u64) -> &str {
        u32::try_from(cell)
            .ok()
            .and_then(|code| text_of(self.symbols, ByteSlice(self.source), Symbol(code)))
            .map_or("", |text| text.0)
    }

    /// The shared index of a relation's column.
    #[inline]
    pub fn bidx(&self, slot: usize, rel: usize, col: usize, node: bool) -> &KeyIndex {
        self.bidx[slot].get_or_init(|| {
            let column = self.rels[rel].get(col).map_or(&[][..], |c| &c.0[..]);
            KeyIndex::build(self.lens[rel], node, |row| column[row])
        })
    }

    /// The shared rows of a node, made on first use.
    #[inline]
    pub fn mat(&self, slot: usize, make: impl FnOnce(&mut Vec<u64>)) -> &[u64] {
        self.mats[slot].get_or_init(|| {
            let mut rows = Vec::new();
            make(&mut rows);
            rows
        })
    }

    /// The shared index of a node's rows on some of its columns.
    #[inline]
    pub fn kidx(&self, slot: usize, rows: &[u64], arity: usize, keys: &[usize], node: bool) -> &KeyIndex {
        self.kidx[slot].get_or_init(|| {
            let n = if arity == 0 { 0 } else { rows.len() / arity };
            if keys.len() == 1 {
                KeyIndex::build(n, node, |row| rows[row * arity + keys[0]])
            } else {
                let mut cells = Vec::with_capacity(keys.len());
                KeyIndex::build(n, false, |row| {
                    cells.clear();
                    cells.extend(keys.iter().map(|k| rows[row * arity + k]));
                    key_of(&cells)
                })
            }
        })
    }
}

use ryo_language::domain::elements::service::{end_at, start_at};
use ryo_language::domain::elements::types::Elements;
use ryo_relations::domain::relations::service::{from_bits, location};
use ryo_relations::domain::relations::types::{Bits, CellKind, Locatedness, Position, Value};
use std::io::Write as _;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

/// What a codegen run does per file, from `RYO_CODEGEN`: nothing (the engine runs), or the engine's rules skipped and
/// `parse` (no facts demanded), `pack` (the facts every selected rule demands), `facts` (the facts the generated rules
/// read), `rules` (those facts and the generated rules, findings written to `RYO_CODEGEN_OUT`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Off,
    Parse,
    Pack,
    Facts,
    Rules,
}

pub fn mode() -> Mode {
    static MODE: OnceLock<Mode> = OnceLock::new();
    *MODE.get_or_init(|| match std::env::var("RYO_CODEGEN").as_deref() {
        Ok("parse") => Mode::Parse,
        Ok("pack") => Mode::Pack,
        Ok("facts") => Mode::Facts,
        Ok("rules") => Mode::Rules,
        _ => Mode::Off,
    })
}

const SLOTS: usize = 1024;
struct Stats {
    nanos: [AtomicU64; SLOTS],
    visits: [AtomicU64; SLOTS],
    rows: [AtomicU64; SLOTS],
    files_with: [AtomicU64; SLOTS],
    cx_nanos: AtomicU64,
    bridge_nanos: AtomicU64,
    bridge_failures: AtomicU64,
    files: AtomicUsize,
}
static STATS: Stats = Stats {
    nanos: [const { AtomicU64::new(0) }; SLOTS],
    visits: [const { AtomicU64::new(0) }; SLOTS],
    rows: [const { AtomicU64::new(0) }; SLOTS],
    files_with: [const { AtomicU64::new(0) }; SLOTS],
    cx_nanos: AtomicU64::new(0),
    bridge_nanos: AtomicU64::new(0),
    bridge_failures: AtomicU64::new(0),
    files: AtomicUsize::new(0),
};

/// Counts a bridged node the table plan refused in a file.
pub fn bridge_failed() {
    let _ = STATS.bridge_failures.fetch_add(1, Ordering::Relaxed);
}

/// Adds the time the native tree's columns took in a file.
pub fn fill_time(spent: std::time::Duration) {
    let _ = STATS.cx_nanos.fetch_add(spent.as_nanos() as u64, Ordering::Relaxed);
}

/// Adds the time the bridge took in a file.
pub fn bridge_time(spent: std::time::Duration) {
    let _ = STATS.bridge_nanos.fetch_add(spent.as_nanos() as u64, Ordering::Relaxed);
}
static OUT: Mutex<Option<std::fs::File>> = Mutex::new(None);

fn timing() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("RYO_CODEGEN_STATS").is_some())
}

/// Where a finding row is: the first key column, then the subject, that is a located stretch or a node.
fn place(row: &[u64], kinds: &[u8], key: &[usize], subject: usize, tree: &Elements) -> Option<(u32, u32)> {
    key.iter().chain(std::iter::once(&subject)).find_map(|&c| {
        let bits = *row.get(c)?;
        match kinds.get(c)? {
            3 => match from_bits(CellKind::Span, Bits(bits)) {
                Value::Span(span) if location(span) == Locatedness::Located => Some((span.start.0, span.end.0)),
                _ => None,
            },
            0 => {
                let at = Position((bits & 0xffff_ffff) as u32);
                Some((start_at(tree, at).0, end_at(tree, at).0))
            }
            _ => None,
        }
    })
}

/// Runs every generated rule over one file and writes its findings. `emit(rule)` answers the rule's key columns, subject
/// column and id.
pub fn run_file(
    rels: Vec<&[FactColumn]>,
    bridged: Vec<Vec<u64>>,
    symbols: &Symbols,
    source: &[u8],
    tree: &Elements,
    path: &str,
    emit: &dyn Fn(usize) -> Option<(Vec<usize>, usize, String)>,
) {
    let stats = timing();
    let started = Instant::now();
    let cx = Cx::new(rels, bridged, symbols, source, super::generated::SIZES);
    if stats {
        let _ = STATS.cx_nanos.fetch_add(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
    }
    let mut text = String::new();
    let mut rows = Vec::new();
    for &(rule, arity, kinds, run) in super::generated::RULES {
        rows.clear();
        let begun = if stats { Some((Instant::now(), cx.visits.get())) } else { None };
        run(&cx, &mut rows);
        if let Some((begun, visits)) = begun {
            let r = rule as usize % SLOTS;
            let _ = STATS.nanos[r].fetch_add(begun.elapsed().as_nanos() as u64, Ordering::Relaxed);
            let _ = STATS.visits[r].fetch_add(cx.visits.get() - visits, Ordering::Relaxed);
            let _ = STATS.rows[r].fetch_add((rows.len() / arity.max(1)) as u64, Ordering::Relaxed);
            if !rows.is_empty() {
                let _ = STATS.files_with[r].fetch_add(1, Ordering::Relaxed);
            }
        }
        if rows.is_empty() {
            continue;
        }
        let Some((key, subject, id)) = emit(rule as usize) else { continue };
        for row in rows.chunks_exact(arity) {
            if let Some((start, end)) = place(row, kinds, &key, subject, tree) {
                use std::fmt::Write as _;
                let _ = writeln!(text, "{{\"rule\":\"{id}\",\"file\":{path:?},\"start\":{start},\"end\":{end}}}");
            }
        }
    }
    let mut out = OUT.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if out.is_none() {
        *out = std::env::var_os("RYO_CODEGEN_OUT").and_then(|p| std::fs::File::create(p).ok());
    }
    if let Some(file) = out.as_mut() {
        let _ = file.write_all(text.as_bytes());
    }
    let done = STATS.files.fetch_add(1, Ordering::Relaxed) + 1;
    let expected = std::env::var("RYO_CODEGEN_NFILES").ok().and_then(|n| n.parse::<usize>().ok());
    if stats && expected == Some(done) {
        let mut dump = format!(
            "{{\"cx_nanos\":{},\"bridge_nanos\":{},\"bridge_failures\":{},\"files\":{done}}}\n",
            STATS.cx_nanos.load(Ordering::Relaxed),
            STATS.bridge_nanos.load(Ordering::Relaxed),
            STATS.bridge_failures.load(Ordering::Relaxed)
        );
        for &(rule, _, _, _) in super::generated::RULES {
            let r = rule as usize % SLOTS;
            let id = emit(rule as usize).map(|e| e.2).unwrap_or_default();
            use std::fmt::Write as _;
            let _ = writeln!(
                dump,
                "{{\"rule\":\"{id}\",\"ordinal\":{rule},\"nanos\":{},\"visits\":{},\"rows\":{},\"files_with\":{}}}",
                STATS.nanos[r].load(Ordering::Relaxed),
                STATS.visits[r].load(Ordering::Relaxed),
                STATS.rows[r].load(Ordering::Relaxed),
                STATS.files_with[r].load(Ordering::Relaxed)
            );
        }
        if let Some(path) = std::env::var_os("RYO_CODEGEN_STATS") {
            let _ = std::fs::write(path, dump);
        }
    }
}

/// Whether `text` matches the regex `pattern`, compiled once per process into `slot`, with the engine's matching semantics.
#[inline]
pub fn matches(
    slot: &OnceLock<ryo_matching::inbound::use_cases::matching::types::CompiledRegex>,
    pattern: &str,
    text: &str,
) -> bool {
    use ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase as _;
    use ryo_matching::inbound::use_cases::matching::types::{MatchOutcome, RegexSource, SearchText};
    let matcher = ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl::new();
    let Some(regex) = slot.get().or_else(|| {
        let compiled = matcher.compile_regex(RegexSource(pattern)).ok()?;
        let _ = slot.set(compiled);
        slot.get()
    }) else {
        return false;
    };
    matches!(matcher.matches_regex(regex, SearchText(text)), MatchOutcome::Matches)
}

/// Whether a span cell is a located stretch whose start is not past its end: the rows span-bounds answers.
#[inline]
pub fn span_ok(bits: u64) -> bool {
    match from_bits(CellKind::Span, Bits(bits)) {
        Value::Span(span) => span != ryo_relations::domain::relations::constants::NO_SPAN && span.start <= span.end,
        _ => false,
    }
}
