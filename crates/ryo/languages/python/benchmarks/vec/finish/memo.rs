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

/// An entry's rows are pending (its batch is running, or it ran and gave none).
const PENDING: u32 = u32::MAX;
/// An entry taken back (its batch failed): absent to every lookup.
const DEAD: u32 = u32::MAX - 1;
/// An entry's rows are in the slot's own table (filed row by row: rows for a key not asked, or a key's rows grown).
const OWN: u32 = u32::MAX - 2;
/// A slot holding this many keys in one file indexes them densely.
/// (`VEC_MEMO_PROMOTE=<n>`; 0: never)
fn promote_at() -> usize {
    static P: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *P.get_or_init(|| match std::env::var("VEC_MEMO_PROMOTE").ok().and_then(|x| x.parse().ok()) {
        Some(0) => usize::MAX,
        Some(n) => n,
        None => 64,
    })
}

/// A key's state in a slot this file: its rows are `len` rows from `start` of chunk `chunk` (a batch's answer kept whole)
/// or of the slot's own table.
#[derive(Clone, Copy)]
struct Ent {
    key: u64,
    chunk: u32,
    start: u32,
    len: u32,
    /// the gather that last handed this key out (a key is handed out once an answer)
    ask: u32,
    /// the batch that asked for it, and its place in that batch's missing keys
    batch: u32,
    pos: u32,
}

/// One (step, column) slot. Keys below `dense.len()` (the file's node ids) find their entry through `dense` (an entry
/// index, valid when it is below `ents.len()` and that entry holds the key: a sparse set, so nothing is cleared between
/// files); other keys through `far` (one hash probe a key). The slot is reset lazily when first asked in a new file.
struct Slot {
    file: u32,
    /// `dense` is in use this file (allocated for a slot once it held `PROMOTE` keys in a file, sized to the file)
    dense_on: bool,
    nodes: usize,
    dense: Vec<u32>,
    far: FxHashMap<u64, u32>,
    ents: Vec<Ent>,
    chunks: Vec<Rc<Table>>,
    own: Table,
    /// bumped by every row-by-row filing (a batch's answer handed back whole must see no such change while it ran)
    filed: u32,
}

impl Slot {
    #[inline]
    fn find(&self, k: u64) -> Option<usize> {
        let i = if self.dense_on && (k as usize) < self.dense.len() {
            self.dense[k as usize] as usize
        } else {
            *self.far.get(&k)? as usize
        };
        match self.ents.get(i) {
            Some(e) if e.key == k && e.chunk != DEAD => Some(i),
            _ => None,
        }
    }

    #[inline]
    fn add(&mut self, e: Ent) -> usize {
        let i = self.ents.len();
        self.ents.push(e);
        if self.dense_on && (e.key as usize) < self.dense.len() {
            self.dense[e.key as usize] = i as u32;
        } else {
            self.far.insert(e.key, i as u32);
            if !self.dense_on && i + 1 >= promote_at() && self.nodes > 0 {
                self.promote();
            }
        }
        i
    }

    /// The slot holds many keys this file: index its node-id keys densely from now on (the hashed entries stay valid
    /// for keys above the array).
    #[cold]
    fn promote(&mut self) {
        if self.dense.len() < self.nodes {
            self.dense.resize(self.nodes, u32::MAX);
        }
        self.dense_on = true;
        for (i, e) in self.ents.iter().enumerate() {
            if (e.key as usize) < self.dense.len() {
                self.dense[e.key as usize] = i as u32;
            }
        }
    }

    fn col(&self, chunk: u32, c: usize) -> &[u64] {
        if chunk == OWN { &self.own.cols[c] } else { &self.chunks[chunk as usize].cols[c] }
    }
}

pub(super) struct Memo {
    slots: Vec<Slot>,
    at: Vec<Vec<(u32, u32)>>,
    file: u32,
    batch: u32,
    ask: u32,
    missing: Vec<u64>,
    ranges: Vec<(u32, u32, u32)>,
    counts: Vec<u32>,
    perm: Vec<u32>,
    order: Vec<(u64, u32)>,
    prof: bool,
    /// asks, keys, keys new to the slot, batches run, answers handed back whole (a batch's own table), gathered, empty
    pub(super) n: [u64; 7],
    /// seconds: before the run, after it (filing and answer)
    t: [f64; 2],
}

impl Memo {
    pub(super) fn new() -> Self {
        Memo {
            slots: Vec::new(),
            at: Vec::new(),
            file: 1,
            batch: 0,
            ask: 0,
            missing: Vec::new(),
            ranges: Vec::new(),
            counts: Vec::new(),
            perm: Vec::new(),
            order: Vec::new(),
            prof: std::env::var_os("VEC_MEMO_PROF").is_some(),
            n: [0; 7],
            t: [0.0; 2],
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
    }

    fn slot(&mut self, node: usize, col: usize, nodes: usize, sc: &Schema) -> usize {
        if self.at.len() <= node {
            self.at.resize_with(node + 1, Vec::new);
        }
        let i = match self.at[node].iter().find(|x| x.0 as usize == col) {
            Some(x) => x.1 as usize,
            None => {
                let i = self.slots.len();
                self.slots.push(Slot { file: 0, dense_on: false, nodes: 0, dense: Vec::new(), far: FxHashMap::default(), ents: Vec::new(), chunks: Vec::new(), own: Table::with(sc.names.clone(), sc.tys.clone()), filed: 0 });
                self.at[node].push((col as u32, i as u32));
                i
            }
        };
        let sl = &mut self.slots[i];
        if sl.file != self.file {
            sl.file = self.file;
            sl.ents.clear();
            if !sl.far.is_empty() {
                sl.far.clear();
            }
            sl.chunks.clear();
            sl.own.n = 0;
            sl.own.cols.iter_mut().for_each(Vec::clear);
            sl.nodes = nodes;
            sl.dense_on = !sl.dense.is_empty() && sl.dense.len() >= nodes;
        }
        i
    }

    /// The rows of `keys` (each key once, in the keys' order; a key's rows in the order filed).
    fn gather(&mut self, slot: usize, keys: &[u64], sc: &Schema) -> Table {
        self.ask = self.ask.wrapping_add(1);
        let ask = self.ask;
        let sl = &mut self.slots[slot];
        self.ranges.clear();
        let mut total = 0usize;
        for k in keys {
            if let Some(i) = sl.find(*k) {
                let e = &mut sl.ents[i];
                if e.ask != ask {
                    e.ask = ask;
                    if e.len > 0 {
                        self.ranges.push((e.chunk, e.start, e.len));
                        total += e.len as usize;
                    }
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
        let sl = &mut self.slots[slot];
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
            let start = sl.own.n as u32;
            let ix = match sl.find(k) {
                Some(x) => x,
                None => sl.add(Ent { key: k, chunk: OWN, start, len: 0, ask: 0, batch: 0, pos: 0 }),
            };
            let e = sl.ents[ix];
            let mut len = 0u32;
            if e.len > 0 {
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
            for c in 0..sl.own.cols.len() {
                let src = &out.cols[c];
                sl.own.cols[c].extend(order[i..j].iter().map(|x| src[x.1 as usize]));
            }
            sl.own.n += j - i;
            let e = &mut sl.ents[ix];
            e.chunk = OWN;
            e.start = start;
            e.len = len + (j - i) as u32;
            i = j;
        }
    }
}

impl Drop for Memo {
    fn drop(&mut self) {
        if self.prof || std::env::var_os("VEC_MEMO_COUNT").is_some() {
            let n = &self.n;
            eprintln!(
                "memo10: asks {} keys {} new {} runs {} whole {} gathered {} empty {}; slots {}; dense cells {}; s before {:.3} after {:.3}",
                n[0], n[1], n[2], n[3], n[4], n[5], n[6], self.slots.len(), self.slots.iter().map(|s| s.dense.len()).sum::<usize>(), self.t[0], self.t[1]
            );
        }
    }
}

impl Exec<'_> {
    /// 10: the per-key memo's ask: keys already asked of this slot this file are answered from it, the rest run once
    /// (as one batch) and are filed. Rows and their order are the old memo's (`VEC_OLD_MEMO=1`).
    #[expect(clippy::too_many_arguments, reason = "a throwaway")]
    pub(super) fn probe_memo(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, col: usize, keys: &[u64], sc: &Schema) -> R<Rc<Table>> {
        let t0 = if self.memo.prof { super::select_kernel::ticks() } else { 0.0 };
        let m = &mut self.memo;
        let slot = m.slot(node, col, f.n, sc);
        m.n[0] += 1;
        m.n[1] += keys.len() as u64;
        self.stats.memo_keys += keys.len() as u64;
        m.batch = m.batch.wrapping_add(1);
        let batch = m.batch;
        let mut missing = std::mem::take(&mut m.missing);
        missing.clear();
        let sl = &mut m.slots[slot];
        let filed0 = sl.filed;
        // while every key so far is new, the missing keys are a prefix of `keys` (no copy)
        let mut all_new = 0usize;
        let mut hit_rows = false;
        // a hit answer that is one chunk whole, in order
        let mut whole: Option<(u32, u32)> = None;
        let mut whole_ok = true;
        for (at, &k) in keys.iter().enumerate() {
            if let Some(i) = sl.find(k) {
                let e = sl.ents[i];
                if all_new == at {
                    missing.extend_from_slice(&keys[..at]);
                    all_new = usize::MAX;
                }
                if e.len > 0 {
                    hit_rows = true;
                    match whole {
                        None if e.start == 0 && e.chunk != OWN && e.chunk != PENDING => whole = Some((e.chunk, e.len)),
                        Some((ch, next)) if ch == e.chunk && next == e.start => whole = Some((ch, next + e.len)),
                        _ => whole_ok = false,
                    }
                }
            } else {
                let pos = if all_new == at { at } else { missing.len() };
                sl.add(Ent { key: k, chunk: PENDING, start: 0, len: 0, ask: 0, batch, pos: pos as u32 });
                if all_new == at {
                    all_new += 1;
                } else {
                    missing.push(k);
                }
            }
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
                Rc::new(m.gather(slot, keys, sc))
            };
            self.stats.memo_out += out.n as u64;
            if self.memo.prof {
                self.memo.t[0] += super::select_kernel::ticks() - t0;
            }
            return Ok(out);
        }
        m.n[3] += 1;
        if self.memo.prof {
            self.memo.t[0] += super::select_kernel::ticks() - t0;
        }
        let run_keys: &[u64] = if all_new == keys.len() { keys } else { &missing };
        let out = self.probe_run(f, run, p, node, col, run_keys, sc);
        let t1 = if self.memo.prof { super::select_kernel::ticks() } else { 0.0 };
        let m = &mut self.memo;
        let out = match out {
            Ok(o) => o,
            Err(e) => {
                let sl = &mut m.slots[slot];
                let run_keys: &[u64] = if all_new == keys.len() { keys } else { &missing };
                for k in run_keys {
                    if let Some(i) = sl.find(*k) {
                        sl.ents[i].chunk = DEAD;
                    }
                }
                m.missing = missing;
                return Err(e);
            }
        };
        self.stats.memo_in += out.n as u64;
        let mut answer: Option<Rc<Table>> = None;
        if out.n > 0 {
            // every row a key of this batch's (else the row-by-row filing), and whether they come grouped in its order
            let sl = &m.slots[slot];
            let kc = &out.cols[col];
            let mut grouped = true;
            let mut general = false;
            let mut last = 0u32;
            for &k in kc.iter().take(out.n) {
                match sl.find(k) {
                    Some(i) if sl.ents[i].batch == batch && sl.ents[i].chunk == PENDING && sl.ents[i].len == 0 => {
                        let pos = sl.ents[i].pos;
                        if pos < last {
                            grouped = false;
                        }
                        last = pos;
                    }
                    _ => {
                        general = true;
                        break;
                    }
                }
            }
            if general {
                m.file_rows(slot, &out, col);
            } else {
                let t = if grouped {
                    out
                } else {
                    // a stable counting sort of the rows by their key's place in the batch
                    let sl = &m.slots[slot];
                    let counts = &mut m.counts;
                    counts.clear();
                    counts.resize(fresh + 1, 0);
                    for &k in &out.cols[col][..out.n] {
                        let i = sl.find(k).expect("a key of the batch");
                        counts[sl.ents[i].pos as usize + 1] += 1;
                    }
                    for i in 1..counts.len() {
                        counts[i] += counts[i - 1];
                    }
                    let at = &mut m.perm;
                    at.clear();
                    at.resize(out.n, 0);
                    for r in 0..out.n {
                        let i = sl.find(out.cols[col][r]).expect("a key of the batch");
                        let p = &mut counts[sl.ents[i].pos as usize];
                        at[*p as usize] = r as u32;
                        *p += 1;
                    }
                    let mut t = Table::with(sc.names.clone(), sc.tys.clone());
                    for c in 0..t.cols.len() {
                        let src = &out.cols[c];
                        let mut v = std::mem::take(&mut t.cols[c]);
                        v.extend(at.iter().map(|r| src[*r as usize]));
                        t.cols[c] = v;
                    }
                    t.n = out.n;
                    Rc::new(t)
                };
                let sl = &mut m.slots[slot];
                let ch = sl.chunks.len() as u32;
                let kc = &t.cols[col];
                let mut r = 0;
                while r < t.n {
                    let k = kc[r];
                    let mut s = r + 1;
                    while s < t.n && kc[s] == k {
                        s += 1;
                    }
                    let i = sl.find(k).expect("a key of the batch");
                    let e = &mut sl.ents[i];
                    e.chunk = ch;
                    e.start = r as u32;
                    e.len = (s - r) as u32;
                    r = s;
                }
                sl.chunks.push(t.clone());
                if !hit_rows && sl.filed == filed0 {
                    m.n[4] += 1;
                    answer = Some(t);
                }
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
                Rc::new(m.gather(slot, keys, sc))
            }
        };
        m.missing = missing;
        self.stats.memo_out += answer.n as u64;
        if self.memo.prof {
            self.memo.t[1] += super::select_kernel::ticks() - t1;
        }
        Ok(answer)
    }
}
