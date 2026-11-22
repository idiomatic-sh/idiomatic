//! 10: the per-key memo of a probed step (keys in, rows out, once a file), and its counter (`VEC_MEMO_COUNT=<path>`).

use rustc_hash::FxHashMap;

/// 10 cycle 1: what the memo is asked, per (step operator, key type), written to `$VEC_MEMO_COUNT` when the run ends.
pub(super) struct Count {
    pub(super) on: bool,
    by: FxHashMap<(String, &'static str), C>,
    /// keys seen this file per slot (to count asks of a key already asked of the same slot)
    seen: FxHashMap<(usize, usize, u64), u32>,
}

#[derive(Default, Clone, Copy)]
struct C {
    asks: u64,
    keys: u64,
    list_hits: u64,
    list_keys: u64,
    new_keys: u64,
    again_keys: u64,
    dup_in_batch: u64,
    small_keys: u64,
    big_keys: u64,
    empty_keys: u64,
    rows: u64,
    runs: u64,
    run_keys: u64,
    unasked: u64,
    sorted: u64,
    hist: [u64; 7],
}

impl Count {
    pub(super) fn new() -> Self {
        Count { on: std::env::var_os("VEC_MEMO_COUNT").is_some(), by: FxHashMap::default(), seen: FxHashMap::default() }
    }

    pub(super) fn file(&mut self) {
        self.seen.clear();
    }

    /// One ask of slot (node, col): its keys, whether answered from the list cache, the keys missing (run), rows out,
    /// and per key whether it has rows (`has`).
    #[expect(clippy::too_many_arguments, reason = "a counter")]
    pub(super) fn ask(&mut self, op: &str, ty: &'static str, node: usize, col: usize, nodes: usize, keys: &[u64], list_hit: bool, missing: usize, rows: usize, empty: u64, unasked: u64) {
        let c = self.by.entry((op.to_owned(), ty)).or_default();
        c.asks += 1;
        c.keys += keys.len() as u64;
        let b = match keys.len() {
            0..=1 => 0,
            2..=3 => 1,
            4..=15 => 2,
            16..=63 => 3,
            64..=255 => 4,
            256..=1023 => 5,
            _ => 6,
        };
        c.hist[b] += 1;
        if keys.windows(2).all(|w| w[0] <= w[1]) {
            c.sorted += 1;
        }
        if list_hit {
            c.list_hits += 1;
            c.list_keys += keys.len() as u64;
            return;
        }
        if missing > 0 {
            c.runs += 1;
            c.run_keys += missing as u64;
        }
        c.rows += rows as u64;
        c.empty_keys += empty;
        c.unasked += unasked;
        for k in keys {
            if (*k as usize) < nodes {
                c.small_keys += 1;
            } else {
                c.big_keys += 1;
            }
            let e = self.seen.entry((node, col, *k)).or_insert(0);
            if *e == 0 {
                c.new_keys += 1;
            } else if *e == u32::MAX {
                c.dup_in_batch += 1;
            } else {
                c.again_keys += 1;
            }
            *e = u32::MAX;
        }
        for k in keys {
            if let Some(e) = self.seen.get_mut(&(node, col, *k)) {
                *e = 1;
            }
        }
    }
}

impl Drop for Count {
    fn drop(&mut self) {
        let Some(p) = std::env::var_os("VEC_MEMO_COUNT") else { return };
        let mut v: Vec<_> = self.by.iter().collect();
        v.sort_by_key(|x| std::cmp::Reverse(x.1.keys));
        let mut s = String::from("op\tty\tasks\tkeys\tlist_hits\tlist_keys\tnew\tagain\tdup_in_batch\tsmall(<nodes)\tbig\tempty_keys\trows\truns\trun_keys\tunasked\tsorted\th1\th2-3\th4-15\th16-63\th64-255\th256-1023\th1024+\n");
        for ((op, ty), c) in v {
            s += &format!(
                "{op}\t{ty}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                c.asks, c.keys, c.list_hits, c.list_keys, c.new_keys, c.again_keys, c.dup_in_batch, c.small_keys, c.big_keys, c.empty_keys, c.rows, c.runs, c.run_keys, c.unasked, c.sorted,
                c.hist.iter().map(u64::to_string).collect::<Vec<_>>().join("\t")
            );
        }
        let _ = std::fs::write(p, s);
    }
}

// ------------------------------------------------------------------------------------------------ the memo (10)

use super::{Exec, File, Provider, Run, Schema, Table, R};
use std::rc::Rc;

/// Columns of a node whose slots are found by position.
const AT: usize = 4;

/// The chunk number of a range in the slot's own table (rows filed row by row).
const OWN: u32 = u32::MAX;

/// A key's rows this file (`file`: else stale): `len` rows from `start` of chunk `chunk` (a batch's answer kept whole)
/// or of the slot's own table.
#[derive(Clone, Copy)]
struct Rng {
    file: u32,
    chunk: u32,
    start: u32,
    len: u32,
    /// the gather that last handed this key out (a key is handed out once an answer)
    ask: u32,
}

/// One (step, column) slot. Nothing in it is cleared between files: every entry carries the file's number.
struct Slot {
    file: u32,
    nodes: usize,
    /// asked this file; the keys of its first ask when they came strictly increasing (so distinct, all new): a range of
    /// the memo's arena, not marked in `seen` until a second ask needs them
    asked: bool,
    first: Option<(u32, u32)>,
    /// the keys asked this file below `nodes` (the file's node ids): a word per 32 keys, the file's number in the high
    /// half and a bit a key in the low half (grown to the largest key asked)
    seen: Vec<u64>,
    /// the keys asked this file at or above `nodes` (the file's number a key)
    far: FxHashMap<u64, u32>,
    /// the ranges of the keys that have rows, indexed lazily (a chunk's keys are filed here only once a key asked again
    /// needs them); cleared at a file that used it
    rows: FxHashMap<u64, Rng>,
    /// keys given rows this file (none: no key is looked up in `rows`)
    with_rows: u32,
    /// the slot's column (a chunk's keys) and the chunks indexed into `rows`
    col: usize,
    indexed: usize,
    chunks: Vec<Rc<Table>>,
    own: Table,
    /// bumped by every filing (an answer handed back whole must see no filing while its batch ran)
    filed: u32,
}

impl Slot {
    /// Marks `k` asked this file; whether it was asked before.
    #[inline]
    fn mark(&mut self, k: u64, file: u32) -> bool {
        if (k as usize) < self.nodes {
            let w = (k >> 5) as usize;
            if w >= self.seen.len() {
                self.seen.resize(w + 1, 0);
            }
            let x = self.seen[w];
            let b = 1u64 << (k & 31);
            if (x >> 32) as u32 != file {
                self.seen[w] = (u64::from(file) << 32) | b;
                false
            } else if x & b != 0 {
                true
            } else {
                self.seen[w] = x | b;
                false
            }
        } else {
            self.far.insert(k, file) == Some(file)
        }
    }

    fn unmark(&mut self, k: u64, file: u32) {
        if (k as usize) < self.nodes {
            let w = (k >> 5) as usize;
            if let Some(x) = self.seen.get_mut(w)
                && (*x >> 32) as u32 == file
            {
                *x &= !(1u64 << (k & 31));
            }
        } else {
            self.far.remove(&k);
        }
    }

    /// Files the keys of the chunks not indexed yet (each chunk's rows are grouped by key).
    fn index(&mut self, file: u32) {
        while self.indexed < self.chunks.len() {
            let ch = self.indexed;
            let t = &self.chunks[ch];
            let kc = &t.cols[self.col][..t.n];
            let mut r = 0;
            while r < kc.len() {
                let k = kc[r];
                let s = r;
                r += 1;
                while r < kc.len() && kc[r] == k {
                    r += 1;
                }
                self.rows.insert(k, Rng { file, chunk: ch as u32, start: s as u32, len: (r - s) as u32, ask: 0 });
                self.with_rows += 1;
            }
            self.indexed += 1;
        }
    }

    #[inline]
    fn rng(&mut self, k: u64, file: u32) -> Option<Rng> {
        if self.indexed < self.chunks.len() {
            self.index(file);
        }
        if self.with_rows == 0 {
            return None;
        }
        self.rows.get(&k).filter(|r| r.file == file).copied()
    }

    fn col(&self, chunk: u32, c: usize) -> &[u64] {
        if chunk == OWN { &self.own.cols[c] } else { &self.chunks[chunk as usize].cols[c] }
    }
}

pub(super) struct Memo {
    slots: Vec<Slot>,
    at: Vec<u32>,
    at_far: FxHashMap<(u32, u32), u32>,
    file: u32,
    ask: u32,
    missing: Vec<u64>,
    /// the first asks' keys this file (cleared at the next: a counter)
    arena: Vec<u64>,
    ranges: Vec<(u32, u32, u32)>,
    order: Vec<(u64, u32)>,
    /// a view probe's rows (finish.rs `probe`)
    pub(super) vrows: Vec<u32>,
    prof: bool,
    /// asks, keys, keys new to the slot, batches run, answers handed back whole, gathered, empty, batches filed row by row
    pub(super) n: [u64; 14],
    /// seconds: before the run, after it (filing and answer)
    t: [f64; 4],
}

impl Memo {
    pub(super) fn new() -> Self {
        Memo {
            slots: Vec::new(),
            at: Vec::new(),
            at_far: FxHashMap::default(),
            file: 1,
            ask: 0,
            missing: Vec::new(),
            arena: Vec::new(),
            ranges: Vec::new(),
            order: Vec::new(),
            vrows: Vec::new(),
            prof: std::env::var_os("VEC_MEMO_PROF").is_some(),
            n: [0; 14],
            t: [0.0; 4],
        }
    }

    /// The section clock (`VEC_MEMO_PROF`), for either memo.
    pub(super) fn tick(&self) -> f64 {
        if self.prof { super::select_kernel::ticks() } else { 0.0 }
    }
    pub(super) fn tock(&mut self, i: usize, t0: f64) {
        if self.prof {
            self.t[i] += super::select_kernel::ticks() - t0;
        }
    }

    /// A new file: a counter (slots reset themselves when first asked in it).
    pub(super) fn next_file(&mut self) {
        self.file += 1;
        self.arena.clear();
    }

    /// Marks the keys of a slot's first ask, kept aside, as asked.
    fn settle(&mut self, slot: usize) {
        let sl = &mut self.slots[slot];
        if let Some((s, l)) = sl.first.take() {
            for k in &self.arena[s as usize..(s + l) as usize] {
                sl.mark(*k, self.file);
            }
        }
    }

    fn slot(&mut self, node: usize, col: usize, nodes: usize, sc: &Schema) -> usize {
        // the slot of (node, col): one read of a flat array for the first `AT` columns of a node (slot + 1; 0: none)
        let found = if col < AT {
            if self.at.len() <= node * AT + col {
                self.at.resize((node + 1) * AT, 0);
            }
            self.at[node * AT + col].checked_sub(1)
        } else {
            self.at_far.get(&(node as u32, col as u32)).copied()
        };
        let i = match found {
            Some(x) => x as usize,
            None => {
                let i = self.slots.len();
                self.slots.push(Slot { file: 0, nodes: 0, asked: false, first: None, seen: Vec::new(), far: FxHashMap::default(), rows: FxHashMap::default(), with_rows: 0, col, indexed: 0, chunks: Vec::new(), own: Table::with(sc.names.clone(), sc.tys.clone()), filed: 0 });
                if col < AT {
                    self.at[node * AT + col] = i as u32 + 1;
                } else {
                    self.at_far.insert((node as u32, col as u32), i as u32);
                }
                i
            }
        };
        let sl = &mut self.slots[i];
        if sl.file != self.file {
            self.n[12] += 1;
            self.n[13] += sl.chunks.len() as u64;
            sl.file = self.file;
            sl.asked = false;
            sl.first = None;
            sl.nodes = nodes;
            if sl.with_rows > 0 {
                sl.rows.clear();
            }
            sl.with_rows = 0;
            sl.indexed = 0;
            if !sl.chunks.is_empty() {
                // redo-alloc: the last file's chunks given back (recycle.rs)
                for t in sl.chunks.drain(..) {
                    super::recycle::recycle(t);
                }
            }
            if sl.own.n > 0 {
                sl.own.n = 0;
                sl.own.cols.iter_mut().for_each(Vec::clear);
            }
        }
        i
    }

    /// The rows of `keys` (each key once, in the keys' order; a key's rows in the order filed).
    fn gather(&mut self, slot: usize, keys: &[u64], sc: &Schema) -> Table {
        self.ask = self.ask.wrapping_add(1);
        let (ask, file) = (self.ask, self.file);
        let sl = &mut self.slots[slot];
        sl.index(file);
        self.ranges.clear();
        let mut total = 0usize;
        if sl.with_rows > 0 {
            for k in keys {
                if let Some(r) = sl.rows.get_mut(k)
                    && r.file == file
                    && r.ask != ask
                {
                    r.ask = ask;
                    self.ranges.push((r.chunk, r.start, r.len));
                    total += r.len as usize;
                }
            }
        }
        let mut out = Table::with(sc.names.clone(), sc.tys.clone());
        for c in 0..out.cols.len() {
            let mut v = std::mem::take(&mut out.cols[c]);
            v.reserve(total);
            for (ch, s, l) in &self.ranges {
                v.extend_from_slice(&sl.col(*ch, c)[*s as usize..(*s + *l) as usize]);
            }
            out.cols[c] = v;
        }
        out.n = total;
        out
    }

    /// The old memo's filing, row by row into the slot's own table: the batch's rows sorted by (key, row); a key that
    /// already has rows has them moved to the end first, so each key's rows stay one range.
    fn file_rows(&mut self, slot: usize, out: &Table, col: usize) {
        let file = self.file;
        self.settle(slot);
        let sl = &mut self.slots[slot];
        sl.index(file);
        sl.filed = sl.filed.wrapping_add(1);
        let order = &mut self.order;
        order.clear();
        order.extend((0..out.n as u32).map(|r| (out.cols[col][r as usize], r)));
        order.sort_unstable();
        let mut i = 0;
        while i < order.len() {
            let k = order[i].0;
            let mut j = i;
            while j < order.len() && order[j].0 == k {
                j += 1;
            }
            sl.mark(k, file);
            let start = sl.own.n as u32;
            let mut len = 0u32;
            match sl.rng(k, file) {
                Some(e) => {
                    for c in 0..sl.own.cols.len() {
                        let (a, b) = (e.start as usize, (e.start + e.len) as usize);
                        if e.chunk == OWN {
                            sl.own.cols[c].extend_from_within(a..b);
                        } else {
                            sl.own.cols[c].extend_from_slice(&sl.chunks[e.chunk as usize].cols[c][a..b]);
                        }
                    }
                    sl.own.n += e.len as usize;
                    len = e.len;
                }
                None => sl.with_rows += 1,
            }
            for c in 0..sl.own.cols.len() {
                let src = &out.cols[c];
                sl.own.cols[c].extend(order[i..j].iter().map(|x| src[x.1 as usize]));
            }
            sl.own.n += j - i;
            sl.rows.insert(k, Rng { file, chunk: OWN, start, len: len + (j - i) as u32, ask: 0 });
            i = j;
        }
    }
}

impl Drop for Memo {
    fn drop(&mut self) {
        if self.prof || std::env::var_os("VEC_MEMO_COUNT").is_some() {
            let n = &self.n;
            eprintln!(
                "memo10: asks {} keys {} new {} runs {} whole {} gathered {} empty {} filed row by row {}; made tables probed {} ({} keys), views probed {} ({} keys); slot resets {} (chunks dropped {}); slots {}; seen words {}; row entries {}; s before {:.3} after {:.3} (key loop {:.3}, slot {:.3})",
                n[0], n[1], n[2], n[3], n[4], n[5], n[6], n[7], n[8], n[9], n[10], n[11], n[12], n[13], self.slots.len(), self.slots.iter().map(|s| s.seen.len()).sum::<usize>(), self.slots.iter().map(|s| s.rows.len()).sum::<usize>(), self.t[0], self.t[1], self.t[2], self.t[3]
            );
        }
    }
}

impl Exec<'_> {
    /// 10: the per-key memo's ask: keys already asked of this slot this file are answered from it, the rest run once
    /// (as one batch) and are filed. Rows and their order are the old memo's (`VEC_OLD_MEMO=1`).
    #[expect(clippy::too_many_arguments, reason = "a throwaway")]
    pub(super) fn probe_memo(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, col: usize, keys: &[u64], sc: &Schema) -> R<Rc<Table>> {
        let t0 = self.memo.tick();
        let m = &mut self.memo;
        // the key loop's and the slot's own clocks only under VEC_MEMO_PROF=2 (they add two clock reads an ask)
        let prof = m.prof && {
            static P2: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
            *P2.get_or_init(|| std::env::var_os("VEC_MEMO_PROF").is_some_and(|v| v == "2"))
        };
        let slot = m.slot(node, col, f.n, sc);
        if prof {
            m.t[3] += super::select_kernel::ticks() - t0;
        }
        let file = m.file;
        m.n[0] += 1;
        m.n[1] += keys.len() as u64;
        self.stats.memo_keys += keys.len() as u64;
        let mut missing = std::mem::take(&mut m.missing);
        missing.clear();
        // a slot's first ask this file with keys strictly increasing: every key is new, nothing to look up or mark yet
        let first = !m.slots[slot].asked && keys.windows(2).all(|w| w[0] < w[1]);
        m.slots[slot].asked = true;
        if first {
            let s = m.arena.len() as u32;
            m.arena.extend_from_slice(keys);
            m.slots[slot].first = Some((s, keys.len() as u32));
        } else {
            m.settle(slot);
        }
        let sl = &mut m.slots[slot];
        let filed0 = sl.filed;
        // while every key so far is new, the missing keys are a prefix of `keys` (no copy)
        let mut all_new = 0usize;
        let mut hit_rows = false;
        // a hit answer that is one chunk whole, in order
        let mut whole: Option<(u32, u32)> = None;
        let mut whole_ok = true;
        let tk = if prof { super::select_kernel::ticks() } else { 0.0 };
        if first {
            all_new = keys.len();
        }
        for (at, &k) in keys.iter().enumerate().skip(if first { keys.len() } else { 0 }) {
            if sl.mark(k, file) {
                if all_new == at {
                    missing.extend_from_slice(&keys[..at]);
                    all_new = usize::MAX;
                }
                if let Some(e) = sl.rng(k, file) {
                    hit_rows = true;
                    match whole {
                        None if e.start == 0 && e.chunk != OWN => whole = Some((e.chunk, e.len)),
                        Some((ch, next)) if ch == e.chunk && next == e.start => whole = Some((ch, next + e.len)),
                        _ => whole_ok = false,
                    }
                }
            } else if all_new == at {
                all_new += 1;
            } else {
                missing.push(k);
            }
        }
        if prof {
            m.t[2] += super::select_kernel::ticks() - tk;
        }
        let fresh = if all_new == keys.len() { keys.len() } else { missing.len() };
        m.n[2] += fresh as u64;
        if fresh == 0 {
            m.missing = missing;
            let out = if !hit_rows {
                m.n[6] += 1;
                self.empty_rc(run, node)?
            } else if let (true, Some((ch, len))) = (whole_ok, whole)
                && m.slots[slot].chunks[ch as usize].n == len as usize
            {
                m.n[4] += 1;
                m.slots[slot].chunks[ch as usize].clone()
            } else {
                m.n[5] += 1;
                super::recycle::rc(m.gather(slot, keys, sc))
            };
            self.stats.memo_out += out.n as u64;
            self.memo.tock(0, t0);
            return Ok(out);
        }
        m.n[3] += 1;
        self.memo.tock(0, t0);
        let run_keys: &[u64] = if all_new == keys.len() { keys } else { &missing };
        let out = self.probe_run(f, run, p, node, col, run_keys, sc);
        let t1 = self.memo.tick();
        let m = &mut self.memo;
        let out = match out {
            Ok(o) => o,
            Err(e) => {
                m.settle(slot);
                let sl = &mut m.slots[slot];
                for k in run_keys {
                    sl.unmark(*k, file);
                }
                m.missing = missing;
                return Err(e);
            }
        };
        self.stats.memo_in += out.n as u64;
        let mut answer: Option<Rc<Table>> = None;
        if out.n > 0 {
            // no filing while the batch ran, and its rows grouped by key in the order of its keys: kept whole as a chunk
            let sl = &m.slots[slot];
            let mut ok = sl.filed == filed0;
            if ok {
                let kc = &out.cols[col][..out.n];
                let (mut j, mut r) = (0usize, 0usize);
                while r < kc.len() {
                    let k = kc[r];
                    while j < run_keys.len() && run_keys[j] != k {
                        j += 1;
                    }
                    if j == run_keys.len() {
                        ok = false;
                        break;
                    }
                    j += 1;
                    r += 1;
                    while r < kc.len() && kc[r] == k {
                        r += 1;
                    }
                }
            }
            if ok {
                let sl = &mut m.slots[slot];
                sl.filed = sl.filed.wrapping_add(1);
                sl.chunks.push(out.clone());
                if !hit_rows {
                    m.n[4] += 1;
                    answer = Some(out);
                }
            } else {
                m.n[7] += 1;
                m.file_rows(slot, &out, col);
            }
        } else if !hit_rows && m.slots[slot].filed == filed0 {
            m.n[6] += 1;
            answer = Some(self.empty_rc(run, node)?);
        }
        let m = &mut self.memo;
        let answer = match answer {
            Some(a) => a,
            None => {
                m.n[5] += 1;
                super::recycle::rc(m.gather(slot, keys, sc))
            }
        };
        m.missing = missing;
        self.stats.memo_out += answer.n as u64;
        self.memo.tock(1, t1);
        Ok(answer)
    }
}
