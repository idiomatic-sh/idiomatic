//! The compiled rule pack (bead ryo-lpxl, throwaway): the loaded navigation programs as a flat array of u32 words (merged
//! prefix tries, made tables, rule anchors) plus a constant pool (the run's base symbols in id order, so every symbol code in
//! the words is final; regex patterns), and the tails (the pack's suppression fragment per rule) as JSON with each merged
//! node defined once. Written once per rule pack by `walk pack <program.json> <out.pack>`; read in one go at start.

use crate::nav::{Alt, Dir, FactRel, Group, Ins, NavProgram, NavRule, Opd, OrderedDfs, OrderedMatch, Prog, R, T, TRel, Val};
use crate::sema::FxMap;
use crate::symbols::Symbols;
use regex_automata::meta::Regex;
use serde_json::Value;

const MAGIC: u32 = 0x4e41_5650; // "NAVP"

struct Enc {
    w: Vec<u32>,
}

impl Enc {
    fn u(&mut self, x: u32) {
        self.w.push(x);
    }
    fn i(&mut self, x: i64) {
        self.w.push(x as u64 as u32);
        self.w.push(((x as u64) >> 32) as u32);
    }
    fn s(&mut self, x: &str) {
        let b = x.as_bytes();
        self.u(b.len() as u32);
        for c in b.chunks(4) {
            let mut a = [0u8; 4];
            a[..c.len()].copy_from_slice(c);
            self.u(u32::from_le_bytes(a));
        }
    }
    fn us(&mut self, xs: &[u32]) {
        self.u(xs.len() as u32);
        self.w.extend_from_slice(xs);
    }
    fn opd(&mut self, o: &Opd) {
        match o {
            Opd::Reg(r) => {
                self.u(0);
                self.u(u32::from(*r));
            }
            Opd::Num(n) => {
                self.u(1);
                self.i(*n);
            }
            Opd::Sym(s) => {
                self.u(2);
                self.u(*s);
            }
            Opd::Syms(s) => {
                self.u(3);
                self.us(s);
            }
            Opd::Nums(n) => {
                self.u(4);
                self.u(n.len() as u32);
                for &x in n.iter() {
                    self.i(x);
                }
            }
            Opd::File => self.u(5),
        }
    }
    fn val(&mut self, v: &Val) {
        match v {
            Val::Reg(r) => self.opd(&Opd::Reg(*r)),
            Val::Syms(s) => self.opd(&Opd::Syms(s.clone())),
            Val::Nums(n) => self.opd(&Opd::Nums(n.clone())),
        }
    }
    fn bound(&mut self, b: &Option<(R, bool)>) {
        match b {
            None => self.u(0),
            Some((r, incl)) => {
                self.u(1 + u32::from(*incl));
                self.u(u32::from(*r));
            }
        }
    }
    fn prog(&mut self, p: &[Ins]) {
        self.u(p.len() as u32);
        for i in p {
            self.ins(i);
        }
    }
    fn progs(&mut self, ps: &[Prog]) {
        self.u(ps.len() as u32);
        for p in ps {
            self.prog(p);
        }
    }
    fn ins(&mut self, i: &Ins) {
        self.u(i.code() as u32);
        match i {
            Ins::IterKind { kinds, out } => {
                self.us(kinds);
                self.u(u32::from(*out));
            }
            Ins::IterValue { rel, col, val, out } => {
                self.u(*rel as u32);
                self.u(u32::from(*col));
                self.val(val);
                self.u(u32::from(*out));
            }
            Ins::IterChildren { parent, out } => {
                self.u(u32::from(*parent));
                self.u(u32::from(*out));
            }
            Ins::IterScan { rel, out } => {
                self.u(*rel as u32);
                self.u(u32::from(*out));
            }
            Ins::IterFact { rel, col, val, out } => {
                self.u(u32::from(*rel));
                self.u(u32::from(*col));
                self.val(val);
                self.u(u32::from(*out));
            }
            Ins::IterFactScan { rel, out } => {
                self.u(u32::from(*rel));
                self.u(u32::from(*out));
            }
            Ins::Row { rel, key } => {
                self.u(*rel as u32);
                self.u(u32::from(*key));
            }
            Ins::Read { rel, col, row, out } => {
                self.u(*rel as u32);
                self.u(u32::from(*col));
                self.u(u32::from(*row));
                self.u(u32::from(*out));
            }
            Ins::ReadFact { rel, col, row, out } => {
                self.u(u32::from(*rel));
                self.u(u32::from(*col));
                self.u(u32::from(*row));
                self.u(u32::from(*out));
            }
            Ins::Test { test, l, r, text, num, regex } => {
                self.u(*test as u32);
                self.opd(l);
                self.opd(r);
                self.u(u32::from(*text) | (u32::from(*num) << 1));
                self.us(&regex.iter().map(|&x| x as u32).collect::<Vec<_>>());
            }
            Ins::Axis { dir, start, out } => {
                self.u(*dir as u32);
                self.u(u32::from(*start));
                self.u(u32::from(*out));
            }
            Ins::Near { start, out, subs } => {
                self.u(u32::from(*start));
                self.u(u32::from(*out));
                self.progs(subs);
            }
            Ins::Sub { negate, subs } => {
                self.u(u32::from(*negate));
                self.progs(subs);
            }
            Ins::Emit { cols, rule } => {
                self.u(cols.len() as u32);
                for c in cols.iter() {
                    self.opd(c);
                }
                self.u(*rule);
            }
            Ins::Yield => {}
            Ins::SpanBetween { a, b, out } => {
                self.opd(a);
                self.opd(b);
                self.u(u32::from(*out));
            }
            Ins::SpanBounds { s, start, end } => {
                self.opd(s);
                self.u(u32::from(*start));
                self.u(u32::from(*end));
            }
            Ins::SourceMatches { span, base, regex, group, outs, capture_text, capture_line } => {
                self.opd(span);
                match base {
                    None => self.u(0),
                    Some(b) => {
                        self.u(1);
                        self.opd(b);
                    }
                }
                self.u(*regex as u32);
                self.u(group.map_or(u32::MAX, |g| g as u32));
                self.us(&outs.iter().map(|&r| u32::from(r)).collect::<Vec<_>>());
                self.u(u32::from(*capture_text) | (u32::from(*capture_line) << 1));
            }
            Ins::Within { anc, desc } => {
                self.u(u32::from(*anc));
                self.u(u32::from(*desc));
            }
            Ins::IterRange { rel, col, val, out, start, lo, hi } => {
                self.u(*rel as u32);
                self.u(u32::from(*col));
                self.val(val);
                self.u(u32::from(*out));
                self.u(u32::from(*start));
                self.bound(lo);
                self.bound(hi);
            }
            Ins::IterField { parent, field, out, field_out } => {
                self.u(u32::from(*parent));
                self.u(*field);
                self.u(u32::from(*out));
                self.u(u32::from(*field_out));
            }
            // load-time specialisations (pack version 4 stores the specialised programs)
            Ins::ReadCol { which, row, out } | Ins::ReadChild { which, row, out } => {
                self.u(u32::from(*which));
                self.u(u32::from(*row));
                self.u(u32::from(*out));
            }
            Ins::TestEqC { reg, c } | Ins::TestNeC { reg, c } => {
                self.u(u32::from(*reg));
                self.i(*c as i64);
            }
            Ins::TestEqR { a, b } | Ins::TestNeR { a, b } => {
                self.u(u32::from(*a));
                self.u(u32::from(*b));
            }
            Ins::TestInC { reg, set, negate } => {
                self.u(u32::from(*reg));
                self.us(set);
                self.u(u32::from(*negate));
            }
            Ins::Hoist { id, key, seg, caps } => {
                self.u(*id);
                self.regs(key);
                self.prog(seg);
                self.regs(caps);
            }
            Ins::HoistProbe { id, key, seg, caps, probe } => {
                self.u(*id);
                self.regs(key);
                self.prog(seg);
                self.regs(caps);
                self.u(probe.0);
                self.u(u32::from(probe.1));
            }
            Ins::ChildToward { parent, desc, out, field } => {
                self.u(u32::from(*parent));
                self.u(u32::from(*desc));
                self.u(u32::from(*out));
                match field {
                    None => self.u(u32::MAX),
                    Some((f, fo)) => {
                        self.u(*f);
                        self.u(u32::from(*fo));
                    }
                }
            }
            Ins::Capture { regs } => self.regs(regs),
            Ins::SubFirst { subs } => self.progs(subs),
            Ins::NearMemo { start, out, subs } => {
                self.u(u32::from(*start));
                self.u(u32::from(*out));
                self.progs(subs);
            }
            Ins::SubKey { negate, key, subs } => {
                self.u(u32::from(*negate));
                self.u(u32::from(*key));
                self.progs(subs);
            }
            Ins::AncExists { negate, start, out, cond } => {
                self.u(u32::from(*negate));
                self.u(u32::from(*start));
                self.u(u32::from(*out));
                self.prog(cond);
            }
            Ins::Fork { subs } => {
                self.u(subs.len() as u32);
                for (slots, q) in subs.iter() {
                    self.us(slots);
                    self.prog(q);
                }
            }
            Ins::Collect { regs } => self.regs(regs),
            Ins::Closure { src, out, xin, kout, nkeys, subs } => {
                self.u(u32::from(*src));
                self.u(u32::from(*out));
                self.u(u32::from(*xin));
                self.regs(kout);
                self.u(*nkeys as u32);
                self.progs(subs);
            }
            Ins::Union { outs, subs, memo } => {
                self.regs(outs);
                self.u(u32::from(*memo));
                self.progs(subs);
            }
            Ins::Let { from, out } => {
                self.opd(from);
                self.u(u32::from(*out));
            }
            Ins::Shared { idx } => self.u(*idx),
            Ins::TextFn { lower, args, out } => {
                self.u(u32::from(*lower));
                self.u(args.len() as u32);
                for a in args.iter() {
                    self.opd(a);
                }
                self.u(u32::from(*out));
            }
            Ins::CaptureText { text, pattern, outs } => {
                self.opd(text);
                self.s(pattern);
                self.regs(outs);
            }
            Ins::Replace { text, regex, replacement, out } => {
                self.opd(text);
                self.u(*regex as u32);
                self.s(replacement);
                self.u(u32::from(*out));
            }
            Ins::NearDerived { start, out, xin, edges, domain } => {
                self.u(u32::from(*start));
                self.u(u32::from(*out));
                self.u(u32::from(*xin));
                self.progs(edges);
                self.progs(domain);
            }
            Ins::Sibling { next, start, out } => {
                self.u(u32::from(*next));
                self.u(u32::from(*start));
                self.u(u32::from(*out));
            }
            Ins::SubMemo { negate, subs } => {
                self.u(u32::from(*negate));
                self.progs(subs);
            }
        }
    }
    fn regs(&mut self, rs: &[R]) {
        self.us(&rs.iter().map(|&r| u32::from(r)).collect::<Vec<_>>());
    }
    fn cols(&mut self, cs: &[usize]) {
        self.us(&cs.iter().map(|&c| c as u32).collect::<Vec<_>>());
    }
}

struct Dec<'a> {
    w: &'a [u32],
    at: usize,
}

impl Dec<'_> {
    fn u(&mut self) -> u32 {
        self.at += 1;
        self.w[self.at - 1]
    }
    fn r(&mut self) -> R {
        self.u() as R
    }
    fn i(&mut self) -> i64 {
        let lo = u64::from(self.u());
        let hi = u64::from(self.u());
        (lo | (hi << 32)) as i64
    }
    fn s(&mut self) -> String {
        let n = self.u() as usize;
        let words = n.div_ceil(4);
        let mut b: Vec<u8> = Vec::with_capacity(words * 4);
        for _ in 0..words {
            b.extend_from_slice(&self.u().to_le_bytes());
        }
        b.truncate(n);
        String::from_utf8(b).expect("utf8")
    }
    fn us(&mut self) -> Box<[u32]> {
        let n = self.u() as usize;
        let v = self.w[self.at..self.at + n].to_vec().into_boxed_slice();
        self.at += n;
        v
    }
    fn opd(&mut self) -> Opd {
        match self.u() {
            0 => Opd::Reg(self.r()),
            1 => Opd::Num(self.i()),
            2 => Opd::Sym(self.u()),
            3 => Opd::Syms(self.us()),
            4 => {
                let n = self.u() as usize;
                Opd::Nums((0..n).map(|_| self.i()).collect::<Vec<_>>().into())
            }
            _ => Opd::File,
        }
    }
    fn val(&mut self) -> Val {
        match self.opd() {
            Opd::Reg(r) => Val::Reg(r),
            Opd::Syms(s) => Val::Syms(s),
            Opd::Nums(n) => Val::Nums(n),
            o => panic!("val {o:?}"),
        }
    }
    fn bound(&mut self) -> Option<(R, bool)> {
        match self.u() {
            0 => None,
            t => Some((self.r(), t == 2)),
        }
    }
    fn regs(&mut self) -> Box<[R]> {
        self.us().iter().map(|&r| r as R).collect::<Vec<_>>().into()
    }
    fn cols(&mut self) -> Vec<usize> {
        self.us().iter().map(|&c| c as usize).collect()
    }
    fn trel(&mut self) -> TRel {
        match self.u() {
            0 => TRel::Ss,
            1 => TRel::Sc,
            2 => TRel::Sl,
            _ => TRel::Id,
        }
    }
    fn prog(&mut self) -> Prog {
        let n = self.u() as usize;
        (0..n).map(|_| self.ins()).collect::<Vec<_>>().into()
    }
    fn progs(&mut self) -> Box<[Prog]> {
        let n = self.u() as usize;
        (0..n).map(|_| self.prog()).collect::<Vec<_>>().into()
    }
    fn ins(&mut self) -> Ins {
        let code = self.u();
        match code {
            0 => {
                let kinds = self.us();
                Ins::IterKind { kinds, out: self.r() }
            }
            1 => {
                let rel = self.trel();
                let col = self.u() as u8;
                let val = self.val();
                Ins::IterValue { rel, col, val, out: self.r() }
            }
            2 => {
                let parent = self.r();
                Ins::IterChildren { parent, out: self.r() }
            }
            3 => {
                let rel = self.trel();
                Ins::IterScan { rel, out: self.r() }
            }
            4 => {
                let rel = self.u() as u16;
                let col = self.u() as u8;
                let val = self.val();
                Ins::IterFact { rel, col, val, out: self.r() }
            }
            5 => {
                let rel = self.u() as u16;
                Ins::IterFactScan { rel, out: self.r() }
            }
            6 => {
                let rel = self.trel();
                Ins::Row { rel, key: self.r() }
            }
            7 => {
                let rel = self.trel();
                let col = self.u() as u8;
                let row = self.r();
                Ins::Read { rel, col, row, out: self.r() }
            }
            8 => {
                let rel = self.u() as u16;
                let col = self.u() as u8;
                let row = self.r();
                Ins::ReadFact { rel, col, row, out: self.r() }
            }
            9 => {
                let test = [T::Eq, T::Ne, T::Lt, T::Le, T::Gt, T::Ge, T::In, T::NotIn, T::Matches, T::Glob, T::NotGlob, T::Starts, T::Ends, T::NumGe, T::NatLt, T::Succ]
                    [self.u() as usize];
                let l = self.opd();
                let r = self.opd();
                let flags = self.u();
                let regex = self.us().iter().map(|&x| x as usize).collect::<Vec<_>>().into();
                Ins::Test { test, l, r, text: flags & 1 != 0, num: flags & 2 != 0, regex }
            }
            10 => {
                let dir = [Dir::Parent, Dir::Child, Dir::Ancestor, Dir::Descendant][self.u() as usize];
                let start = self.r();
                Ins::Axis { dir, start, out: self.r() }
            }
            11 => {
                let start = self.r();
                let out = self.r();
                Ins::Near { start, out, subs: self.progs() }
            }
            12 => {
                let negate = self.u() != 0;
                Ins::Sub { negate, subs: self.progs() }
            }
            13 => {
                let n = self.u() as usize;
                let cols = (0..n).map(|_| self.opd()).collect::<Vec<_>>().into();
                Ins::Emit { cols, rule: self.u() }
            }
            14 => Ins::Yield,
            15 => {
                let a = self.opd();
                let b = self.opd();
                Ins::SpanBetween { a, b, out: self.r() }
            }
            16 => {
                let s = self.opd();
                let start = self.r();
                Ins::SpanBounds { s, start, end: self.r() }
            }
            17 => {
                let span = self.opd();
                let base = if self.u() == 1 { Some(self.opd()) } else { None };
                let regex = self.u() as usize;
                let g = self.u();
                let group = (g != u32::MAX).then_some(g as usize);
                let outs = self.us().iter().map(|&r| r as R).collect::<Vec<_>>().into();
                let flags = self.u();
                Ins::SourceMatches { span, base, regex, group, outs, capture_text: flags & 1 != 0, capture_line: flags & 2 != 0 }
            }
            18 => {
                let anc = self.r();
                Ins::Within { anc, desc: self.r() }
            }
            19 => {
                let rel = self.trel();
                let col = self.u() as u8;
                let val = self.val();
                let out = self.r();
                let start = self.r();
                let lo = self.bound();
                let hi = self.bound();
                Ins::IterRange { rel, col, val, out, start, lo, hi }
            }
            20 => {
                let parent = self.r();
                let field = self.u();
                let out = self.r();
                Ins::IterField { parent, field, out, field_out: self.r() }
            }
            21 => {
                let n = self.u() as usize;
                let subs = (0..n)
                    .map(|_| {
                        let slots = self.us();
                        (slots, self.prog())
                    })
                    .collect::<Vec<_>>()
                    .into();
                Ins::Fork { subs }
            }
            32 => Ins::Collect { regs: self.regs() },
            33 => {
                let src = self.r();
                let out = self.r();
                let xin = self.r();
                let kout = self.regs();
                let nkeys = self.u() as usize;
                Ins::Closure { src, out, xin, kout, nkeys, subs: self.progs() }
            }
            34 => {
                let outs = self.regs();
                let memo = self.u() != 0;
                Ins::Union { outs, subs: self.progs(), memo }
            }
            35 => {
                let from = self.opd();
                Ins::Let { from, out: self.r() }
            }
            36 => Ins::Shared { idx: self.u() },
            37 => {
                let lower = self.u() != 0;
                let n = self.u() as usize;
                let args = (0..n).map(|_| self.opd()).collect::<Vec<_>>().into();
                Ins::TextFn { lower, args, out: self.r() }
            }
            38 => {
                let text = self.opd();
                let pattern = self.s().into();
                Ins::CaptureText { text, pattern, outs: self.regs() }
            }
            39 => {
                let text = self.opd();
                let regex = self.u() as usize;
                let replacement = self.s().into();
                Ins::Replace { text, regex, replacement, out: self.r() }
            }
            40 => {
                let start = self.r();
                let out = self.r();
                let xin = self.r();
                let edges = self.progs();
                Ins::NearDerived { start, out, xin, edges, domain: self.progs() }
            }
            41 => {
                let next = self.u() != 0;
                let start = self.r();
                Ins::Sibling { next, start, out: self.r() }
            }
            42 => {
                let negate = self.u() != 0;
                Ins::SubMemo { negate, subs: self.progs() }
            }
            22 | 23 => {
                let which = self.u() as u8;
                let row = self.r();
                let out = self.r();
                if code == 22 { Ins::ReadCol { which, row, out } } else { Ins::ReadChild { which, row, out } }
            }
            24 | 25 => {
                let reg = self.r();
                let c = self.i() as u64;
                if code == 24 { Ins::TestEqC { reg, c } } else { Ins::TestNeC { reg, c } }
            }
            26 | 27 => {
                let a = self.r();
                let b = self.r();
                if code == 26 { Ins::TestEqR { a, b } } else { Ins::TestNeR { a, b } }
            }
            28 => {
                let reg = self.r();
                let set = self.us();
                Ins::TestInC { reg, set, negate: self.u() != 0 }
            }
            29 | 46 => {
                let id = self.u();
                let key = self.regs();
                let seg = self.prog();
                let caps = self.regs();
                if code == 29 {
                    Ins::Hoist { id, key, seg, caps }
                } else {
                    let pc = self.u();
                    Ins::HoistProbe { id, key, seg, caps, probe: (pc, self.r()) }
                }
            }
            47 => {
                let parent = self.r();
                let desc = self.r();
                let out = self.r();
                let f = self.u();
                let field = if f == u32::MAX { None } else { Some((f, self.r())) };
                Ins::ChildToward { parent, desc, out, field }
            }
            30 => Ins::Capture { regs: self.regs() },
            31 => Ins::SubFirst { subs: self.progs() },
            43 => {
                let start = self.r();
                let out = self.r();
                Ins::NearMemo { start, out, subs: self.progs() }
            }
            44 => {
                let negate = self.u() != 0;
                let key = self.r();
                Ins::SubKey { negate, key, subs: self.progs() }
            }
            45 => {
                let negate = self.u() != 0;
                let start = self.r();
                let out = self.r();
                Ins::AncExists { negate, start, out, cond: self.prog() }
            }
            c => panic!("ins code {c}"),
        }
    }
}

fn cell(k: ryo_relations::domain::relations::types::CellKind) -> u32 {
    use ryo_relations::domain::relations::types::CellKind as C;
    match k {
        C::Node => 0,
        C::Symbol => 1,
        C::File => 2,
        C::Span => 3,
        C::Flag => 4,
        _ => 5,
    }
}

fn uncell(k: u32) -> ryo_relations::domain::relations::types::CellKind {
    use ryo_relations::domain::relations::types::CellKind as C;
    match k {
        0 => C::Node,
        1 => C::Symbol,
        2 => C::File,
        3 => C::Span,
        4 => C::Flag,
        _ => C::Number,
    }
}

/// The tails with each merged node defined once: (nodes by merged id with inputs named by merged id, per rule its merged ids
/// in step order, root position, emit key, id).
fn tails_dedup(v: &Value) -> Value {
    let mut nodes = serde_json::Map::new();
    let mut rules = Vec::new();
    for r in v["rules"].as_array().unwrap() {
        let steps = r["steps"].as_array().unwrap();
        let merged_of: FxMap<u64, u64> =
            steps.iter().map(|s| (s["step"].as_u64().unwrap(), s["merged"].as_u64().unwrap())).collect();
        let mut ids = Vec::new();
        for s in steps {
            let m = s["merged"].as_u64().unwrap();
            ids.push(Value::from(m));
            let key = m.to_string();
            if nodes.contains_key(&key) {
                continue;
            }
            let mut d = s.clone();
            let o = d.as_object_mut().unwrap();
            o.remove("step");
            o.remove("merged");
            for k in ["from", "left", "right", "edges", "domain"] {
                if let Some(Value::Object(x)) = o.get_mut(k) {
                    if let Some(st) = x.remove("step") {
                        x.insert("m".into(), Value::from(merged_of[&st.as_u64().unwrap()]));
                    }
                }
            }
            nodes.insert(key, d);
        }
        let root_pos = steps.iter().position(|s| s["step"] == r["root"]).unwrap();
        rules.push(serde_json::json!({"id": r["id"], "ids": ids, "root": root_pos, "emit_key": r["emit_key"]}));
    }
    serde_json::json!({"nodes": nodes, "rules": rules})
}

/// The tails back in the walk loader's format (step numbers = positions); unused (the loader reads the merged form).
#[allow(dead_code)]
fn tails_expand(t: &Value) -> Value {
    let nodes = t["nodes"].as_object().unwrap();
    let mut rules = Vec::new();
    for r in t["rules"].as_array().unwrap() {
        let ids: Vec<u64> = r["ids"].as_array().unwrap().iter().map(|x| x.as_u64().unwrap()).collect();
        let mut pos_of: FxMap<u64, usize> = FxMap::default();
        for (i, &m) in ids.iter().enumerate() {
            pos_of.entry(m).or_insert(i);
        }
        let steps: Vec<Value> = ids
            .iter()
            .enumerate()
            .map(|(i, &m)| {
                let mut d = nodes[&m.to_string()].clone();
                let o = d.as_object_mut().unwrap();
                o.insert("step".into(), Value::from(i));
                o.insert("merged".into(), Value::from(m));
                for k in ["from", "left", "right", "edges", "domain"] {
                    if let Some(Value::Object(x)) = o.get_mut(k) {
                        if let Some(mm) = x.remove("m") {
                            x.insert("step".into(), Value::from(pos_of[&mm.as_u64().unwrap()]));
                        }
                    }
                }
                d
            })
            .collect();
        rules.push(serde_json::json!({"id": r["id"], "root": r["root"], "emit_key": r["emit_key"], "steps": steps}));
    }
    serde_json::json!({"rules": rules})
}

/// Writes the pack: [magic, version, tails JSON length, nav word count], the tails JSON (padded to 4 bytes), the words.
pub(crate) fn write(v: &Value, symbols: &Symbols, nav: &NavProgram, out: &str) -> usize {
    let regex_patterns = &nav.regex_patterns;
    let tails = serde_json::to_string(&tails_dedup(v)).unwrap();
    let mut e = Enc { w: Vec::new() };
    // constant pool: base symbols in id order, then regex patterns
    let texts = symbols.base_texts();
    e.u(texts.len() as u32);
    for t in texts {
        e.s(t);
    }
    e.u(regex_patterns.len() as u32);
    for p in regex_patterns {
        e.s(p);
    }
    e.u(nav.max_regs as u32);
    e.u(nav.nslots as u32);
    // facts
    e.u(nav.facts.len() as u32);
    for f in &nav.facts {
        e.s(&f.name);
        e.us(&f.kinds.iter().map(|&k| cell(k)).collect::<Vec<_>>());
        e.u(f.nregs as u32);
        match f.keyed {
            None => e.u(u32::MAX),
            Some((c, r)) => {
                e.u(u32::from(c));
                e.u(u32::from(r));
            }
        }
        e.u(u32::from(f.base));
        match &f.made {
            None => e.u(0),
            Some((progs, keys, distinct, width)) => {
                e.u(1);
                e.progs(progs);
                e.us(&keys.iter().map(|&k| k as u32).collect::<Vec<_>>());
                e.u(distinct.map_or(u32::MAX, |d| d as u32));
                e.u(*width as u32);
            }
        }
        match &f.made_om {
            None => e.u(0),
            Some(om) => {
                e.u(1);
                e.progs(&om.left);
                e.progs(&om.right);
                for x in [om.lw, om.rw, om.lo, om.ro] {
                    e.u(x as u32);
                }
                e.cols(&om.left_on);
                e.cols(&om.right_on);
                e.u(u32::from(om.number));
                e.u(om.output.len() as u32);
                for &(right, col) in &om.output {
                    e.u(u32::from(right));
                    e.u(col as u32);
                }
                match &om.dfs {
                    None => e.u(0),
                    Some(d) => {
                        e.u(1);
                        for x in [d.root, d.source, d.destination, d.order, d.edge] {
                            e.u(x as u32);
                        }
                        e.cols(&d.root_keys);
                        e.cols(&d.edge_keys);
                    }
                }
            }
        }
    }
    // rules: id, core columns, xmerged, per branch the anchors of its alternatives (the per-file empty-bucket skip)
    e.u(nav.rules.len() as u32);
    for r in &nav.rules {
        e.s(&r.id);
        e.u(r.xmerged);
        e.us(&r.kinds.iter().map(|&k| u32::from(k)).collect::<Vec<_>>());
        e.u(r.programs.len() as u32);
        for alts in &r.programs {
            e.u(alts.len() as u32);
            for a in alts {
                match &a.anchor {
                    None => e.u(u32::MAX),
                    Some(k) => e.us(k),
                }
            }
        }
        e.u(r.aux.len() as u32);
        for (m, ps, kinds) in &r.aux {
            e.u(*m);
            e.progs(ps);
            e.us(&kinds.iter().map(|&k| u32::from(k)).collect::<Vec<_>>());
        }
    }
    e.progs(&nav.shared);
    e.u(u32::from(nav.needs_provider));
    e.us(&nav.slot_rule);
    for s in &nav.slot_of {
        e.us(s);
    }
    e.u(nav.groups.len() as u32);
    for g in &nav.groups {
        match &g.kinds {
            None => e.u(u32::MAX),
            Some(k) => e.us(k),
        }
        e.us(&g.slots);
        e.prog(&g.prog);
    }
    let mut bytes: Vec<u8> = Vec::new();
    for x in [MAGIC, 4, tails.len() as u32, e.w.len() as u32] {
        bytes.extend_from_slice(&x.to_le_bytes());
    }
    bytes.extend_from_slice(tails.as_bytes());
    while bytes.len() % 4 != 0 {
        bytes.push(b' ');
    }
    for x in &e.w {
        bytes.extend_from_slice(&x.to_le_bytes());
    }
    std::fs::write(out, &bytes).expect("write pack");
    bytes.len()
}

/// Reads a pack: the symbols (every base symbol at its id), the tails in the loader's format, the navigation program.
pub(crate) fn read(path: &str) -> (Symbols, Value, NavProgram) {
    let bytes = std::fs::read(path).expect("pack");
    let word = |i: usize| u32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap());
    assert_eq!(word(0), MAGIC, "pack magic");
    assert_eq!(word(1), 4, "pack version");
    let tails_len = word(2) as usize;
    let nwords = word(3) as usize;
    let t0 = std::time::Instant::now();
    let tails: Value = serde_json::from_slice(&bytes[16..16 + tails_len]).expect("tails");
    let t_tails = t0.elapsed().as_secs_f64();
    let off = (16 + tails_len).div_ceil(4) * 4;
    let words: Vec<u32> = bytes[off..off + nwords * 4].chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
    // the file's bytes are not needed past this point (peak memory: the words and the decoded programs only)
    drop(bytes);
    let mut d = Dec { w: &words, at: 0 };
    let mut symbols = Symbols::default();
    let n = d.u();
    for _ in 0..n {
        let t = d.s();
        symbols.base(&t);
    }
    let t_sym = t0.elapsed().as_secs_f64();
    let n = d.u();
    let regexes: Vec<Regex> = (0..n).map(|_| Regex::new(&d.s()).expect("regex")).collect();
    let t_re = t0.elapsed().as_secs_f64();
    let max_regs = d.u() as usize;
    let nslots = d.u() as usize;
    let n = d.u();
    let mut facts = Vec::new();
    for _ in 0..n {
        let name = d.s();
        let kinds = d.us().iter().map(|&k| uncell(k)).collect::<Vec<_>>();
        let nregs = d.u() as usize;
        let kc = d.u();
        let keyed = if kc == u32::MAX { None } else { Some((kc as u8, d.r())) };
        let base = d.u() as u16;
        let made = if d.u() == 1 {
            let progs = d.progs().into_vec();
            let keys = d.us().iter().map(|&k| k as usize).collect();
            let dd = d.u();
            let distinct = (dd != u32::MAX).then_some(dd as usize);
            let width = d.u() as usize;
            Some((progs, keys, distinct, width))
        } else {
            None
        };
        let made_om = if d.u() == 1 {
            let left = d.progs().into_vec();
            let right = d.progs().into_vec();
            let (lw, rw, lo, ro) = (d.u() as usize, d.u() as usize, d.u() as usize, d.u() as usize);
            let left_on = d.cols();
            let right_on = d.cols();
            let number = d.u() != 0;
            let n = d.u() as usize;
            let output = (0..n).map(|_| (d.u() != 0, d.u() as usize)).collect();
            let dfs = if d.u() == 1 {
                let (root, source, destination, order, edge) = (d.u() as usize, d.u() as usize, d.u() as usize, d.u() as usize, d.u() as usize);
                let root_keys = d.cols();
                let edge_keys = d.cols();
                Some(OrderedDfs { root, source, destination, order, edge, root_keys, edge_keys })
            } else {
                None
            };
            Some(OrderedMatch { left, right, lw, rw, left_on, right_on, lo, ro, number, output, dfs })
        } else {
            None
        };
        let (model, relation) = if made.is_some() || made_om.is_some() {
            (None, None)
        } else {
            use ryo_relations::domain::labels::types::NameRef;
            use ryo_relations::domain::relations::service::relation_named;
            let relation = relation_named(NameRef(&name));
            let model =
                if std::env::var("NAV_PROVIDER_ONLY").is_ok() { None } else { crate::semrows::RELATIONS.iter().position(|r| *r == name) };
            (model, relation)
        };
        facts.push(FactRel { model, name, relation, kinds, made, made_om, keyed, base, nregs });
    }
    let n = d.u();
    let mut rules = Vec::new();
    for _ in 0..n {
        let id = d.s();
        let xmerged = d.u();
        let kinds = d.us().iter().map(|&k| k as u8).collect();
        let nb = d.u();
        let programs = (0..nb)
            .map(|_| {
                let na = d.u();
                (0..na)
                    .map(|_| {
                        let anchor = if d.w[d.at] == u32::MAX {
                            d.at += 1;
                            None
                        } else {
                            Some(d.us())
                        };
                        Alt { prog: Vec::new().into_boxed_slice(), anchor, est: 0.0, per: 0.0 }
                    })
                    .collect()
            })
            .collect();
        let na = d.u();
        let aux = (0..na)
            .map(|_| {
                let m = d.u();
                let ps = d.progs().into_vec();
                let kinds = d.us().iter().map(|&k| k as u8).collect();
                (m, ps, kinds)
            })
            .collect();
        rules.push(NavRule { id, programs, xmerged, kinds, nregs: 0, aux });
    }
    let shared = d.progs().into_vec();
    let needs_provider = d.u() != 0;
    let slot_rule = d.us().into_vec();
    let slot_of = (0..rules.len()).map(|_| d.us().into_vec()).collect();
    let n = d.u();
    let mut groups = Vec::new();
    for _ in 0..n {
        let kinds = if d.w[d.at] == u32::MAX {
            d.at += 1;
            None
        } else {
            Some(d.us())
        };
        let slots = d.us();
        groups.push(Group { kinds, slots, prog: d.prog() });
    }
    let t_nav = t0.elapsed().as_secs_f64();
    if std::env::var("NAV_VERBOSE").is_ok() {
        eprintln!("pack sizes: Ins {} B, Opd {} B, Val {} B", std::mem::size_of::<Ins>(), std::mem::size_of::<Opd>(), std::mem::size_of::<Val>());
    }
    if std::env::var("NAV_VERBOSE").is_ok() {
        eprintln!(
            "pack read: tails json {t_tails:.3} symbols {t_sym:.3} regexes {t_re:.3} nav {t_nav:.3} done {:.3} (cumulative s); {} regexes",
            t0.elapsed().as_secs_f64(),
            regexes.len()
        );
    }
    let mut nav = NavProgram { groups, slot_of, slot_rule, nslots, rules, shared, facts, regexes, regex_patterns: Vec::new(), max_regs, needs_provider };
    // the pack stores the specialised programs (written after specialize_all)
    (symbols, tails, nav)
}
