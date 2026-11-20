//! The compact executable form of the navigation programs and its dispatch loop (bead ryo-dxw1, throwaway).
//!
//! The instruction enum of nav.rs stays the IR every load-time pass works on. `encode` is the last pass: every program of the
//! pack (merged groups, made tables, ordered matches, aux scans, shared sub-programs and every nested sub-program) becomes a
//! run of fixed 16-byte words in ONE arena; operands are register numbers and indexes into three side tables (a u32 pool of
//! lists, a u64 constant table, a string table). The pack stores the arena and the tables as they are; a run reads them with
//! one read each and executes from them: no per-instruction allocation, no decode pass.
//!
//! The loop (`xrun`) keeps one frame per running iteration in a local array, so an iteration's next element is a jump, not a
//! call; sub-programs (checks, hoisted segments, closure edges) are calls. Memo sites are numbered at encoding time and their
//! per-file state sits in arrays stamped with the file's generation (nothing is cleared per file but the keyed maps).

use crate::nav::{Dir, Exec, FIdx, Ins, NavProgram, Opd, Prog, R, T, TRel, Val};
use crate::sema::FxMap;
use crate::symbols::NULL;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(crate) struct XIns {
    pub op: u8,
    pub f: u8,
    pub a: u16,
    pub b: u16,
    pub c: u16,
    pub x: u32,
    pub y: u32,
}
const _: () = assert!(std::mem::size_of::<XIns>() == 16);

macro_rules! ops {
    ($($name:ident = $v:expr, $code:expr;)*) => {
        #[allow(dead_code)]
        pub(crate) mod op { $(pub const $name: u8 = $v;)* }
        /// opcode -> (name, the IR instruction's code it stands for: nav::INS_NAMES)
        pub(crate) const XOPS: &[(u8, &str, usize)] = &[$(($v, stringify!($name), $code),)*];
    };
}

// name = opcode, IR code (nav::Ins::code) charged in the per-kind step counters
ops! {
    FAIL = 0, 14;
    READ_COL = 1, 22;
    READ_CHILD = 2, 23;
    TEST_EQ_C = 3, 24;
    TEST_EQ_K = 4, 24;
    TEST_NE_C = 5, 25;
    TEST_NE_K = 6, 25;
    TEST_EQ_R = 7, 26;
    TEST_NE_R = 8, 27;
    TEST_IN_C = 9, 28;
    READ = 10, 7;
    TEST = 11, 9;
    ROW = 12, 6;
    READ_FACT = 13, 8;
    ITER_CHILDREN = 14, 2;
    ITER_KIND = 15, 0;
    ITER_VALUE_C = 16, 1;
    ITER_VALUE_R = 17, 1;
    ITER_SCAN = 18, 3;
    ITER_FACT_KEYED = 19, 4;
    ITER_FACT_R = 20, 4;
    ITER_FACT_C = 21, 4;
    ITER_FACT_L = 22, 4;
    ITER_FACT2 = 23, 49;
    ITER_FACT_SCAN = 24, 5;
    AXIS_PARENT = 25, 10;
    AXIS_ANC = 26, 10;
    AXIS_CHILD = 27, 10;
    AXIS_DESC = 28, 10;
    WITHIN = 29, 18;
    ITER_RANGE = 30, 19;
    ITER_FIELD = 31, 20;
    NEAR = 32, 11;
    SUB = 33, 12;
    EMIT = 34, 13;
    FORK = 35, 21;
    HOIST = 36, 29;
    CAPTURE = 37, 30;
    HOIST_PROBE = 38, 46;
    CHILD_TOWARD = 39, 47;
    UNION_KEY = 40, 48;
    SUB_FIRST = 41, 31;
    YIELD = 42, 14;
    SPAN_BETWEEN = 43, 15;
    SPAN_BOUNDS = 44, 16;
    SOURCE_MATCHES = 45, 17;
    CLOSURE = 46, 33;
    COLLECT = 47, 32;
    UNION = 48, 34;
    LET = 49, 35;
    JUMP = 50, 36;
    TEXT_FN = 51, 37;
    CAPTURE_TEXT = 52, 38;
    REPLACE = 53, 39;
    NEAR_DERIVED = 54, 40;
    SIBLING = 55, 41;
    SUB_MEMO = 56, 42;
    NEAR_MEMO = 57, 43;
    SUB_KEY = 58, 44;
    ANC_EXISTS = 59, 45;
    TEST_CMP = 60, 9;
    ROW_ANY = 61, 6;
    ROW_CHILD = 62, 6;
    SUB1 = 63, 12;
    // fused pairs (stage 2): a read and the constant test of what it read; each counts the two steps it stands for
    RC_EQ = 64, 22;
    RC_NE = 65, 22;
    RC_IN = 66, 22;
    RCH_EQ = 67, 23;
    RCH_NE = 68, 23;
    RCH_IN = 69, 23;
    RF_EQ = 70, 8;
    RF_NE = 71, 8;
    RF_IN = 72, 8;
    RF_EQ_R = 73, 8;
    RC_EQ_R = 74, 22;
    RCH_EQ_R = 75, 23;
    EQ_EQ = 76, 24;
    FIELD_KIND_EQ = 77, 20;
    FIELD_KIND_IN = 78, 20;
    CHILD_KIND_EQ = 79, 2;
}

pub(crate) const NXOPS: usize = 96;

/// opcode -> name
pub(crate) fn xop_name(op: usize) -> &'static str {
    XOPS.iter().find(|o| o.0 as usize == op).map_or("?", |o| o.1)
}

fn xop_code(op: u8) -> usize {
    XOPS.iter().find(|o| o.0 == op).map_or(14, |o| o.2)
}

/// The pack's programs in executable form.
#[derive(Default)]
pub(crate) struct Compact {
    pub on: bool,
    pub code: Vec<XIns>,
    /// lists: registers, kinds, value sets, sub-program entries, operand descriptions
    pub pool: Vec<u32>,
    pub k64: Vec<u64>,
    pub strs: Vec<String>,
    /// constant value walks of tree columns, by content: (relation, column, sorted by start, pool offset of [n, values...])
    pub consts: Vec<(u8, u8, bool, u32)>,
    /// fact column indexes the programs walk: (fact, column); tree column indexes: (relation, column); two-column fact
    /// indexes: (fact, column, column)
    pub fidx: Vec<(u16, u8)>,
    pub tidx: Vec<(u8, u8)>,
    pub f2: Vec<(u16, u8, u8)>,
    pub group_entry: Vec<u32>,
    /// the anchor register of a kind-anchored group (u32::MAX: none)
    pub group_out: Vec<u32>,
    /// the rule a group's steps are charged to (the rule count: shared by several)
    pub group_rule: Vec<u32>,
    pub shared_entry: Vec<u32>,
    /// memo sites: hoists, keyed checks and unions, per-file checks, per-file unions, per-node memos, fork children
    pub sites: [u32; 6],
    /// opcode -> IR code (the per-kind step counters)
    pub xcode: Vec<u8>,
    /// entries of the made tables' programs (per fact), of the ordered matches' inputs (per fact: left, right) and of the
    /// rules' aux scans (per rule, per aux relation)
    pub fact_e: Vec<Vec<u32>>,
    pub om_e: Vec<(Vec<u32>, Vec<u32>)>,
    pub aux_e: Vec<Vec<Vec<u32>>>,
}

const S_HOIST: usize = 0;
const S_KEY: usize = 1;
const S_SUBMEMO: usize = 2;
const S_UNION: usize = 3;
const S_NODE: usize = 4;
const S_FORK: usize = 5;

// operand words: a register, or bit 31 and an index into the u64 constants
const XK: u32 = 0x8000_0000;
// tagged operands (two pool words) of the cold instructions that need an operand's kind
const TG_REG: u32 = 0;
const TG_NUM: u32 = 1;
const TG_SYM: u32 = 2;
const TG_SYMS: u32 = 3;
const TG_NUMS: u32 = 4;
const TG_FILE: u32 = 5;

const TESTS: [T; 16] =
    [T::Eq, T::Ne, T::Lt, T::Le, T::Gt, T::Ge, T::In, T::NotIn, T::Matches, T::Glob, T::NotGlob, T::Starts, T::Ends, T::NumGe, T::NatLt, T::Succ];

fn test_code(t: T) -> u8 {
    TESTS.iter().position(|x| *x == t).unwrap() as u8
}

fn rel_code(r: TRel) -> u8 {
    match r {
        TRel::Ss => 0,
        TRel::Sc => 1,
        TRel::Sl => 2,
        TRel::Id => 3,
    }
}

const RELS: [TRel; 4] = [TRel::Ss, TRel::Sc, TRel::Sl, TRel::Id];

// ------------------------------------------------------------------ the encoder

struct Encoder<'n> {
    nav: &'n NavProgram,
    x: Compact,
    k64_at: FxMap<u64, u32>,
    list_at: FxMap<Vec<u32>, u32>,
    const_at: FxMap<(u8, u8, bool, Vec<u32>), u32>,
    fidx_at: FxMap<(u16, u8), u32>,
    tidx_at: FxMap<(u8, u8), u32>,
    f2_at: FxMap<(u16, u8, u8), u32>,
    hoist_at: FxMap<u32, u32>,
    str_at: FxMap<String, u32>,
    jumps: Vec<(usize, u32)>,
    shared_done: usize,
    max_reg: usize,
    sentinels: usize,
    fuse: bool,
    pub fused: usize,
}

impl Encoder<'_> {
    fn reg(&mut self, r: R) -> u16 {
        self.max_reg = self.max_reg.max(r as usize + 1);
        r
    }

    fn k(&mut self, v: u64) -> u32 {
        if let Some(&i) = self.k64_at.get(&v) {
            return i;
        }
        let i = self.x.k64.len() as u32;
        self.x.k64.push(v);
        self.k64_at.insert(v, i);
        i
    }

    /// a list in the pool (identical lists stored once); its offset
    fn list(&mut self, xs: Vec<u32>) -> u32 {
        if let Some(&o) = self.list_at.get(&xs) {
            return o;
        }
        let o = self.x.pool.len() as u32;
        self.x.pool.extend_from_slice(&xs);
        self.list_at.insert(xs, o);
        o
    }

    fn site(&mut self, class: usize) -> u32 {
        let s = self.x.sites[class];
        self.x.sites[class] += 1;
        s
    }

    fn str(&mut self, s: &str) -> u32 {
        if let Some(&i) = self.str_at.get(s) {
            return i;
        }
        let i = self.x.strs.len() as u32;
        self.x.strs.push(s.to_owned());
        self.str_at.insert(s.to_owned(), i);
        i
    }

    /// an operand's value as one word
    fn xo(&mut self, o: &Opd) -> u32 {
        match o {
            Opd::Reg(r) => u32::from(self.reg(*r)),
            Opd::Num(n) => XK | self.k(*n as u64),
            Opd::Sym(s) => XK | self.k(u64::from(*s)),
            _ => XK | self.k(0),
        }
    }

    /// an operand with its kind: two words
    fn tagged(&mut self, o: &Opd) -> [u32; 2] {
        match o {
            Opd::Reg(r) => [TG_REG, u32::from(self.reg(*r))],
            Opd::Num(n) => [TG_NUM, self.k(*n as u64)],
            Opd::Sym(s) => [TG_SYM, *s],
            Opd::Syms(s) => {
                let mut l = vec![s.len() as u32];
                l.extend_from_slice(s);
                [TG_SYMS, self.list(l)]
            }
            Opd::Nums(n) => {
                // a run of constants in the u64 table (not shared: the run must be contiguous)
                let at = self.x.k64.len() as u32;
                self.x.k64.extend(n.iter().map(|&x| x as u64));
                [TG_NUMS, self.list(vec![n.len() as u32, at])]
            }
            Opd::File => [TG_FILE, 0],
        }
    }

    fn regs(&mut self, rs: &[R]) -> Vec<u32> {
        rs.iter().map(|&r| u32::from(self.reg(r))).collect()
    }

    fn const_list(&mut self, rel: TRel, col: u8, val: &Val, sorted: bool) -> u32 {
        let wanted: Vec<u32> = match val {
            Val::Syms(s) => s.to_vec(),
            Val::Nums(v) => v.iter().map(|&x| x as u32).collect(),
            Val::Reg(_) => unreachable!(),
        };
        let key = (rel_code(rel), col, sorted, wanted);
        if let Some(&i) = self.const_at.get(&key) {
            return i;
        }
        let mut l = vec![key.3.len() as u32];
        l.extend_from_slice(&key.3);
        let off = self.list(l);
        let i = self.x.consts.len() as u32;
        self.x.consts.push((key.0, col, sorted, off));
        self.const_at.insert(key, i);
        i
    }

    fn fidx(&mut self, rel: u16, col: u8) -> u16 {
        if let Some(&i) = self.fidx_at.get(&(rel, col)) {
            return i as u16;
        }
        let i = self.x.fidx.len() as u32;
        assert!(i < 65536);
        self.x.fidx.push((rel, col));
        self.fidx_at.insert((rel, col), i);
        i as u16
    }

    fn tidx(&mut self, rel: TRel, col: u8) -> u32 {
        let key = (rel_code(rel), col);
        if let Some(&i) = self.tidx_at.get(&key) {
            return i;
        }
        let i = self.x.tidx.len() as u32;
        self.x.tidx.push(key);
        self.tidx_at.insert(key, i);
        i
    }

    fn f2(&mut self, rel: u16, c1: u8, c2: u8) -> u16 {
        if let Some(&i) = self.f2_at.get(&(rel, c1, c2)) {
            return i as u16;
        }
        let i = self.x.f2.len() as u32;
        assert!(i < 65536);
        self.x.f2.push((rel, c1, c2));
        self.f2_at.insert((rel, c1, c2), i);
        i as u16
    }

    /// [n, entries...] of a sub-program list
    fn sublist(&mut self, es: &[u32]) -> u32 {
        let mut l = vec![es.len() as u32];
        l.extend_from_slice(es);
        self.list(l)
    }

    fn progs(&mut self, ps: &[Prog]) -> Vec<u32> {
        ps.iter().map(|p| self.prog(p)).collect()
    }

    /// Encodes one program (its sub-programs first, so its own words are contiguous); returns its entry.
    fn prog(&mut self, p: &[Ins]) -> u32 {
        if let [Ins::Shared { idx }] = p {
            if (*idx as usize) < self.shared_done {
                return self.x.shared_entry[*idx as usize];
            }
        }
        let subs: Vec<Vec<u32>> = p
            .iter()
            .map(|i| {
                let mut es = Vec::new();
                crate::nav::subs_ref(i, &mut |q| es.push(q));
                es.into_iter().map(|q| self.prog(q)).collect()
            })
            .collect();
        let entry = self.x.code.len() as u32;
        let mut k = 0;
        while k < p.len() {
            if self.fuse && k + 1 < p.len() {
                if let Some(w) = self.fused_pair(&p[k], &p[k + 1]) {
                    self.x.code.push(w);
                    self.fused += 1;
                    k += 2;
                    continue;
                }
            }
            let w = self.ins(&p[k], &subs[k]);
            if w.op == op::JUMP {
                self.jumps.push((self.x.code.len(), w.x));
            }
            self.x.code.push(w);
            k += 1;
        }
        let terminal = matches!(
            p.last(),
            Some(Ins::Emit { .. } | Ins::Yield | Ins::Capture { .. } | Ins::Collect { .. } | Ins::Shared { .. } | Ins::Fork { .. })
        );
        if !terminal {
            self.x.code.push(XIns { op: op::FAIL, ..XIns::default() });
            self.sentinels += 1;
        }
        entry
    }

    /// a read followed by a constant test of the register it wrote (the register is still written)
    fn fused_pair(&mut self, a: &Ins, b: &Ins) -> Option<XIns> {
        let (kind, row, out, sel): (u8, R, R, u32) = match a {
            Ins::ReadCol { which, row, out } => (0, *row, *out, u32::from(*which)),
            Ins::ReadChild { which, row, out } => (1, *row, *out, 4 + u32::from(*which)),
            Ins::ReadFact { rel, col, row, out } => (2, *row, *out, (u32::from(*rel) << 8) | u32::from(*col)),
            Ins::TestEqC { reg, c } => {
                // two constant equalities in a row
                let Ins::TestEqC { reg: r2, c: c2 } = b else { return None };
                if *c > u64::from(u32::MAX) || *c2 > u64::from(u32::MAX) {
                    return None;
                }
                return Some(XIns { op: op::EQ_EQ, f: 0, a: self.reg(*reg), b: self.reg(*r2), c: 0, x: *c as u32, y: *c2 as u32 });
            }
            _ => return None,
        };
        // operands: a = row, b = out; READ_COL/READ_CHILD: f = column, x = constant or set, y = set length;
        // READ_FACT: y = fact column, x = constant, (sets: x = pool offset of [n, values...])
        let (a16, b16) = (self.reg(row), self.reg(out));
        let base = [op::RC_EQ, op::RCH_EQ, op::RF_EQ][kind as usize];
        match b {
            Ins::TestEqC { reg, c } | Ins::TestNeC { reg, c } if *reg == out && *c <= u64::from(u32::MAX) => {
                let o = base + u8::from(matches!(b, Ins::TestNeC { .. }));
                if kind == 2 { Some(XIns { op: o, f: 0, a: a16, b: b16, c: 0, x: *c as u32, y: sel }) } else { Some(XIns { op: o, f: sel as u8, a: a16, b: b16, c: 0, x: *c as u32, y: 0 }) }
            }
            Ins::TestInC { reg, set, negate } if *reg == out => {
                let mut l = vec![set.len() as u32];
                l.extend_from_slice(set);
                let off = self.list(l);
                if kind == 2 {
                    Some(XIns { op: base + 2, f: u8::from(*negate), a: a16, b: b16, c: 0, x: off, y: sel })
                } else {
                    Some(XIns { op: base + 2, f: (sel as u8) | (u8::from(*negate) << 7), a: a16, b: b16, c: 0, x: off, y: 0 })
                }
            }
            Ins::TestEqR { a: ra, b: rb } if *ra == out || *rb == out => {
                let other = if *ra == out { *rb } else { *ra };
                let o = [op::RC_EQ_R, op::RCH_EQ_R, op::RF_EQ_R][kind as usize];
                let c16 = self.reg(other);
                if kind == 2 { Some(XIns { op: o, f: 0, a: a16, b: b16, c: c16, x: 0, y: sel }) } else { Some(XIns { op: o, f: sel as u8, a: a16, b: b16, c: c16, x: 0, y: 0 }) }
            }
            _ => None,
        }
    }

    fn ins(&mut self, i: &Ins, subs: &[u32]) -> XIns {
        let z = XIns::default();
        match i {
            Ins::ReadCol { which, row, out } => XIns { op: op::READ_COL, f: *which, a: self.reg(*row), b: self.reg(*out), ..z },
            Ins::ReadChild { which, row, out } => XIns { op: op::READ_CHILD, f: 4 + *which, a: self.reg(*row), b: self.reg(*out), ..z },
            Ins::TestEqC { reg, c } => {
                if *c <= u64::from(u32::MAX) {
                    XIns { op: op::TEST_EQ_C, a: self.reg(*reg), x: *c as u32, ..z }
                } else {
                    XIns { op: op::TEST_EQ_K, a: self.reg(*reg), x: self.k(*c), ..z }
                }
            }
            Ins::TestNeC { reg, c } => {
                if *c <= u64::from(u32::MAX) {
                    XIns { op: op::TEST_NE_C, a: self.reg(*reg), x: *c as u32, ..z }
                } else {
                    XIns { op: op::TEST_NE_K, a: self.reg(*reg), x: self.k(*c), ..z }
                }
            }
            Ins::TestEqR { a, b } => XIns { op: op::TEST_EQ_R, a: self.reg(*a), b: self.reg(*b), ..z },
            Ins::TestNeR { a, b } => XIns { op: op::TEST_NE_R, a: self.reg(*a), b: self.reg(*b), ..z },
            Ins::TestInC { reg, set, negate } => {
                let off = self.list(set.to_vec());
                XIns { op: op::TEST_IN_C, f: u8::from(*negate), a: self.reg(*reg), x: off, y: set.len() as u32, ..z }
            }
            Ins::Read { rel, col, row, out } => XIns { op: op::READ, f: (rel_code(*rel) << 4) | *col, a: self.reg(*row), b: self.reg(*out), ..z },
            Ins::Test { test, l, r, text, num, regex } => {
                // an ordered comparison of plain values: decided here, no operand kinds at run time
                if matches!(test, T::Lt | T::Le | T::Gt | T::Ge) && !*text && matches!(l, Opd::Reg(_) | Opd::Num(_) | Opd::Sym(_)) && matches!(r, Opd::Reg(_) | Opd::Num(_) | Opd::Sym(_)) {
                    return XIns { op: op::TEST_CMP, f: test_code(*test) | (u8::from(*num) << 4), x: self.xo(l), y: self.xo(r), ..z };
                }
                let mut d = Vec::new();
                d.extend(self.tagged(l));
                d.extend(self.tagged(r));
                d.push(regex.len() as u32);
                d.extend(regex.iter().map(|&x| x as u32));
                let off = self.list(d);
                XIns { op: op::TEST, f: test_code(*test) | (u8::from(*text) << 4) | (u8::from(*num) << 5), x: off, ..z }
            }
            Ins::Row { rel, key } => match rel {
                TRel::Ss | TRel::Sl => XIns { op: op::ROW_ANY, a: self.reg(*key), ..z },
                TRel::Sc => XIns { op: op::ROW_CHILD, a: self.reg(*key), ..z },
                TRel::Id => XIns { op: op::ROW, f: rel_code(*rel), a: self.reg(*key), ..z },
            },
            Ins::ReadFact { rel, col, row, out } => XIns { op: op::READ_FACT, f: *col, a: self.reg(*row), b: self.reg(*out), x: u32::from(*rel), ..z },
            Ins::IterChildren { parent, out } => XIns { op: op::ITER_CHILDREN, a: self.reg(*out), b: self.reg(*parent), ..z },
            Ins::IterKind { kinds, out } => {
                let off = self.list(kinds.to_vec());
                XIns { op: op::ITER_KIND, a: self.reg(*out), x: off, y: kinds.len() as u32, ..z }
            }
            Ins::IterValue { rel, col, val, out } => match val {
                Val::Reg(r) => XIns { op: op::ITER_VALUE_R, a: self.reg(*out), b: self.reg(*r), x: self.tidx(*rel, *col), ..z },
                _ => XIns { op: op::ITER_VALUE_C, a: self.reg(*out), x: self.const_list(*rel, *col, val, false), ..z },
            },
            Ins::IterScan { rel, out } => XIns { op: op::ITER_SCAN, f: rel_code(*rel), a: self.reg(*out), ..z },
            Ins::IterFact { rel, col, val, out } => {
                let keyed = self.nav.facts[*rel as usize].keyed.is_some_and(|k| k.0 == *col);
                match val {
                    Val::Reg(r) if keyed => XIns { op: op::ITER_FACT_KEYED, a: self.reg(*out), b: self.reg(*r), x: u32::from(*rel), ..z },
                    Val::Reg(r) => XIns { op: op::ITER_FACT_R, a: self.reg(*out), b: self.reg(*r), c: self.fidx(*rel, *col), ..z },
                    Val::Syms(s) if s.len() == 1 => XIns { op: op::ITER_FACT_C, a: self.reg(*out), c: self.fidx(*rel, *col), y: self.k(u64::from(s[0])), ..z },
                    Val::Syms(s) => {
                        let mut l = vec![s.len() as u32];
                        for &v in s.iter() {
                            l.push(self.k(u64::from(v)));
                        }
                        XIns { op: op::ITER_FACT_L, a: self.reg(*out), c: self.fidx(*rel, *col), y: self.list(l), ..z }
                    }
                    Val::Nums(n) => {
                        let mut l = vec![n.len() as u32];
                        for &v in n.iter() {
                            l.push(self.k(v as u64));
                        }
                        XIns { op: op::ITER_FACT_L, a: self.reg(*out), c: self.fidx(*rel, *col), y: self.list(l), ..z }
                    }
                }
            }
            Ins::IterFact2 { rel, c1, v1, c2, v2, out } => {
                let at = self.x.k64.len() as u32;
                self.x.k64.push(*v1);
                self.x.k64.push(*v2);
                XIns { op: op::ITER_FACT2, a: self.reg(*out), c: self.f2(*rel, *c1, *c2), y: at, ..z }
            }
            Ins::IterFactScan { rel, out } => XIns { op: op::ITER_FACT_SCAN, a: self.reg(*out), x: u32::from(*rel), ..z },
            Ins::Axis { dir, start, out } => {
                let o = match dir {
                    Dir::Parent => op::AXIS_PARENT,
                    Dir::Ancestor => op::AXIS_ANC,
                    Dir::Child => op::AXIS_CHILD,
                    Dir::Descendant => op::AXIS_DESC,
                };
                XIns { op: o, a: self.reg(*out), b: self.reg(*start), ..z }
            }
            Ins::Within { anc, desc } => XIns { op: op::WITHIN, a: self.reg(*anc), b: self.reg(*desc), ..z },
            Ins::IterRange { rel, col, val, out, start, lo, hi } => {
                // f: bit 0 lo present, 1 lo inclusive, 2 hi present, 3 hi inclusive, 4 value from a register
                let mut f = 0u8;
                let (mut lr, mut hr) = (0u32, 0u32);
                if let Some((r, incl)) = lo {
                    f |= 1 | (u8::from(*incl) << 1);
                    lr = u32::from(self.reg(*r));
                }
                if let Some((r, incl)) = hi {
                    f |= 4 | (u8::from(*incl) << 3);
                    hr = u32::from(self.reg(*r));
                }
                let (src, tid) = match val {
                    Val::Reg(r) => {
                        f |= 16;
                        (u32::from(self.reg(*r)), self.tidx(*rel, *col))
                    }
                    _ => (self.const_list(*rel, *col, val, true), 0),
                };
                let off = self.list(vec![src, lr, hr, tid]);
                XIns { op: op::ITER_RANGE, f, a: self.reg(*out), b: self.reg(*start), x: off, ..z }
            }
            Ins::IterField { parent, field, out, field_out } => XIns { op: op::ITER_FIELD, a: self.reg(*out), b: self.reg(*parent), c: self.reg(*field_out), x: *field, ..z },
            Ins::Near { start, out, .. } => XIns { op: op::NEAR, a: self.reg(*out), b: self.reg(*start), x: self.sublist(subs), ..z },
            Ins::Sub { negate, .. } => {
                if subs.len() == 1 {
                    XIns { op: op::SUB1, f: u8::from(*negate), x: subs[0], ..z }
                } else {
                    XIns { op: op::SUB, f: u8::from(*negate), x: self.sublist(subs), ..z }
                }
            }
            Ins::Emit { cols, rule } => {
                let l: Vec<u32> = cols.iter().map(|c| self.xo(c)).collect();
                assert!(l.len() < 65536);
                XIns { op: op::EMIT, c: l.len() as u16, x: self.list(l), y: *rule, ..z }
            }
            Ins::Fork { subs: children } => {
                // per child: entry, child number (its per-file enabled answer), slots offset, slot count
                let mut l = Vec::new();
                for ((slots, _), e) in children.iter().zip(subs) {
                    let so = self.list(slots.to_vec());
                    let cid = self.site(S_FORK);
                    l.extend([*e, cid, so, slots.len() as u32]);
                }
                // not shared with another fork: the child numbers are this fork's own
                let off = self.x.pool.len() as u32;
                self.x.pool.extend_from_slice(&l);
                XIns { op: op::FORK, x: off, y: children.len() as u32, ..z }
            }
            Ins::Hoist { id, key, caps, .. } | Ins::HoistProbe { id, key, caps, .. } => {
                assert!(key.len() <= 2, "a hoist key of at most two registers");
                let n = self.hoist_at.len() as u32;
                let site = *self.hoist_at.entry(*id).or_insert(n);
                self.x.sites[S_HOIST] = self.hoist_at.len() as u32;
                let kr = self.regs(key);
                let cr = self.regs(caps);
                let capl = self.list(cr.clone());
                let mut l = vec![site, subs[0], key.len() as u32, kr.first().copied().unwrap_or(0), kr.get(1).copied().unwrap_or(0), cr.len() as u32, capl];
                if let Ins::HoistProbe { probe, .. } = i {
                    l.extend([probe.0, u32::from(self.reg(probe.1)), *id]);
                    return XIns { op: op::HOIST_PROBE, x: self.list(l), ..z };
                }
                XIns { op: op::HOIST, x: self.list(l), ..z }
            }
            Ins::Capture { regs } => {
                let l = self.regs(regs);
                XIns { op: op::CAPTURE, c: l.len() as u16, x: self.list(l), ..z }
            }
            Ins::ChildToward { parent, desc, out, field } => {
                let (f, x, y) = match field {
                    Some((fl, fo)) => (1, *fl, u32::from(self.reg(*fo))),
                    None => (0, 0, 0),
                };
                XIns { op: op::CHILD_TOWARD, f, a: self.reg(*out), b: self.reg(*parent), c: self.reg(*desc), x, y }
            }
            Ins::UnionKey { outs, key, .. } => {
                let or = self.regs(outs);
                let capl = self.list(or.clone());
                let site = self.site(S_KEY);
                let l = vec![or.len() as u32, capl, self.sublist(subs), site];
                XIns { op: op::UNION_KEY, a: self.reg(*key), x: self.list(l), ..z }
            }
            Ins::Union { outs, memo, .. } => {
                let or = self.regs(outs);
                let capl = self.list(or.clone());
                let site = self.site(S_UNION);
                let l = vec![or.len() as u32, capl, self.sublist(subs), site];
                XIns { op: op::UNION, f: u8::from(*memo), x: self.list(l), ..z }
            }
            Ins::SubFirst { .. } => XIns { op: op::SUB_FIRST, x: subs[0], ..z },
            Ins::Yield => XIns { op: op::YIELD, ..z },
            Ins::SpanBetween { a, b, out } => XIns { op: op::SPAN_BETWEEN, a: self.reg(*out), x: self.xo(a), y: self.xo(b), ..z },
            Ins::SpanBounds { s, start, end } => XIns { op: op::SPAN_BOUNDS, a: self.reg(*start), b: self.reg(*end), x: self.xo(s), ..z },
            Ins::SourceMatches { span, base, regex, group, outs, capture_text, capture_line } => {
                let or = self.regs(outs);
                let mut l = vec![self.xo(span), base.as_ref().map_or(u32::MAX, |b| self.xo(b)), *regex, group.unwrap_or(u32::MAX), or.len() as u32];
                l.extend(or);
                XIns { op: op::SOURCE_MATCHES, f: u8::from(*capture_text) | (u8::from(*capture_line) << 1), x: self.list(l), ..z }
            }
            Ins::Closure { src, out, xin, kout, nkeys, .. } => {
                let mut caps = vec![u32::from(self.reg(*out))];
                caps.extend(self.regs(kout));
                let capl = self.list(caps.clone());
                let l = vec![*nkeys as u32, caps.len() as u32, capl, self.sublist(subs)];
                XIns { op: op::CLOSURE, a: self.reg(*src), b: self.reg(*xin), x: self.list(l), ..z }
            }
            Ins::Collect { regs } => {
                let l = self.regs(regs);
                XIns { op: op::COLLECT, c: l.len() as u16, x: self.list(l), ..z }
            }
            Ins::Let { from, out } => XIns { op: op::LET, a: self.reg(*out), x: self.xo(from), ..z },
            Ins::Shared { idx } => XIns { op: op::JUMP, x: *idx, ..z },
            Ins::TextFn { lower, args, out } => {
                let mut l = vec![args.len() as u32];
                for a in args.iter() {
                    l.extend(self.tagged(a));
                }
                XIns { op: op::TEXT_FN, f: u8::from(*lower), a: self.reg(*out), x: self.list(l), ..z }
            }
            Ins::CaptureText { text, pattern, outs } => {
                let mut l = self.tagged(text).to_vec();
                l.push(self.str(pattern));
                l.push(outs.len() as u32);
                l.extend(self.regs(outs));
                XIns { op: op::CAPTURE_TEXT, x: self.list(l), ..z }
            }
            Ins::Replace { text, regex, replacement, out } => {
                let mut l = self.tagged(text).to_vec();
                l.push(*regex as u32);
                l.push(self.str(replacement));
                XIns { op: op::REPLACE, a: self.reg(*out), x: self.list(l), ..z }
            }
            Ins::NearDerived { start, out, xin, edges, .. } => {
                let ne = edges.len();
                let l = vec![self.sublist(&subs[..ne]), self.sublist(&subs[ne..])];
                XIns { op: op::NEAR_DERIVED, a: self.reg(*out), b: self.reg(*start), c: self.reg(*xin), x: self.list(l), ..z }
            }
            Ins::Sibling { next, start, out } => XIns { op: op::SIBLING, f: u8::from(*next), a: self.reg(*out), b: self.reg(*start), ..z },
            Ins::SubMemo { negate, .. } => XIns { op: op::SUB_MEMO, f: u8::from(*negate), x: self.sublist(subs), y: self.site(S_SUBMEMO), ..z },
            Ins::NearMemo { start, out, .. } => XIns { op: op::NEAR_MEMO, a: self.reg(*out), b: self.reg(*start), x: self.sublist(subs), y: self.site(S_NODE), ..z },
            Ins::SubKey { negate, key, .. } => XIns { op: op::SUB_KEY, f: u8::from(*negate), a: self.reg(*key), x: self.sublist(subs), y: self.site(S_KEY), ..z },
            Ins::AncExists { negate, start, out, .. } => XIns { op: op::ANC_EXISTS, f: u8::from(*negate), a: self.reg(*out), b: self.reg(*start), x: subs[0], y: self.site(S_NODE), ..z },
        }
    }
}

/// The last load-time pass: every program of `nav` into the compact form. Entries land in the program's own structures
/// (`Group::xe`-like fields) and `nav.x`.
pub(crate) fn encode(nav: &mut NavProgram) {
    let fuse = !std::env::var("NAV_XFUSE").is_ok_and(|v| v == "0");
    let mut e = Encoder {
        nav,
        x: Compact::default(),
        k64_at: FxMap::default(),
        list_at: FxMap::default(),
        const_at: FxMap::default(),
        fidx_at: FxMap::default(),
        tidx_at: FxMap::default(),
        f2_at: FxMap::default(),
        hoist_at: FxMap::default(),
        str_at: FxMap::default(),
        jumps: Vec::new(),
        shared_done: 0,
        max_reg: 0,
        sentinels: 0,
        fuse,
        fused: 0,
    };
    // word 0 fails: an entry of 0 is never a program
    e.x.code.push(XIns::default());
    let nav_ref: &NavProgram = e.nav;
    for q in &nav_ref.shared {
        let entry = e.prog(q);
        e.x.shared_entry.push(entry);
        e.shared_done += 1;
    }
    let mut fact_e: Vec<Vec<u32>> = Vec::new();
    let mut om_e: Vec<(Vec<u32>, Vec<u32>)> = Vec::new();
    for f in &nav_ref.facts {
        fact_e.push(f.made.as_ref().map_or(Vec::new(), |m| e.progs(&m.0)));
        om_e.push(f.made_om.as_ref().map_or((Vec::new(), Vec::new()), |om| (e.progs(&om.left), e.progs(&om.right))));
    }
    let mut aux_e: Vec<Vec<Vec<u32>>> = Vec::new();
    for r in &nav_ref.rules {
        aux_e.push(r.aux.iter().map(|(_, ps, _)| e.progs(ps)).collect());
    }
    let nrules = nav_ref.rules.len() as u32;
    for g in &nav_ref.groups {
        let entry = e.prog(&g.prog);
        e.x.group_entry.push(entry);
        let out = match (&g.kinds, g.prog.first()) {
            (Some(_), Some(Ins::IterKind { out, .. })) => u32::from(*out),
            (Some(_), _) => panic!("a kind-anchored group starts with its kind walk"),
            _ => u32::MAX,
        };
        e.x.group_out.push(out);
        let r0 = nav_ref.slot_rule[g.slots[0] as usize];
        e.x.group_rule.push(if g.slots.iter().all(|&s| nav_ref.slot_rule[s as usize] == r0) { r0 } else { nrules });
    }
    // shared sub-programs referenced before they were encoded
    for &(at, idx) in &e.jumps {
        e.x.code[at].x = e.x.shared_entry[idx as usize];
    }
    assert!(e.max_reg <= nav_ref.max_regs.max(1), "a register past the program's register count");
    let mut xcode = vec![14u8; NXOPS];
    for o in XOPS {
        xcode[o.0 as usize] = o.2 as u8;
    }
    e.x.xcode = xcode;
    e.x.on = true;
    let mut census = vec![0usize; NXOPS];
    for w in &e.x.code {
        census[w.op as usize] += 1;
    }
    let mut cs: Vec<(usize, &str)> = census.iter().enumerate().filter(|(_, n)| **n > 0).map(|(o, n)| (*n, xop_name(o))).collect();
    cs.sort_by(|a, b| b.0.cmp(&a.0));
    eprintln!(
        "{{\"compact\":{{\"instructions\":{},\"code_bytes\":{},\"pool_words\":{},\"k64\":{},\"strs\":{},\"consts\":{},\"fact_indexes\":{},\"sites\":{:?},\"sentinels\":{},\"fused\":{},\"census\":{{{}}}}}}}",
        e.x.code.len(),
        e.x.code.len() * 16,
        e.x.pool.len(),
        e.x.k64.len(),
        e.x.strs.len(),
        e.x.consts.len(),
        e.x.fidx.len(),
        e.x.sites,
        e.sentinels,
        e.fused,
        cs.iter().map(|(n, name)| format!("\"{name}\":{n}")).collect::<Vec<_>>().join(",")
    );
    let mut x = e.x;
    x.fact_e = fact_e;
    x.om_e = om_e;
    x.aux_e = aux_e;
    nav.x = x;
}

// ------------------------------------------------------------------ per-worker state (kept across files)

/// One running iteration of a program: where its elements are (a table and a cursor, never a pointer) and where it resumes.
#[derive(Clone, Copy)]
struct Frame {
    kind: u8,
    flag: u8,
    out: u16,
    pc: u32,
    cur: u32,
    end: u32,
    src: u32,
    x: u64,
    y: u64,
}

const K_CHILD: u8 = 0;
const K_RANGE: u8 = 1;
const K_FIELD: u8 = 2;
const K_CHAIN: u8 = 3;
const K_ANC: u8 = 4;
const K_RANGEW: u8 = 5;
const K_TUP: u8 = 6;
const K_IA: u8 = 7;
const K_FORK: u8 = 8;
const K_KINDS: u8 = 9;
// boundaries: frames at or above K_CALL are not iterations
const K_CALL: u8 = 32;
const K_DRIVE: u8 = 33;
// what an inline call does with its sub-program's answer
const M_SUB: u8 = 0;
const M_SUBKEY: u8 = 1;
const M_HOIST: u8 = 2;
const M_UNION: u8 = 3;
const FL_NEG: u8 = 8;
const FL_OFF: u8 = 16;
const NFRAMES: usize = 32;
const FL_VISIT: u8 = 1;
const FL_RECLAIM: u8 = 2;

/// A fact column's index for the file: a chain of rows per key (`next` is a run of the arena); the first row of a key is
/// in `head` (a run of the arena indexed by node id) when the column holds nodes, else in the worker's keyed map.
#[derive(Default, Clone, Copy)]
pub(crate) struct XFIdx {
    generation: u32,
    dense: bool,
    head: u32,
    nhead: u32,
    next: u32,
}

/// A tree column's value index for the file: ids sorted by (value, start, id), two runs of the arena.
#[derive(Default, Clone, Copy)]
pub(crate) struct XTIdx {
    generation: u32,
    vals: u32,
    ids: u32,
    len: u32,
}

/// What a worker keeps for the compact executor. Every per-file answer is stamped with the file's generation; the storage
/// of every table stays allocated across files.
#[derive(Default)]
pub(crate) struct XState {
    pub generation: u32,
    /// captured and collected tuples of the file: hoists, unions, closures (read by offset, so it may grow while read)
    pub tup: Vec<u64>,
    /// the capture stack (the running hoisted segments' captures)
    pub cap: Vec<u64>,
    /// hoists without a key: (generation, offset, length)
    hoist0: Vec<(u32, u32, u32)>,
    /// hoists per key values
    hoistk: FxMap<(u32, u64, u64), (u32, u32)>,
    /// probed hoists: the keys built, and per (site, keys, probed value) the tuples
    probe_built: FxMap<(u32, u64, u64), ()>,
    probek: FxMap<(u32, u64, u64, u64), (u32, u32)>,
    /// per hoist site over the run: (lookups, misses, steps in misses)
    pub hsite: Vec<(u64, u64, u64)>,
    /// per keyed check or union over the run
    pub ksite: Vec<(u64, u64, u64)>,
    keym: FxMap<(u32, u64), bool>,
    ukeym: FxMap<(u32, u64), (u32, u32)>,
    /// per-file checks: generation * 2 + answer
    submemo: Vec<u32>,
    /// per-file unions: (generation, offset, length)
    umemo: Vec<(u32, u32, u32)>,
    /// per-node memos: (generation, offset into `nm`)
    nm_at: Vec<(u32, u32)>,
    nm: Vec<u32>,
    pub nm_peak: usize,
    /// fork children: generation * 2 + enabled
    fork: Vec<u32>,
    /// the file's index storage: every constant list, fact index chain and tree index is a run of this arena (reset per
    /// file, storage kept; read by offset, so it may grow while read)
    ia: Vec<u32>,
    /// constant value walks: (generation, offset, length)
    clist: Vec<(u32, u32, u32)>,
    fidx: Vec<XFIdx>,
    /// fact indexes on columns that do not hold nodes: (index, key) -> first row
    fat: FxMap<(u16, u64), u32>,
    tidx: Vec<XTIdx>,
    pairs: Vec<(u64, u32)>,
    /// two-column fact indexes: generation and the chain's run; (index, value, value) -> first row
    f2: Vec<(u32, u32)>,
    f2at: FxMap<(u16, u64, u64), u32>,
    caps: Vec<Option<regex_automata::util::captures::Captures>>,
    scratch: Vec<u32>,
    /// the per-kind driver's groups and the kind's range of the kind index
    pub drive: Vec<u32>,
    pub drive_range: (u32, u32),
    /// counter build: steps and nanoseconds per opcode, fall-through pairs
    pub xsteps: Vec<u64>,
    pub xns: Vec<u64>,
    pub xpairs: Vec<u64>,
    pub last: Option<std::time::Instant>,
    pub cur_op: usize,
    pub optime: bool,
    pub tup_peak: usize,
    pub ia_peak: usize,
}

impl XState {
    /// a new file: one more generation; the keyed maps and the arenas are emptied (their storage stays)
    pub(crate) fn start_file(&mut self, x: &Compact, nregex: usize) {
        if self.xsteps.is_empty() {
            self.hoist0 = vec![(0, 0, 0); x.sites[S_HOIST] as usize];
            self.hsite = vec![(0, 0, 0); x.sites[S_HOIST] as usize];
            self.ksite = vec![(0, 0, 0); x.sites[S_KEY] as usize];
            self.submemo = vec![0; x.sites[S_SUBMEMO] as usize];
            self.umemo = vec![(0, 0, 0); x.sites[S_UNION] as usize];
            self.nm_at = vec![(0, 0); x.sites[S_NODE] as usize];
            self.fork = vec![0; x.sites[S_FORK] as usize];
            self.clist = vec![(0, 0, 0); x.consts.len()];
            self.fidx = vec![XFIdx::default(); x.fidx.len()];
            self.tidx = vec![XTIdx::default(); x.tidx.len()];
            self.f2 = vec![(0, 0); x.f2.len()];
            self.caps = (0..nregex).map(|_| None).collect();
            self.xsteps = vec![0; NXOPS];
            self.xns = vec![0; NXOPS];
            self.xpairs = vec![0; NXOPS * NXOPS];
            self.optime = std::env::var_os("NAV_OPTIME").is_some();
        }
        self.generation += 1;
        self.tup_peak = self.tup_peak.max(self.tup.len());
        self.nm_peak = self.nm_peak.max(self.nm.len());
        self.ia_peak = self.ia_peak.max(self.ia.len());
        self.tup.clear();
        self.cap.clear();
        self.nm.clear();
        self.ia.clear();
        self.fat.clear();
        self.f2at.clear();
        self.hoistk.clear();
        self.probe_built.clear();
        self.probek.clear();
        self.keym.clear();
        self.ukeym.clear();
    }

    /// bytes held (capacities), by structure
    pub(crate) fn bytes(&self) -> Vec<(&'static str, usize)> {
        vec![
            ("tuples", self.tup.capacity() * 8),
            ("captures", self.cap.capacity() * 8),
            ("node_memo_slab", self.nm.capacity() * 4),
            ("keyed_maps", self.hoistk.capacity() * 40 + self.keym.capacity() * 24 + self.ukeym.capacity() * 32 + self.probek.capacity() * 48 + self.probe_built.capacity() * 32),
            (
                "site_arrays",
                self.hoist0.len() * 12 + self.hsite.len() * 24 + self.ksite.len() * 24 + self.submemo.len() * 4 + self.umemo.len() * 12 + self.nm_at.len() * 8 + self.fork.len() * 4,
            ),
            ("index_arena", self.ia.capacity() * 4 + self.pairs.capacity() * 16),
            ("index_maps", self.fat.capacity() * 24 + self.f2at.capacity() * 32),
            ("index_tables", self.clist.len() * 12 + self.fidx.len() * 20 + self.tidx.len() * 16 + self.f2.len() * 8),
        ]
    }
}

#[derive(Clone, Copy)]
enum XO<'x> {
    Reg(R),
    Num(i64),
    Sym(u32),
    Syms(&'x [u32]),
    Nums(&'x [u64]),
    File,
}

fn tagged(x: &Compact, at: usize) -> XO<'_> {
    let (tag, v) = (x.pool[at], x.pool[at + 1]);
    match tag {
        TG_REG => XO::Reg(v as R),
        TG_NUM => XO::Num(x.k64[v as usize] as i64),
        TG_SYM => XO::Sym(v),
        TG_SYMS => {
            let n = x.pool[v as usize] as usize;
            XO::Syms(&x.pool[v as usize + 1..v as usize + 1 + n])
        }
        TG_NUMS => {
            let (n, at) = (x.pool[v as usize] as usize, x.pool[v as usize + 1] as usize);
            XO::Nums(&x.k64[at..at + n])
        }
        _ => XO::File,
    }
}

fn span(s: u32, e: u32) -> u64 {
    (u64::from(s) << 32) | u64::from(e)
}

#[inline(always)]
fn inset(set: &[u32], val: u64) -> bool {
    val <= u64::from(u32::MAX) && if set.len() <= 8 { set.contains(&(val as u32)) } else { set.binary_search(&(val as u32)).is_ok() }
}

impl<'p> Exec<'_, 'p> {
    #[inline]
    fn xv(&self, w: u32) -> u64 {
        if w & XK == 0 { self.regs[w as usize] } else { self.prog.x.k64[(w & !XK) as usize] }
    }

    fn xoval(&self, o: XO<'_>) -> u64 {
        match o {
            XO::Reg(r) => self.regs[r as usize],
            XO::Num(n) => n as u64,
            XO::Sym(s) => u64::from(s),
            _ => 0,
        }
    }

    fn xtext(&self, o: XO<'_>, v: u64) -> Option<&str> {
        if matches!(o, XO::File) {
            return Some(self.f.path);
        }
        self.sym.str(v as u32)
    }

    /// the generic test (nav.rs `test` over the encoded operands)
    #[inline(never)]
    fn xtest(&self, i: &XIns) -> bool {
        let x = &self.prog.x;
        let at = i.x as usize;
        let (l, r) = (tagged(x, at), tagged(x, at + 2));
        let nre = x.pool[at + 4] as usize;
        let regex = &x.pool[at + 5..at + 5 + nre];
        let test = TESTS[(i.f & 15) as usize];
        let (text, num) = (i.f & 16 != 0, i.f & 32 != 0);
        let lv = self.xoval(l);
        match test {
            T::Eq | T::Ne | T::Lt | T::Le | T::Gt | T::Ge => {
                let rv = self.xoval(r);
                if test == T::Eq {
                    return lv == rv;
                }
                if test == T::Ne {
                    return lv != rv;
                }
                let ord = if text {
                    if lv == rv {
                        std::cmp::Ordering::Equal
                    } else {
                        match (self.sym.str(lv as u32), self.sym.str(rv as u32)) {
                            (Some(a), Some(b)) => a.cmp(b),
                            _ => lv.cmp(&rv),
                        }
                    }
                } else if num {
                    (lv as i64).cmp(&(rv as i64))
                } else {
                    lv.cmp(&rv)
                };
                match test {
                    T::Lt => ord.is_lt(),
                    T::Le => ord.is_le(),
                    T::Gt => ord.is_gt(),
                    _ => ord.is_ge(),
                }
            }
            T::In | T::NotIn => {
                let inside = match r {
                    XO::Syms(s) => s.binary_search(&(lv as u32)).is_ok(),
                    XO::Nums(n) => n.binary_search_by(|x| (*x as i64).cmp(&(lv as i64))).is_ok(),
                    _ => lv == self.xoval(r),
                };
                inside == (test == T::In)
            }
            T::Matches | T::Glob | T::NotGlob => {
                let hit = self.xtext(l, lv).is_some_and(|t| regex.iter().any(|&k| self.prog.regexes[k as usize].is_match(t)));
                if test == T::NotGlob { !hit } else { hit }
            }
            T::NumGe | T::NatLt | T::Succ => {
                let rv = self.xoval(r);
                let kind = |o: XO<'_>| -> bool { matches!(o, XO::Num(_)) || (matches!(o, XO::Reg(_)) && num && !text) };
                if test == T::NatLt {
                    return match (self.xtext(l, lv), self.xtext(r, rv)) {
                        (Some(a), Some(b)) => crate::nav::natural_compare(a, b).is_lt(),
                        _ => false,
                    };
                }
                let read = |o: XO<'_>, v: u64| -> Option<crate::nav::Numeric> {
                    if kind(o) {
                        return Some(crate::nav::Numeric::Int(i128::from(v as i64)));
                    }
                    crate::nav::read_numeric(self.xtext(o, v)?)
                };
                match (read(l, lv), read(r, rv)) {
                    (Some(crate::nav::Numeric::Int(a)), Some(crate::nav::Numeric::Int(b))) if test == T::Succ => b == a + 1,
                    (Some(_), Some(_)) if test == T::Succ => false,
                    (Some(a), Some(b)) => crate::nav::numeric_cmp(a, b).is_some_and(|o| o.is_ge()),
                    _ => false,
                }
            }
            T::Starts | T::Ends => {
                let rv = self.xoval(r);
                match (self.xtext(l, lv), self.xtext(r, rv)) {
                    (Some(a), Some(b)) => {
                        if test == T::Starts { a.starts_with(b) } else { a.ends_with(b) }
                    }
                    _ => false,
                }
            }
        }
    }

    /// Runs the program at `entry` (the executor the worker's counters ask for).
    #[inline]
    pub(crate) fn xcall(&mut self, entry: u32) -> bool {
        if self.c.diag { self.xrun::<true>(entry) } else { self.xrun::<false>(entry) }
    }

    /// A top-level run (a group's program): its steps and visits are charged to the running rule.
    pub(crate) fn xtop(&mut self, entry: u32) {
        let (s0, v0) = (self.c.steps, self.c.visits);
        self.xcall(entry);
        let r = self.cur_rule;
        self.c.rule_steps[r] = self.c.rule_steps[r].wrapping_add(self.c.steps - s0);
        self.c.rule_visits[r] = self.c.rule_visits[r].wrapping_add(self.c.visits - v0);
    }

    /// Runs every group anchored on one kind over the kind's candidates (nodes outer, groups inner) in one call of the
    /// loop. No per-rule accounting: the counter build drives per candidate through `xtop`.
    pub(crate) fn xdrive(&mut self, kind: u32, groups: &[(u32, u32, u32)]) {
        let k = kind as usize;
        if k + 1 >= self.cache.kind_off.len() {
            return;
        }
        let xs = &mut self.cache.x;
        xs.drive.clear();
        xs.drive.extend(groups.iter().map(|g| g.0));
        xs.drive_range = (self.cache.kind_off[k], self.cache.kind_off[k + 1]);
        self.xrun::<false>(0);
    }

    /// the ids of a constant value walk (built once per file into the index arena): (offset, length)
    #[inline(never)]
    fn xconst(&mut self, id: u32) -> (u32, u32) {
        let generation = self.cache.x.generation;
        let e = self.cache.x.clist[id as usize];
        if e.0 == generation {
            return (e.1, e.2);
        }
        let t0 = std::time::Instant::now();
        self.c.const_misses += 1;
        let prog = self.prog;
        let (relc, col, sorted, off) = prog.x.consts[id as usize];
        let rel = RELS[relc as usize];
        let n = prog.x.pool[off as usize] as usize;
        let wanted = &prog.x.pool[off as usize + 1..off as usize + 1 + n];
        let v = self.f.view;
        let at = self.cache.x.ia.len();
        if rel == TRel::Id && col == 1 && self.cache.ident.is_none() {
            // identifier text against a few constants: compare the parser's own strings, no interning of the file's names
            let texts: Vec<&str> = wanted.iter().filter_map(|&w| self.sym.str(w)).collect();
            for (i, node) in self.f.table.node.iter().enumerate() {
                let t = match node {
                    ruff_python_ast::AnyNodeRef::Identifier(x) => x.as_str(),
                    ruff_python_ast::AnyNodeRef::ExprName(x) => x.id.as_str(),
                    _ => continue,
                };
                if texts.contains(&t) {
                    self.cache.x.ia.push(i as u32);
                }
            }
            self.cache.x.ia[at..].sort_by_key(|&id| (v.start[id as usize], id));
        } else {
            let csr = self.col_csr(rel, col);
            for &w in wanted {
                self.cache.x.ia.extend_from_slice(csr.bucket(w));
            }
            if sorted {
                self.cache.x.ia[at..].sort_by_key(|&id| (v.start[id as usize], id));
            }
            self.c.made_rows += (self.cache.x.ia.len() - at) as u64;
        }
        self.c.made_tables += 1;
        let ns = t0.elapsed().as_nanos() as u64;
        self.c.index_ns += ns;
        self.c.charge_index(self.cur_rule, ns);
        self.c.index_split[1] += ns;
        let len = (self.cache.x.ia.len() - at) as u32;
        self.cache.x.clist[id as usize] = (generation, at as u32, len);
        (at as u32, len)
    }

    /// a fact column's index for this file (the fact is loaded first, as nav.rs `fact_index` does): one pass over the
    /// column, rows chained per key in row order
    #[inline(never)]
    fn xfact_index(&mut self, id: u16) {
        let prog = self.prog;
        let (rel, col) = prog.x.fidx[id as usize];
        self.ensure(rel);
        let generation = self.cache.x.generation;
        if self.cache.x.fidx[id as usize].generation == generation {
            return;
        }
        let t0 = std::time::Instant::now();
        let n_view = self.f.view.len();
        let dense = matches!(prog.facts[rel as usize].kinds[col as usize], ryo_relations::domain::relations::types::CellKind::Node);
        let cache = &mut *self.cache;
        let c = &cache.facts.as_ref().unwrap().cols[rel as usize][col as usize];
        let xs = &mut cache.x;
        let mut ix = XFIdx { generation, dense, head: 0, nhead: 0, next: 0 };
        if dense {
            ix.head = xs.ia.len() as u32;
            ix.nhead = n_view as u32 + 1;
            xs.ia.resize(xs.ia.len() + n_view + 1, u32::MAX);
        }
        ix.next = xs.ia.len() as u32;
        xs.ia.resize(xs.ia.len() + c.len(), u32::MAX);
        assert!(xs.ia.len() < u32::MAX as usize);
        for (row, &v) in c.iter().enumerate().rev() {
            if dense {
                let k = ix.head as usize + (v as usize).min(n_view);
                xs.ia[ix.next as usize + row] = xs.ia[k];
                xs.ia[k] = row as u32;
            } else {
                let e = xs.fat.entry((id, v)).or_insert(u32::MAX);
                xs.ia[ix.next as usize + row] = *e;
                *e = row as u32;
            }
        }
        xs.fidx[id as usize] = ix;
        self.c.made_tables += 1;
        self.c.made_rows += c.len() as u64;
        let ns = t0.elapsed().as_nanos() as u64;
        self.c.index_ns += ns;
        self.c.charge_index(self.cur_rule, ns);
        self.c.index_split[2] += ns;
    }

    /// the first row of a key in a fact column's index (u32::MAX: none) and the offset of the index's chain
    #[inline(always)]
    fn xfact_first(&self, id: u16, key: u64) -> (u32, u32) {
        let xs = &self.cache.x;
        let ix = xs.fidx[id as usize];
        let first = if ix.dense {
            if key as usize >= ix.nhead as usize - 1 { u32::MAX } else { xs.ia[ix.head as usize + key as usize] }
        } else {
            xs.fat.get(&(id, key)).copied().unwrap_or(u32::MAX)
        };
        (first, ix.next)
    }

    /// a fact's rows by two columns' values for this file: the first row of a pair in the keyed map, rows chained
    #[inline(never)]
    fn xfact_index2(&mut self, id: u16) {
        let prog = self.prog;
        let (rel, c1, c2) = prog.x.f2[id as usize];
        self.ensure(rel);
        let generation = self.cache.x.generation;
        if self.cache.x.f2[id as usize].0 == generation {
            return;
        }
        let t0 = std::time::Instant::now();
        let cache = &mut *self.cache;
        let facts = cache.facts.as_ref().unwrap();
        let (a, b) = (&facts.cols[rel as usize][c1 as usize], &facts.cols[rel as usize][c2 as usize]);
        let xs = &mut cache.x;
        let next = xs.ia.len();
        let n = a.len().min(b.len());
        xs.ia.resize(next + n, u32::MAX);
        for row in (0..n).rev() {
            let e = xs.f2at.entry((id, a[row], b[row])).or_insert(u32::MAX);
            xs.ia[next + row] = *e;
            *e = row as u32;
        }
        xs.f2[id as usize] = (generation, next as u32);
        self.c.made_tables += 1;
        self.c.made_rows += a.len() as u64;
        let ns = t0.elapsed().as_nanos() as u64;
        self.c.index_ns += ns;
        self.c.charge_index(self.cur_rule, ns);
        self.c.index_split[2] += ns;
    }

    /// a tree column's value index for this file; the run of `key`'s ids in the index arena (offsets)
    #[inline(never)]
    fn xtree_range(&mut self, id: u32, key: u32) -> (u32, u32) {
        let generation = self.cache.x.generation;
        if self.cache.x.tidx[id as usize].generation != generation {
            let t0 = std::time::Instant::now();
            let (relc, col) = self.prog.x.tidx[id as usize];
            let rel = RELS[relc as usize];
            let n = if rel == TRel::Id { self.f.view.n_nodes } else { self.f.view.len() };
            let mut pairs = std::mem::take(&mut self.cache.x.pairs);
            pairs.clear();
            for nid in 0..n as u32 {
                if let Some(v) = self.tread(rel, col, nid) {
                    pairs.push(((u64::from(v as u32) << 32) | u64::from(self.f.view.start[nid as usize]), nid));
                }
            }
            pairs.sort_unstable();
            let xs = &mut self.cache.x;
            let vals = xs.ia.len() as u32;
            xs.ia.extend(pairs.iter().map(|p| (p.0 >> 32) as u32));
            let ids = xs.ia.len() as u32;
            xs.ia.extend(pairs.iter().map(|p| p.1));
            xs.tidx[id as usize] = XTIdx { generation, vals, ids, len: pairs.len() as u32 };
            xs.pairs = pairs;
            self.c.made_tables += 1;
            self.c.made_rows += n as u64;
            let ns = t0.elapsed().as_nanos() as u64;
            self.c.index_ns += ns;
            self.c.charge_index(self.cur_rule, ns);
            self.c.index_split[0] += ns;
        }
        let xs = &self.cache.x;
        let ix = xs.tidx[id as usize];
        let vals = &xs.ia[ix.vals as usize..(ix.vals + ix.len) as usize];
        let a = vals.partition_point(|&x| x < key);
        let b = a + vals[a..].partition_point(|&x| x == key);
        (ix.ids + a as u32, ix.ids + b as u32)
    }

    /// the slab offset of a per-node memo site in this file (its cells start unknown)
    fn xnode_memo(&mut self, site: u32) -> usize {
        const UNKNOWN: u32 = u32::MAX - 1;
        let xs = &mut self.cache.x;
        let e = xs.nm_at[site as usize];
        if e.0 == xs.generation {
            return e.1 as usize;
        }
        let n = self.f.view.len();
        let off = xs.nm.len();
        xs.nm.resize(off + n, UNKNOWN);
        xs.nm_at[site as usize] = (xs.generation, off as u32);
        self.c.memo_allocs += 1;
        self.c.memo_cells += n as u64;
        off
    }

    fn xany<const D: bool>(&mut self, list: u32) -> bool {
        let prog = self.prog;
        let n = prog.x.pool[list as usize] as usize;
        for j in 0..n {
            self.c.subs += 1;
            if self.xrun::<D>(prog.x.pool[list as usize + 1 + j]) {
                return true;
            }
        }
        false
    }

    /// nav.rs `near_memo` over the slab
    #[inline(never)]
    fn xnear_memo<const D: bool>(&mut self, s: u32, out: R, subs: u32, site: u32) -> Option<u32> {
        const UNKNOWN: u32 = u32::MAX - 1;
        const NONE: u32 = u32::MAX;
        let v = self.f.view;
        if s as usize >= v.len() {
            return None;
        }
        let base = self.xnode_memo(site);
        let sb = self.cache.x.scratch.len();
        let mut x = s;
        let res = loop {
            let m = self.cache.x.nm[base + x as usize];
            if m != UNKNOWN {
                break m;
            }
            let p = v.parent[x as usize];
            if p == u32::MAX {
                self.cache.x.nm[base + x as usize] = NONE;
                break NONE;
            }
            self.regs[out as usize] = u64::from(p);
            self.c.visits += 1;
            if self.xany::<D>(subs) {
                self.cache.x.nm[base + x as usize] = p;
                break p;
            }
            self.cache.x.scratch.push(x);
            x = p;
        };
        let xs = &mut self.cache.x;
        for k in sb..xs.scratch.len() {
            let y = xs.scratch[k];
            xs.nm[base + y as usize] = res;
        }
        xs.scratch.truncate(sb);
        (res != NONE).then_some(res)
    }

    /// nav.rs `anc_exists` over the slab
    #[inline(never)]
    fn xanc_exists<const D: bool>(&mut self, s: u32, out: R, cond: u32, site: u32) -> bool {
        const UNKNOWN: u32 = u32::MAX - 1;
        let v = self.f.view;
        if s as usize >= v.len() {
            return false;
        }
        let base = self.xnode_memo(site);
        let sb = self.cache.x.scratch.len();
        let mut x = s;
        let res = loop {
            let m = self.cache.x.nm[base + x as usize];
            if m != UNKNOWN {
                break m;
            }
            let p = v.parent[x as usize];
            if p == u32::MAX {
                self.cache.x.nm[base + x as usize] = 0;
                break 0;
            }
            self.regs[out as usize] = u64::from(p);
            self.c.visits += 1;
            self.c.subs += 1;
            if self.xrun::<D>(cond) {
                self.cache.x.nm[base + x as usize] = 1;
                break 1;
            }
            self.cache.x.scratch.push(x);
            x = p;
        };
        let xs = &mut self.cache.x;
        for k in sb..xs.scratch.len() {
            let y = xs.scratch[k];
            xs.nm[base + y as usize] = res;
        }
        xs.scratch.truncate(sb);
        res == 1
    }

    /// the distinct rows (width `w`) collected since `base`, appended to the tuple arena in first-seen order: (offset, length)
    fn xdistinct(&mut self, base: usize, w: usize) -> (u32, u32) {
        let w = w.max(1);
        let rows = &self.collect[base..];
        let tup = &mut self.cache.x.tup;
        let off = tup.len();
        if rows.len() <= w * 8 {
            for row in rows.chunks(w) {
                if !tup[off..].chunks(w).any(|t| t == row) {
                    tup.extend_from_slice(row);
                }
            }
        } else {
            let mut seen: std::collections::HashSet<&[u64], std::hash::BuildHasherDefault<crate::sema::Fx>> = Default::default();
            for row in rows.chunks(w) {
                if seen.insert(row) {
                    tup.extend_from_slice(row);
                }
            }
        }
        let len = tup.len() - off;
        assert!(tup.len() < u32::MAX as usize);
        self.collect.truncate(base);
        (off as u32, len as u32)
    }

    /// Ins::Union: the distinct tuples of its sub-programs (memoised per file when it reads no bound register)
    #[inline(never)]
    fn xunion<const D: bool>(&mut self, at: u32, memo: bool) -> (u32, u32, bool) {
        let prog = self.prog;
        let p = &prog.x.pool[at as usize..];
        let (w, subs, site) = (p[0] as usize, p[2], p[3] as usize);
        if memo {
            let e = self.cache.x.umemo[site];
            if e.0 == self.cache.x.generation {
                return (e.1, e.2, false);
            }
        }
        let base = self.collect.len();
        let n = prog.x.pool[subs as usize] as usize;
        for j in 0..n {
            self.c.subs += 1;
            self.xrun::<D>(prog.x.pool[subs as usize + 1 + j]);
        }
        let (off, len) = self.xdistinct(base, w);
        if memo {
            self.cache.x.umemo[site] = (self.cache.x.generation, off, len);
        }
        (off, len, !memo)
    }

    /// Ins::UnionKey: the union's distinct rows per file per value of its key register
    #[inline(never)]
    fn xunion_key<const D: bool>(&mut self, at: u32, k: u64) -> (u32, u32, bool) {
        let prog = self.prog;
        let p = &prog.x.pool[at as usize..];
        let (w, subs, site) = (p[0] as usize, p[2], p[3]);
        let st = &mut self.cache.x.ksite[site as usize];
        st.0 += 1;
        let off = st.0 >= 256 && st.1 * 10 >= st.0 * 9 && st.2 < 32 * st.1;
        if !off {
            if let Some(&(o, l)) = self.cache.x.ukeym.get(&(site, k)) {
                return (o, l, false);
            }
        }
        let s0 = self.c.steps;
        let base = self.collect.len();
        let n = prog.x.pool[subs as usize] as usize;
        for j in 0..n {
            self.c.subs += 1;
            self.xrun::<D>(prog.x.pool[subs as usize + 1 + j]);
        }
        let (o, l) = self.xdistinct(base, w);
        let st = &mut self.cache.x.ksite[site as usize];
        st.1 += 1;
        st.2 += self.c.steps - s0;
        if !off {
            self.cache.x.ukeym.insert((site, k), (o, l));
        }
        (o, l, off)
    }

    /// Ins::Closure: breadth-first from the start over the edge sub-programs; the reached rows go to the tuple arena
    #[inline(never)]
    fn xclosure<const D: bool>(&mut self, i: &XIns) -> (u32, u32) {
        let prog = self.prog;
        let p = &prog.x.pool[i.x as usize..];
        let (nkeys, subs) = (p[0] as usize, p[3]);
        let ns = prog.x.pool[subs as usize] as usize;
        let entries = &prog.x.pool[subs as usize + 1..subs as usize + 1 + ns];
        let xin = i.b as usize;
        let start = self.regs[i.a as usize];
        let saved_x = self.regs[xin];
        let base = self.collect.len();
        let mut found: Vec<u64> = Vec::new();
        if nkeys == 0 {
            let mut seen: std::collections::HashSet<u64, std::hash::BuildHasherDefault<crate::sema::Fx>> = Default::default();
            let mut stack: Vec<u64> = vec![start];
            while let Some(n) = stack.pop() {
                self.regs[xin] = n;
                self.collect.truncate(base);
                for &q in entries {
                    self.c.subs += 1;
                    self.xrun::<D>(q);
                }
                for k in base..self.collect.len() {
                    let y = self.collect[k];
                    if seen.insert(y) {
                        self.c.visits += 1;
                        found.push(y);
                        stack.push(y);
                    }
                }
            }
        } else {
            let w = 1 + nkeys;
            let mut seen: std::collections::HashSet<Vec<u64>, std::hash::BuildHasherDefault<crate::sema::Fx>> = Default::default();
            let mut stack: Vec<(u64, Vec<u64>)> = vec![(start, Vec::new())];
            let mut first = true;
            while let Some((n, keys)) = stack.pop() {
                self.regs[xin] = n;
                self.collect.truncate(base);
                for &q in entries {
                    self.c.subs += 1;
                    self.xrun::<D>(q);
                }
                let rows: Vec<u64> = self.collect[base..].to_vec();
                for row in rows.chunks(w) {
                    if !first && row[1..] != keys[..] {
                        continue;
                    }
                    let mut k = row[1..].to_vec();
                    k.push(row[0]);
                    if seen.insert(k) {
                        self.c.visits += 1;
                        found.extend_from_slice(row);
                        stack.push((row[0], row[1..].to_vec()));
                    }
                }
                first = false;
            }
        }
        self.collect.truncate(base);
        self.regs[xin] = saved_x;
        let tup = &mut self.cache.x.tup;
        let off = tup.len();
        tup.extend_from_slice(&found);
        (off as u32, found.len() as u32)
    }

    /// Ins::NearDerived
    #[inline(never)]
    fn xnear_derived<const D: bool>(&mut self, i: &XIns) -> Option<u64> {
        let prog = self.prog;
        let (edges, domain) = (prog.x.pool[i.x as usize], prog.x.pool[i.x as usize + 1]);
        let ne = prog.x.pool[edges as usize] as usize;
        let edges = &prog.x.pool[edges as usize + 1..edges as usize + 1 + ne];
        let mut seen: std::collections::HashSet<u64, std::hash::BuildHasherDefault<crate::sema::Fx>> = Default::default();
        let mut level = vec![self.regs[i.b as usize]];
        let base = self.collect.len();
        let mut answer = None;
        'up: while !level.is_empty() {
            let mut next = Vec::new();
            for &n in &level {
                self.regs[i.c as usize] = n;
                self.collect.truncate(base);
                for &q in edges {
                    self.c.subs += 1;
                    self.xrun::<D>(q);
                }
                for &y in &self.collect[base..] {
                    if seen.insert(y) {
                        next.push(y);
                    }
                }
            }
            for &y in &next {
                self.regs[i.a as usize] = y;
                self.c.visits += 1;
                if self.xany::<D>(domain) {
                    answer = Some(y);
                    break 'up;
                }
            }
            level = next;
        }
        self.collect.truncate(base);
        answer
    }

    /// Ins::SourceMatches: every match of the regex inside the span continues the program
    #[inline(never)]
    fn xsource_matches<const D: bool>(&mut self, i: &XIns, pc: u32) -> bool {
        let prog = self.prog;
        let p = &prog.x.pool[i.x as usize..];
        let sp = self.xv(p[0]);
        let base_line = (p[1] != u32::MAX).then(|| self.xv(p[1]));
        let regex = p[2] as usize;
        let group = (p[3] != u32::MAX).then_some(p[3]);
        let outs = &p[5..5 + p[4] as usize];
        let (capture_text, capture_line) = (i.f & 1 != 0, i.f & 2 != 0);
        let f = self.f;
        let bytes = f.source.as_bytes();
        let (s, e) = ((sp >> 32) as usize, (sp & 0xffff_ffff) as usize);
        if e > bytes.len() || s > e {
            return false;
        }
        let re = &prog.regexes[regex];
        let mut caps = self.cache.x.caps[regex].take().unwrap_or_else(|| re.create_captures());
        let mut at = 0usize;
        let hay = &bytes[s..e];
        let line_at = |off: usize| -> u64 { base_line.unwrap_or(0) + bytes[s..off].iter().filter(|&&b| b == b'\n').count() as u64 };
        let mut flow = false;
        while at <= hay.len() {
            re.search_captures(&regex_automata::Input::new(hay).range(at..), &mut caps);
            let Some(m) = caps.get_match() else { break };
            let (ms, me) = (s + m.start(), s + m.end());
            at = if m.end() > m.start() { m.end() } else { m.end() + 1 };
            let mut k = 0;
            self.regs[outs[k] as usize] = span(ms as u32, me as u32);
            k += 1;
            if base_line.is_some() {
                self.regs[outs[k] as usize] = line_at(ms);
                k += 1;
            }
            if let Some(g) = group {
                let Some(cg) = caps.get_group(g as usize) else { continue };
                let (cs, ce) = (s + cg.start, s + cg.end);
                self.regs[outs[k] as usize] = span(cs as u32, ce as u32);
                k += 1;
                if capture_text {
                    let t = String::from_utf8_lossy(&bytes[cs..ce]).to_string();
                    self.regs[outs[k] as usize] = u64::from(self.sym.minted(t));
                    k += 1;
                }
                if capture_line {
                    self.regs[outs[k] as usize] = line_at(cs);
                }
            }
            self.c.visits += 1;
            if self.xrun::<D>(pc + 1) {
                flow = true;
                break;
            }
        }
        self.cache.x.caps[regex] = Some(caps);
        flow
    }

    /// Ins::CaptureText
    #[inline(never)]
    fn xcapture_text<const D: bool>(&mut self, i: &XIns, pc: u32) -> bool {
        let prog = self.prog;
        let at = i.x as usize;
        let text = tagged(&prog.x, at);
        let pattern = prog.x.strs[prog.x.pool[at + 2] as usize].as_str();
        let n = prog.x.pool[at + 3] as usize;
        let outs = &prog.x.pool[at + 4..at + 4 + n];
        let v = self.xoval(text);
        let Some(t) = self.xtext(text, v).map(str::to_owned) else { return false };
        let mut found: Vec<Vec<std::ops::Range<usize>>> = Vec::new();
        crate::nav::capture_matches(pattern, &t, &t, &mut Vec::new(), &mut found);
        for spans in found {
            for (k, r) in spans.into_iter().enumerate() {
                let id = self.sym.minted(t[r].to_owned());
                self.regs[outs[k] as usize] = u64::from(id);
            }
            self.c.visits += 1;
            if self.xrun::<D>(pc + 1) {
                return true;
            }
        }
        false
    }

    /// a hoisted segment's captured tuples for this execution: (offset, length, not kept by a memo)
    #[inline(never)]
    fn xhoist<const D: bool>(&mut self, at: usize) -> (u32, u32, bool) {
        let pool = &self.prog.x.pool;
        let (site, seg, nkey) = (pool[at], pool[at + 1], pool[at + 2]);
        let k0 = if nkey > 0 { self.regs[pool[at + 3] as usize] } else { 0 };
        let k1 = if nkey > 1 { self.regs[pool[at + 4] as usize] } else { 0 };
        let generation = self.cache.x.generation;
        let st = &mut self.cache.x.hsite[site as usize];
        st.0 += 1;
        let off = st.0 >= 256 && st.1 * 10 >= st.0 * 9 && st.2 < 32 * st.1;
        let cached = if off {
            None
        } else if nkey == 0 {
            let e = self.cache.x.hoist0[site as usize];
            (e.0 == generation).then_some((e.1, e.2))
        } else {
            self.cache.x.hoistk.get(&(site, k0, k1)).copied()
        };
        let got = match cached {
            Some((o, l)) => (o, l, false),
            None => {
                let s0 = self.c.steps;
                let base = self.cache.x.cap.len();
                self.xrun::<D>(seg);
                let xs = &mut self.cache.x;
                let o = xs.tup.len();
                xs.tup.extend_from_slice(&xs.cap[base..]);
                xs.cap.truncate(base);
                let l = xs.tup.len() - o;
                assert!(xs.tup.len() < u32::MAX as usize);
                self.c.hoist_misses += 1;
                let st = &mut xs.hsite[site as usize];
                st.1 += 1;
                st.2 += self.c.steps - s0;
                if !off {
                    if nkey == 0 {
                        xs.hoist0[site as usize] = (generation, o as u32, l as u32);
                    } else {
                        xs.hoistk.insert((site, k0, k1), (o as u32, l as u32));
                    }
                }
                (o as u32, l as u32, off)
            }
        };
        self.c.hoist_runs += 1;
        got
    }

    /// a probed hoist's tuples whose probed column equals the probing register: (offset, length) in the tuple arena
    #[inline(never)]
    fn xhoist_probe<const D: bool>(&mut self, at: usize) -> Option<(u32, u32)> {
        let pool = &self.prog.x.pool;
        let (site, seg, nkey) = (pool[at], pool[at + 1], pool[at + 2]);
        let k0 = if nkey > 0 { self.regs[pool[at + 3] as usize] } else { 0 };
        let k1 = if nkey > 1 { self.regs[pool[at + 4] as usize] } else { 0 };
        let w = pool[at + 5].max(1) as usize;
        let (pidx, preg) = (pool[at + 7] as usize, pool[at + 8]);
        if !self.cache.x.probe_built.contains_key(&(site, k0, k1)) {
            let base = self.cache.x.cap.len();
            self.xrun::<D>(seg);
            self.c.hoist_misses += 1;
            let xs = &mut self.cache.x;
            // group by the probed value, each group in capture order
            let mut m: FxMap<u64, Vec<u64>> = FxMap::default();
            for tup in xs.cap[base..].chunks(w) {
                m.entry(tup[pidx]).or_default().extend_from_slice(tup);
            }
            xs.cap.truncate(base);
            for (pv, rows) in m {
                let o = xs.tup.len();
                xs.tup.extend_from_slice(&rows);
                xs.probek.insert((site, k0, k1, pv), (o as u32, rows.len() as u32));
            }
            xs.probe_built.insert((site, k0, k1), ());
        }
        self.c.hoist_runs += 1;
        let want = self.regs[preg as usize];
        self.cache.x.probek.get(&(site, k0, k1, want)).copied()
    }

    /// The dispatch loop. Returns true when the program reached Yield (the caller stops enumerating).
    #[allow(clippy::too_many_lines)]
    pub(crate) fn xrun<const D: bool>(&mut self, entry: u32) -> bool {
        let prog: &'p NavProgram = self.prog;
        let x: &'p Compact = &prog.x;
        let code: &'p [XIns] = &x.code;
        let pool: &'p [u32] = &x.pool;
        let k64: &'p [u64] = &x.k64;
        let v = self.f.view;
        let n = v.kind.len();
        let vc: [&[u32]; 7] = [&v.kind, &v.text, &v.start, &v.stop, &v.parent, &v.ordinal, &v.field];
        let par: &[u32] = &v.parent;
        let fld: &[u32] = &v.field;
        let child_off: &[u32] = &v.child_off;
        let children: &[u32] = &v.children;
        let mut fr: arrayvec::ArrayVec<Frame, NFRAMES> = arrayvec::ArrayVec::new();
        let mut steps = 0u64;
        let mut visits = 0u64;
        let mut pc = entry as usize;
        let mut prev = 0usize;
        macro_rules! r {
            ($i:expr) => {
                self.regs[$i as usize]
            };
        }
        macro_rules! xv {
            ($w:expr) => {{
                let w = $w;
                if w & XK == 0 { self.regs[w as usize] } else { k64[(w & !XK) as usize] }
            }};
        }
        macro_rules! sync {
            () => {
                self.c.steps += steps;
                steps = 0;
                self.c.visits += visits;
                visits = 0;
            };
        }
        // an iteration needs a frame: past the local frames, the rest of the program (from this instruction) runs as a
        // nested call; first statement of every arm that pushes a frame, before any work
        macro_rules! need_frame {
            ($l:lifetime, $i:expr) => {
                if fr.is_full() {
                    steps -= 1;
                    if D {
                        self.c.op_steps[x.xcode[$i.op as usize] as usize] -= 1;
                        self.cache.x.xsteps[$i.op as usize] -= 1;
                    }
                    sync!();
                    let y = self.xrun::<D>(pc as u32);
                    break $l y;
                }
            };
        }
        macro_rules! frame {
            ($kind:expr, $out:expr, $cur:expr, $end:expr, $src:expr, $x:expr, $flag:expr) => {
                fr.push(Frame { kind: $kind, flag: $flag, out: $out, pc: pc as u32, cur: $cur, end: $end, src: $src, x: $x, y: 0 })
            };
        }
        'run: loop {
            let mut yielded: bool = 'fail: {
                let i: &XIns = &code[pc];
                steps += 1;
                if D {
                    let c = &mut *self.c;
                    if c.op_steps.len() < crate::nav::NINS {
                        c.op_steps.resize(crate::nav::NINS, 0);
                    }
                    c.op_steps[x.xcode[i.op as usize] as usize] += 1;
                    let xs = &mut self.cache.x;
                    xs.xsteps[i.op as usize] += 1;
                    xs.xpairs[prev * NXOPS + i.op as usize] += 1;
                    prev = i.op as usize;
                    if xs.optime {
                        // the time since the last dispatch goes to the instruction dispatched then
                        let t = std::time::Instant::now();
                        if let Some(l) = xs.last {
                            xs.xns[xs.cur_op] += (t - l).as_nanos() as u64;
                        }
                        xs.last = Some(t);
                        xs.cur_op = i.op as usize;
                    }
                }
                match i.op {
                    op::READ_COL => {
                        let id = r!(i.a) as usize;
                        if id >= n {
                            break 'fail false;
                        }
                        r!(i.b) = u64::from(vc[(i.f & 7) as usize][id]);
                    }
                    op::READ_CHILD => {
                        let id = r!(i.a) as usize;
                        if id >= n || par[id] == u32::MAX {
                            break 'fail false;
                        }
                        r!(i.b) = u64::from(vc[(i.f & 7) as usize][id]);
                    }
                    op::TEST_EQ_C => {
                        if r!(i.a) != u64::from(i.x) {
                            break 'fail false;
                        }
                    }
                    op::TEST_NE_C => {
                        if r!(i.a) == u64::from(i.x) {
                            break 'fail false;
                        }
                    }
                    op::TEST_EQ_K => {
                        if r!(i.a) != k64[i.x as usize] {
                            break 'fail false;
                        }
                    }
                    op::TEST_NE_K => {
                        if r!(i.a) == k64[i.x as usize] {
                            break 'fail false;
                        }
                    }
                    op::TEST_EQ_R => {
                        if r!(i.a) != r!(i.b) {
                            break 'fail false;
                        }
                    }
                    op::TEST_NE_R => {
                        if r!(i.a) == r!(i.b) {
                            break 'fail false;
                        }
                    }
                    op::TEST_IN_C => {
                        if inset(&pool[i.x as usize..(i.x + i.y) as usize], r!(i.a)) == (i.f != 0) {
                            break 'fail false;
                        }
                    }
                    // ---- fused: a read and the constant test of what it read (two steps when the read finds its row)
                    op::RC_EQ | op::RC_NE | op::RC_IN | op::RC_EQ_R | op::RCH_EQ | op::RCH_NE | op::RCH_IN | op::RCH_EQ_R => {
                        let id = r!(i.a) as usize;
                        let child = i.op >= op::RCH_EQ && i.op != op::RC_EQ_R;
                        if id >= n || (child && par[id] == u32::MAX) {
                            break 'fail false;
                        }
                        let val = u64::from(vc[(i.f & 7) as usize][id]);
                        r!(i.b) = val;
                        steps += 1;
                        let pass = match i.op {
                            op::RC_EQ | op::RCH_EQ => {
                                if D {
                                    self.c.op_steps[24] += 1;
                                }
                                val == u64::from(i.x)
                            }
                            op::RC_NE | op::RCH_NE => {
                                if D {
                                    self.c.op_steps[25] += 1;
                                }
                                val != u64::from(i.x)
                            }
                            op::RC_IN | op::RCH_IN => {
                                if D {
                                    self.c.op_steps[28] += 1;
                                }
                                let len = pool[i.x as usize] as usize;
                                inset(&pool[i.x as usize + 1..i.x as usize + 1 + len], val) != (i.f & 128 != 0)
                            }
                            _ => {
                                if D {
                                    self.c.op_steps[26] += 1;
                                }
                                val == r!(i.c)
                            }
                        };
                        if !pass {
                            break 'fail false;
                        }
                    }
                    op::RF_EQ | op::RF_NE | op::RF_IN | op::RF_EQ_R => {
                        let val = self.cache.facts.as_ref().unwrap().cols[(i.y >> 8) as usize][(i.y & 255) as usize][r!(i.a) as usize];
                        r!(i.b) = val;
                        steps += 1;
                        let pass = match i.op {
                            op::RF_EQ => {
                                if D {
                                    self.c.op_steps[24] += 1;
                                }
                                val == u64::from(i.x)
                            }
                            op::RF_NE => {
                                if D {
                                    self.c.op_steps[25] += 1;
                                }
                                val != u64::from(i.x)
                            }
                            op::RF_IN => {
                                if D {
                                    self.c.op_steps[28] += 1;
                                }
                                let len = pool[i.x as usize] as usize;
                                inset(&pool[i.x as usize + 1..i.x as usize + 1 + len], val) != (i.f != 0)
                            }
                            _ => {
                                if D {
                                    self.c.op_steps[26] += 1;
                                }
                                val == r!(i.c)
                            }
                        };
                        if !pass {
                            break 'fail false;
                        }
                    }
                    op::EQ_EQ => {
                        if r!(i.a) != u64::from(i.x) {
                            break 'fail false;
                        }
                        steps += 1;
                        if D {
                            self.c.op_steps[24] += 1;
                        }
                        if r!(i.b) != u64::from(i.y) {
                            break 'fail false;
                        }
                    }
                    op::READ => {
                        let id = r!(i.a) as u32;
                        match self.tread(RELS[(i.f >> 4) as usize], i.f & 15, id) {
                            Some(val) => r!(i.b) = val,
                            None => break 'fail false,
                        }
                    }
                    op::TEST => {
                        if !self.xtest(i) {
                            break 'fail false;
                        }
                    }
                    op::TEST_CMP => {
                        let (lv, rv) = (xv!(i.x), xv!(i.y));
                        let ord = if i.f & 16 != 0 { (lv as i64).cmp(&(rv as i64)) } else { lv.cmp(&rv) };
                        let pass = match i.f & 15 {
                            2 => ord.is_lt(),
                            3 => ord.is_le(),
                            4 => ord.is_gt(),
                            _ => ord.is_ge(),
                        };
                        if !pass {
                            break 'fail false;
                        }
                    }
                    op::ROW_ANY => {
                        if r!(i.a) as u32 as usize >= n {
                            break 'fail false;
                        }
                    }
                    op::ROW_CHILD => {
                        let id = r!(i.a) as u32 as usize;
                        if id >= n || par[id] == u32::MAX {
                            break 'fail false;
                        }
                    }
                    op::ROW => {
                        let id = r!(i.a) as u32;
                        if !self.row_exists(RELS[i.f as usize], id) {
                            break 'fail false;
                        }
                    }
                    op::READ_FACT => {
                        let val = self.cache.facts.as_ref().unwrap().cols[i.x as usize][i.f as usize][r!(i.a) as usize];
                        r!(i.b) = val;
                    }
                    op::ITER_CHILDREN | op::AXIS_CHILD => {
                        need_frame!('fail, i);
                        let p0 = r!(i.b) as u32 as usize;
                        if p0 >= n {
                            break 'fail false;
                        }
                        frame!(K_CHILD, i.a, child_off[p0], child_off[p0 + 1], 0, 0, 0);
                        break 'fail false;
                    }
                    op::ITER_FIELD => {
                        need_frame!('fail, i);
                        let p0 = r!(i.b) as u32 as usize;
                        if p0 >= n {
                            break 'fail false;
                        }
                        frame!(K_FIELD, i.a, child_off[p0], child_off[p0 + 1], u32::from(i.c), u64::from(i.x), 0);
                        break 'fail false;
                    }
                    op::ITER_KIND => {
                        need_frame!('fail, i);
                        frame!(K_KINDS, i.a, 0, 0, 0, (u64::from(i.x) << 32) | u64::from(i.y), 0);
                        break 'fail false;
                    }
                    op::ITER_VALUE_C => {
                        need_frame!('fail, i);
                        sync!();
                        let (o, len) = self.xconst(i.x);
                        frame!(K_IA, i.a, o, o + len, 0, 0, 0);
                        break 'fail false;
                    }
                    op::ITER_VALUE_R => {
                        need_frame!('fail, i);
                        sync!();
                        let key = r!(i.b) as u32;
                        let (a, b) = self.xtree_range(i.x, key);
                        frame!(K_IA, i.a, a, b, 0, 0, 0);
                        break 'fail false;
                    }
                    op::ITER_SCAN => {
                        let rel = RELS[i.f as usize];
                        let cnt = if rel == TRel::Id { v.n_nodes } else { v.len() };
                        sync!();
                        for id in 0..cnt {
                            if !self.row_exists(rel, id as u32) {
                                continue;
                            }
                            r!(i.a) = id as u64;
                            self.c.visits += 1;
                            if self.xrun::<D>(pc as u32 + 1) {
                                break 'fail true;
                            }
                        }
                        break 'fail false;
                    }
                    op::ITER_FACT_KEYED => {
                        need_frame!('fail, i);
                        sync!();
                        let key = r!(i.b);
                        let (a, b) = self.keyed_rows(i.x as u16, key);
                        frame!(K_RANGE, i.a, a, b, 0, 0, 0);
                        break 'fail false;
                    }
                    op::ITER_FACT_R | op::ITER_FACT_C => {
                        need_frame!('fail, i);
                        sync!();
                        self.xfact_index(i.c);
                        let key = if i.op == op::ITER_FACT_R { r!(i.b) } else { k64[i.y as usize] };
                        let (first, next) = self.xfact_first(i.c, key);
                        if first == u32::MAX {
                            break 'fail false;
                        }
                        frame!(K_CHAIN, i.a, first, 0, 0, u64::from(next), 0);
                        break 'fail false;
                    }
                    op::ITER_FACT_L => {
                        sync!();
                        self.xfact_index(i.c);
                        let nk = pool[i.y as usize] as usize;
                        for j in 0..nk {
                            let key = k64[pool[i.y as usize + 1 + j] as usize];
                            let (mut row, next) = self.xfact_first(i.c, key);
                            while row != u32::MAX {
                                r!(i.a) = u64::from(row);
                                self.c.visits += 1;
                                if self.xrun::<D>(pc as u32 + 1) {
                                    break 'fail true;
                                }
                                row = self.cache.x.ia[(next + row) as usize];
                            }
                        }
                        break 'fail false;
                    }
                    op::ITER_FACT2 => {
                        need_frame!('fail, i);
                        sync!();
                        self.xfact_index2(i.c);
                        let Some(&first) = self.cache.x.f2at.get(&(i.c, k64[i.y as usize], k64[i.y as usize + 1])) else { break 'fail false };
                        let next = self.cache.x.f2[i.c as usize].1;
                        frame!(K_CHAIN, i.a, first, 0, 0, u64::from(next), 0);
                        break 'fail false;
                    }
                    op::ITER_FACT_SCAN => {
                        need_frame!('fail, i);
                        sync!();
                        self.ensure(i.x as u16);
                        let cnt = self.cache.facts.as_ref().unwrap().cols[i.x as usize].first().map_or(0, Vec::len);
                        frame!(K_RANGE, i.a, 0, cnt as u32, 0, 0, 0);
                        break 'fail false;
                    }
                    op::AXIS_PARENT => {
                        let s = r!(i.b) as u32 as usize;
                        if s >= n {
                            break 'fail false;
                        }
                        let p0 = par[s];
                        if p0 == u32::MAX {
                            break 'fail false;
                        }
                        r!(i.a) = u64::from(p0);
                    }
                    op::AXIS_ANC => {
                        need_frame!('fail, i);
                        let s = r!(i.b) as u32 as usize;
                        if s >= n {
                            break 'fail false;
                        }
                        frame!(K_ANC, i.a, par[s], 0, 0, 0, 0);
                        break 'fail false;
                    }
                    op::AXIS_DESC => {
                        let s = r!(i.b) as u32 as usize;
                        if s >= n {
                            break 'fail false;
                        }
                        let mut stack: Vec<u32> = Vec::new();
                        let (b, e) = (child_off[s] as usize, child_off[s + 1] as usize);
                        stack.extend(children[b..e].iter().rev());
                        sync!();
                        while let Some(d) = stack.pop() {
                            r!(i.a) = u64::from(d);
                            self.c.visits += 1;
                            if self.xrun::<D>(pc as u32 + 1) {
                                break 'fail true;
                            }
                            let (b, e) = (child_off[d as usize] as usize, child_off[d as usize + 1] as usize);
                            stack.extend(children[b..e].iter().rev());
                        }
                        break 'fail false;
                    }
                    op::WITHIN => {
                        let (a, d) = (r!(i.a) as u32, r!(i.b) as u32);
                        if d as usize >= n || a as usize >= n {
                            break 'fail false;
                        }
                        let mut y = par[d as usize];
                        while y != u32::MAX && y != a {
                            y = par[y as usize];
                        }
                        if y != a {
                            break 'fail false;
                        }
                    }
                    op::CHILD_TOWARD => {
                        let (p0, d) = (r!(i.b) as u32, r!(i.c) as u32);
                        if d as usize >= n || p0 as usize >= n {
                            break 'fail false;
                        }
                        let mut y = par[d as usize];
                        while y != u32::MAX && par[y as usize] != p0 {
                            y = par[y as usize];
                        }
                        if y == u32::MAX {
                            break 'fail false;
                        }
                        if i.f != 0 {
                            if fld[y as usize] != i.x {
                                break 'fail false;
                            }
                            r!(i.y) = u64::from(i.x);
                        }
                        visits += 1;
                        r!(i.a) = u64::from(y);
                    }
                    op::ITER_RANGE => {
                        need_frame!('fail, i);
                        let at = i.x as usize;
                        let (src, lr, hr, tid) = (pool[at], pool[at + 1], pool[at + 2], pool[at + 3]);
                        let lo = (i.f & 1 != 0).then(|| (r!(lr) as i64, i.f & 2 != 0));
                        let hi = if i.f & 4 != 0 { r!(hr) as i64 } else { 0 };
                        sync!();
                        // flag: bit 0 an upper bound, bit 1 inclusive, bit 2 started
                        let (a, b) = if i.f & 16 != 0 {
                            let key = r!(src) as u32;
                            self.xtree_range(tid, key)
                        } else {
                            let (o, l) = self.xconst(src);
                            (o, o + l)
                        };
                        let list: &[u32] = &self.cache.x.ia[a as usize..b as usize];
                        let first = match lo {
                            Some((bound, true)) => list.partition_point(|&id| i64::from(v.start[id as usize]) < bound),
                            Some((bound, false)) => list.partition_point(|&id| i64::from(v.start[id as usize]) <= bound),
                            None => 0,
                        };
                        frame!(K_RANGEW, i.a, a + first as u32, b, 0, hi as u64, (i.f >> 2) & 3);
                        break 'fail false;
                    }
                    op::NEAR => {
                        let s = r!(i.b) as u32 as usize;
                        if s >= n {
                            break 'fail false;
                        }
                        let mut a = par[s];
                        let mut found = false;
                        sync!();
                        while a != u32::MAX {
                            r!(i.a) = u64::from(a);
                            self.c.visits += 1;
                            if self.xany::<D>(i.x) {
                                found = true;
                                break;
                            }
                            a = par[a as usize];
                        }
                        if !found {
                            break 'fail false;
                        }
                        r!(i.a) = u64::from(a);
                    }
                    op::SUB1 => {
                        // the check's sub-program runs in this loop: a call frame takes its answer
                        need_frame!('fail, i);
                        self.c.subs += 1;
                        fr.push(Frame { kind: K_CALL, flag: M_SUB | if i.f != 0 { FL_NEG } else { 0 }, out: 0, pc: pc as u32, cur: 0, end: 0, src: 0, x: 0, y: 0 });
                        pc = i.x as usize;
                        continue 'run;
                    }
                    op::SUB => {
                        need_frame!('fail, i);
                        let cnt = pool[i.x as usize];
                        if cnt == 0 {
                            if i.f == 0 {
                                break 'fail false;
                            }
                        } else {
                            self.c.subs += 1;
                            fr.push(Frame { kind: K_CALL, flag: M_SUB | if i.f != 0 { FL_NEG } else { 0 }, out: 0, pc: pc as u32, cur: 1, end: cnt, src: i.x, x: 0, y: 0 });
                            pc = pool[i.x as usize + 1] as usize;
                            continue 'run;
                        }
                    }
                    op::NEAR_MEMO => {
                        sync!();
                        let s = r!(i.b) as u32;
                        match self.xnear_memo::<D>(s, i.a, i.x, i.y) {
                            Some(a) => r!(i.a) = u64::from(a),
                            None => break 'fail false,
                        }
                    }
                    op::SUB_KEY => {
                        need_frame!('fail, i);
                        let k = r!(i.a);
                        let st = &mut self.cache.x.ksite[i.y as usize];
                        st.0 += 1;
                        let off = st.0 >= 256 && st.1 * 10 >= st.0 * 9 && st.2 < 32 * st.1;
                        let hit = if off { None } else { self.cache.x.keym.get(&(i.y, k)).copied() };
                        let cnt = pool[i.x as usize];
                        if hit.is_none() && cnt > 0 {
                            // a miss: the sub-programs run in this loop; the call frame records the answer under the key
                            self.c.subs += 1;
                            fr.push(Frame { kind: K_CALL, flag: M_SUBKEY | if i.f != 0 { FL_NEG } else { 0 } | if off { FL_OFF } else { 0 }, out: 0, pc: pc as u32, cur: 1, end: cnt, src: i.x, x: self.c.steps + steps, y: 0 });
                            pc = pool[i.x as usize + 1] as usize;
                            continue 'run;
                        }
                        sync!();
                        let any = match hit {
                            Some(a) => a,
                            None => {
                                let s0 = self.c.steps;
                                let a = self.xany::<D>(i.x);
                                let st = &mut self.cache.x.ksite[i.y as usize];
                                st.1 += 1;
                                st.2 += self.c.steps - s0;
                                if !off {
                                    self.cache.x.keym.insert((i.y, k), a);
                                }
                                a
                            }
                        };
                        if any == (i.f != 0) {
                            break 'fail false;
                        }
                    }
                    op::ANC_EXISTS => {
                        sync!();
                        let s = r!(i.b) as u32;
                        if self.xanc_exists::<D>(s, i.a, i.x, i.y) == (i.f != 0) {
                            break 'fail false;
                        }
                    }
                    op::SUB_FIRST => {
                        sync!();
                        self.c.subs += 1;
                        if self.xrun::<D>(i.x) {
                            self.first_fail = true;
                            break 'fail false;
                        }
                    }
                    op::SUB_MEMO => {
                        let generation = self.cache.x.generation;
                        let m = self.cache.x.submemo[i.y as usize];
                        let any = if m >> 1 == generation {
                            m & 1 == 1
                        } else {
                            sync!();
                            let a = self.xany::<D>(i.x);
                            self.cache.x.submemo[i.y as usize] = (generation << 1) | u32::from(a);
                            a
                        };
                        if any == (i.f != 0) {
                            break 'fail false;
                        }
                    }
                    op::EMIT => {
                        let slot = if i.y == u32::MAX { self.cur_rule } else { i.y as usize };
                        for k in 0..i.c as usize {
                            let val = xv!(pool[i.x as usize + k]);
                            self.outs[slot].push(val);
                        }
                        self.c.emitted += 1;
                        break 'fail false;
                    }
                    op::YIELD => break 'fail true,
                    op::FAIL => {
                        steps -= 1;
                        if pc == 0 {
                            // word 0 is the per-kind driver's entry: one frame walks the kind's (candidate, group) pairs
                            let (lo, hi) = self.cache.x.drive_range;
                            fr.push(Frame { kind: K_DRIVE, flag: 0, out: 0, pc: 0, cur: lo, end: hi, src: 0, x: 0, y: 0 });
                        }
                        break 'fail false;
                    }
                    op::JUMP => {
                        // an indirection, not a step of the plan
                        steps -= 1;
                        pc = i.x as usize;
                        continue 'run;
                    }
                    op::COLLECT => {
                        for k in 0..i.c as usize {
                            let val = r!(pool[i.x as usize + k]);
                            self.collect.push(val);
                        }
                        break 'fail false;
                    }
                    op::CAPTURE => {
                        if i.c == 0 {
                            self.cache.x.cap.push(0);
                        }
                        for k in 0..i.c as usize {
                            let val = r!(pool[i.x as usize + k]);
                            self.cache.x.cap.push(val);
                        }
                        break 'fail false;
                    }
                    op::FORK => {
                        if D {
                            // the counter build charges a child owned by one rule to that rule: children run as calls
                            self.c.fork_children += u64::from(i.y);
                            sync!();
                            for j in 0..i.y as usize {
                                let at = i.x as usize + 4 * j;
                                let slots = &pool[pool[at + 2] as usize..(pool[at + 2] + pool[at + 3]) as usize];
                                if !slots.iter().any(|&s| self.cache.enabled[s as usize]) {
                                    self.c.fork_disabled += 1;
                                    continue;
                                }
                                let r0 = prog.slot_rule[slots[0] as usize] as usize;
                                if self.cur_rule != r0 && slots.iter().all(|&s| prog.slot_rule[s as usize] as usize == r0) {
                                    let (s0, v0, saved) = (self.c.steps, self.c.visits, self.cur_rule);
                                    self.cur_rule = r0;
                                    let y = self.xrun::<D>(pool[at]);
                                    self.cur_rule = saved;
                                    let (ds, dv) = (self.c.steps - s0, self.c.visits - v0);
                                    self.c.rule_steps[r0] += ds;
                                    self.c.rule_visits[r0] += dv;
                                    self.c.rule_steps[saved] = self.c.rule_steps[saved].wrapping_sub(ds);
                                    self.c.rule_visits[saved] = self.c.rule_visits[saved].wrapping_sub(dv);
                                    if y {
                                        break 'fail true;
                                    }
                                    continue;
                                }
                                if self.xrun::<D>(pool[at]) {
                                    break 'fail true;
                                }
                            }
                            break 'fail false;
                        }
                        need_frame!('fail, i);
                        frame!(K_FORK, 0, 0, i.y, i.x, 0, 0);
                        break 'fail false;
                    }
                    op::HOIST => {
                        need_frame!('fail, i);
                        let at = i.x as usize;
                        let (site, seg, nkey) = (pool[at], pool[at + 1], pool[at + 2]);
                        let st = &mut self.cache.x.hsite[site as usize];
                        st.0 += 1;
                        let off = st.0 >= 256 && st.1 * 10 >= st.0 * 9 && st.2 < 32 * st.1;
                        let cached = if off {
                            None
                        } else if nkey == 0 {
                            let e = self.cache.x.hoist0[site as usize];
                            (e.0 == self.cache.x.generation).then_some((e.1, e.2))
                        } else {
                            let k0 = r!(pool[at + 3]);
                            let k1 = if nkey > 1 { r!(pool[at + 4]) } else { 0 };
                            self.cache.x.hoistk.get(&(site, k0, k1)).copied()
                        };
                        match cached {
                            Some((o, l)) => {
                                self.c.hoist_runs += 1;
                                let ncaps = pool[at + 5];
                                frame!(K_TUP, 0, o, o + l, o, (u64::from(pool[at + 6]) << 32) | (u64::from(ncaps) << 16) | u64::from(ncaps.max(1)), 0);
                                break 'fail false;
                            }
                            None => {
                                // a miss: the segment runs in this loop; the call frame turns into the replay of its captures
                                let base = self.cache.x.cap.len() as u32;
                                fr.push(Frame { kind: K_CALL, flag: M_HOIST | if off { FL_OFF } else { 0 }, out: 0, pc: pc as u32, cur: base, end: 0, src: 0, x: self.c.steps + steps, y: 0 });
                                pc = seg as usize;
                                continue 'run;
                            }
                        }
                    }
                    op::HOIST_PROBE => {
                        need_frame!('fail, i);
                        sync!();
                        let at = i.x as usize;
                        let Some((o, l)) = self.xhoist_probe::<D>(at) else { break 'fail false };
                        let ncaps = pool[at + 5];
                        frame!(K_TUP, 0, o, o + l, o, (u64::from(pool[at + 6]) << 32) | (u64::from(ncaps) << 16) | u64::from(ncaps.max(1)), 0);
                        break 'fail false;
                    }
                    op::UNION | op::UNION_KEY => {
                        need_frame!('fail, i);
                        let at = i.x as usize;
                        let (subs, site) = (pool[at + 2], pool[at + 3]);
                        let cnt = pool[subs as usize];
                        // a hit of the memo replays; a miss runs the sub-programs in this loop (the call frame collects)
                        let (hit, off) = if i.op == op::UNION {
                            let e = self.cache.x.umemo[site as usize];
                            ((i.f != 0 && e.0 == self.cache.x.generation).then_some((e.1, e.2)), false)
                        } else {
                            let k = r!(i.a);
                            let st = &mut self.cache.x.ksite[site as usize];
                            st.0 += 1;
                            let off = st.0 >= 256 && st.1 * 10 >= st.0 * 9 && st.2 < 32 * st.1;
                            (if off { None } else { self.cache.x.ukeym.get(&(site, k)).copied() }, off)
                        };
                        if hit.is_none() && cnt > 0 {
                            self.c.subs += 1;
                            let base = self.collect.len() as u64;
                            fr.push(Frame { kind: K_CALL, flag: M_UNION | if off { FL_OFF } else { 0 }, out: 0, pc: pc as u32, cur: 1, end: cnt, src: subs, x: self.c.steps + steps, y: base });
                            pc = pool[subs as usize + 1] as usize;
                            continue 'run;
                        }
                        sync!();
                        let (o, l, loose) = match hit {
                            Some((o, l)) => (o, l, false),
                            None if i.op == op::UNION => self.xunion::<D>(i.x, i.f != 0),
                            None => {
                                // no sub-program: the (empty) answer through the call path, its lookup not counted twice
                                self.cache.x.ksite[site as usize].0 -= 1;
                                let k = r!(i.a);
                                self.xunion_key::<D>(i.x, k)
                            }
                        };
                        let nouts = pool[i.x as usize];
                        frame!(K_TUP, 0, o, o + l, o, (u64::from(pool[i.x as usize + 1]) << 32) | (u64::from(nouts) << 16) | u64::from(nouts.max(1)), FL_VISIT | if loose { FL_RECLAIM } else { 0 });
                        break 'fail false;
                    }
                    op::CLOSURE => {
                        need_frame!('fail, i);
                        sync!();
                        let (o, l) = self.xclosure::<D>(i);
                        let at = i.x as usize;
                        let (nkeys, ncaps, capl) = (pool[at], pool[at + 1], pool[at + 2]);
                        frame!(K_TUP, 0, o, o + l, o, (u64::from(capl) << 32) | (u64::from(ncaps) << 16) | u64::from(1 + nkeys), FL_RECLAIM);
                        break 'fail false;
                    }
                    op::LET => {
                        let val = xv!(i.x);
                        r!(i.a) = val;
                    }
                    op::SIBLING => {
                        let s = r!(i.b) as u32;
                        if s as usize >= n || par[s as usize] == u32::MAX {
                            break 'fail false;
                        }
                        let p0 = par[s as usize] as usize;
                        let kids = &children[child_off[p0] as usize..child_off[p0 + 1] as usize];
                        let Some(at) = kids.iter().position(|&k| k == s) else { break 'fail false };
                        let j = if i.f != 0 { at + 1 } else { at.wrapping_sub(1) };
                        match kids.get(j) {
                            Some(&k) => r!(i.a) = u64::from(k),
                            None => break 'fail false,
                        }
                    }
                    op::SPAN_BETWEEN => {
                        let (a, b) = (xv!(i.x), xv!(i.y));
                        r!(i.a) = (a & !0xffff_ffff) | (b & 0xffff_ffff);
                    }
                    op::SPAN_BOUNDS => {
                        let s = xv!(i.x);
                        r!(i.a) = s >> 32;
                        r!(i.b) = s & 0xffff_ffff;
                    }
                    op::SOURCE_MATCHES => {
                        sync!();
                        if self.xsource_matches::<D>(i, pc as u32) {
                            break 'fail true;
                        }
                        break 'fail false;
                    }
                    op::CAPTURE_TEXT => {
                        sync!();
                        if self.xcapture_text::<D>(i, pc as u32) {
                            break 'fail true;
                        }
                        break 'fail false;
                    }
                    op::NEAR_DERIVED => {
                        sync!();
                        match self.xnear_derived::<D>(i) {
                            Some(y) => r!(i.a) = y,
                            None => break 'fail false,
                        }
                    }
                    op::TEXT_FN => {
                        let at = i.x as usize;
                        let mut t = String::new();
                        let mut ok = true;
                        for k in 0..pool[at] as usize {
                            let o = tagged(x, at + 1 + 2 * k);
                            let val = self.xoval(o);
                            match self.xtext(o, val) {
                                Some(s) => t.push_str(s),
                                None => {
                                    ok = false;
                                    break;
                                }
                            }
                        }
                        if !ok {
                            break 'fail false;
                        }
                        if i.f != 0 {
                            t = t.to_lowercase();
                        }
                        r!(i.a) = u64::from(self.sym.minted(t));
                    }
                    op::REPLACE => {
                        let at = i.x as usize;
                        let o = tagged(x, at);
                        let val = self.xoval(o);
                        let (regex, replacement) = (pool[at + 2] as usize, x.strs[pool[at + 3] as usize].as_str());
                        let res = match self.xtext(o, val).map(str::to_owned) {
                            None => NULL,
                            Some(t) => {
                                let mut result = String::new();
                                let mut last = 0;
                                for m in prog.regexes[regex].find_iter(t.as_bytes()) {
                                    result.push_str(&t[last..m.start()]);
                                    result.push_str(replacement);
                                    last = m.end();
                                }
                                result.push_str(&t[last..]);
                                self.sym.minted(result)
                            }
                        };
                        r!(i.a) = u64::from(res);
                    }
                    _ => unreachable!("an opcode the encoder never writes"),
                }
                pc += 1;
                continue 'run;
            };
            // ---- backtrack (the branch failed): the innermost running iteration gives its next element, or ends;
            // ---- unwind (the program yielded): iterations are dropped down to the innermost inline call, which gets the answer
            loop {
                let Some(f) = fr.last_mut() else {
                    sync!();
                    return yielded;
                };
                if f.kind >= K_CALL {
                    // a boundary: an inline sub-program (or the per-kind driver) ended with `yielded` as its answer
                    let i: &XIns = &code[f.pc as usize];
                    match f.kind {
                        K_DRIVE => {
                            // the next (candidate, group) of the kind: nodes outer, groups inner
                            yielded = false;
                            let ng = self.cache.x.drive.len() as u32;
                            if f.src == ng {
                                f.src = 0;
                                f.cur += 1;
                            }
                            if f.cur < f.end {
                                let g = self.cache.x.drive[f.src as usize] as usize;
                                f.src += 1;
                                let id = self.cache.kind_ids[f.cur as usize];
                                r!(x.group_out[g]) = u64::from(id);
                                visits += 1;
                                self.cur_rule = x.group_rule[g] as usize;
                                pc = x.group_entry[g] as usize + 1;
                                prev = 0;
                                continue 'run;
                            }
                        }
                        _ => match f.flag & 7 {
                            M_SUB | M_SUBKEY => {
                                // any sub-program yielding answers the check; else the next one runs
                                if !yielded && f.cur < f.end {
                                    self.c.subs += 1;
                                    pc = pool[(f.src + 1 + f.cur) as usize] as usize;
                                    f.cur += 1;
                                    prev = 0;
                                    continue 'run;
                                }
                                if f.flag & 7 == M_SUBKEY {
                                    let k = r!(i.a);
                                    let now = self.c.steps + steps;
                                    let st = &mut self.cache.x.ksite[i.y as usize];
                                    st.1 += 1;
                                    st.2 += now - f.x;
                                    if f.flag & FL_OFF == 0 {
                                        self.cache.x.keym.insert((i.y, k), yielded);
                                    }
                                }
                                let pass = yielded != (f.flag & FL_NEG != 0);
                                yielded = false;
                                if pass {
                                    pc = f.pc as usize + 1;
                                    prev = 0;
                                    let _ = fr.pop();
                                    continue 'run;
                                }
                            }
                            M_HOIST => {
                                // the segment ran: its captures become the site's tuples, replayed by this frame
                                yielded = false;
                                let at = i.x as usize;
                                let (site, nkey) = (pool[at], pool[at + 2]);
                                let k0 = if nkey > 0 { r!(pool[at + 3]) } else { 0 };
                                let k1 = if nkey > 1 { r!(pool[at + 4]) } else { 0 };
                                let now = self.c.steps + steps;
                                let off = f.flag & FL_OFF != 0;
                                let xs = &mut self.cache.x;
                                let base = f.cur as usize;
                                let o = xs.tup.len();
                                xs.tup.extend_from_slice(&xs.cap[base..]);
                                xs.cap.truncate(base);
                                let l = xs.tup.len() - o;
                                assert!(xs.tup.len() < u32::MAX as usize);
                                self.c.hoist_misses += 1;
                                let st = &mut xs.hsite[site as usize];
                                st.1 += 1;
                                st.2 += now - f.x;
                                if !off {
                                    if nkey == 0 {
                                        xs.hoist0[site as usize] = (xs.generation, o as u32, l as u32);
                                    } else {
                                        xs.hoistk.insert((site, k0, k1), (o as u32, l as u32));
                                    }
                                }
                                self.c.hoist_runs += 1;
                                let ncaps = pool[at + 5];
                                *f = Frame { kind: K_TUP, flag: if off { FL_RECLAIM } else { 0 }, out: 0, pc: f.pc, cur: o as u32, end: (o + l) as u32, src: o as u32, x: (u64::from(pool[at + 6]) << 32) | (u64::from(ncaps) << 16) | u64::from(ncaps.max(1)), y: 0 };
                                continue;
                            }
                            _ => {
                                // M_UNION: every sub-program runs to its end; then the distinct rows are replayed by this frame
                                yielded = false;
                                if f.cur < f.end {
                                    self.c.subs += 1;
                                    pc = pool[(f.src + 1 + f.cur) as usize] as usize;
                                    f.cur += 1;
                                    prev = 0;
                                    continue 'run;
                                }
                                let at = i.x as usize;
                                let (nouts, site) = (pool[at], pool[at + 3]);
                                let now = self.c.steps + steps;
                                let (o, l) = self.xdistinct(f.y as usize, nouts as usize);
                                let loose = if i.op == op::UNION {
                                    if i.f != 0 {
                                        self.cache.x.umemo[site as usize] = (self.cache.x.generation, o, l);
                                    }
                                    i.f == 0
                                } else {
                                    let k = r!(i.a);
                                    let st = &mut self.cache.x.ksite[site as usize];
                                    st.1 += 1;
                                    st.2 += now - f.x;
                                    if f.flag & FL_OFF == 0 {
                                        self.cache.x.ukeym.insert((site, k), (o, l));
                                    }
                                    f.flag & FL_OFF != 0
                                };
                                *f = Frame { kind: K_TUP, flag: FL_VISIT | if loose { FL_RECLAIM } else { 0 }, out: 0, pc: f.pc, cur: o, end: o + l, src: o, x: (u64::from(pool[at + 1]) << 32) | (u64::from(nouts) << 16) | u64::from(nouts.max(1)), y: 0 };
                                continue;
                            }
                        },
                    }
                    let _ = fr.pop();
                    continue;
                }
                if yielded {
                    // an iteration of the sub-program that yielded: dropped
                    let _ = fr.pop();
                    continue;
                }
                match f.kind {
                    K_CHILD => {
                        if f.cur < f.end {
                            r!(f.out) = u64::from(children[f.cur as usize]);
                            f.cur += 1;
                            visits += 1;
                            pc = f.pc as usize + 1;
                            prev = 0;
                            continue 'run;
                        }
                    }
                    K_FIELD => {
                        let mut hit = false;
                        while f.cur < f.end {
                            let ch = children[f.cur as usize];
                            f.cur += 1;
                            if u64::from(fld[ch as usize]) == f.x {
                                r!(f.out) = u64::from(ch);
                                r!(f.src) = f.x;
                                hit = true;
                                break;
                            }
                        }
                        if hit {
                            visits += 1;
                            pc = f.pc as usize + 1;
                            prev = 0;
                            continue 'run;
                        }
                    }
                    K_RANGE => {
                        if f.cur < f.end {
                            r!(f.out) = u64::from(f.cur);
                            f.cur += 1;
                            visits += 1;
                            pc = f.pc as usize + 1;
                            prev = 0;
                            continue 'run;
                        }
                    }
                    K_CHAIN => {
                        // a chain of rows: x is the offset of the chain's links in the index arena
                        if f.cur != u32::MAX {
                            r!(f.out) = u64::from(f.cur);
                            f.cur = self.cache.x.ia[f.x as usize + f.cur as usize];
                            visits += 1;
                            pc = f.pc as usize + 1;
                            prev = 0;
                            continue 'run;
                        }
                    }
                    K_IA => {
                        // a run of ids in the index arena
                        if f.cur < f.end {
                            r!(f.out) = u64::from(self.cache.x.ia[f.cur as usize]);
                            f.cur += 1;
                            visits += 1;
                            pc = f.pc as usize + 1;
                            prev = 0;
                            continue 'run;
                        }
                    }
                    K_ANC => {
                        if f.cur != u32::MAX {
                            r!(f.out) = u64::from(f.cur);
                            f.cur = par[f.cur as usize];
                            visits += 1;
                            pc = f.pc as usize + 1;
                            prev = 0;
                            continue 'run;
                        }
                    }
                    K_KINDS => {
                        // x: kinds offset << 32 | kinds left; cur..end: the running bucket
                        loop {
                            if f.cur < f.end {
                                r!(f.out) = u64::from(self.cache.kind_ids[f.cur as usize]);
                                f.cur += 1;
                                visits += 1;
                                pc = f.pc as usize + 1;
                                prev = 0;
                                continue 'run;
                            }
                            let left = f.x & 0xffff_ffff;
                            if left == 0 {
                                break;
                            }
                            let at = (f.x >> 32) as usize;
                            let kk = pool[at] as usize;
                            f.x = (((at + 1) as u64) << 32) | (left - 1);
                            let ko = &self.cache.kind_off;
                            if kk + 1 < ko.len() {
                                f.cur = ko[kk];
                                f.end = ko[kk + 1];
                            }
                        }
                    }
                    K_RANGEW => {
                        // a failed first-element check ends the walk (only after the walk gave an element)
                        if self.first_fail && f.flag & 4 != 0 {
                            self.first_fail = false;
                            f.cur = f.end;
                        }
                        f.flag |= 4;
                        if f.cur < f.end {
                            let id = self.cache.x.ia[f.cur as usize];
                            let st = i64::from(v.start[id as usize]);
                            let hi = f.x as i64;
                            let over = match f.flag & 3 {
                                3 => st > hi,
                                1 => st >= hi,
                                _ => false,
                            };
                            if !over {
                                f.cur += 1;
                                r!(f.out) = u64::from(id);
                                r!(code[f.pc as usize].b) = st as u64;
                                visits += 1;
                                pc = f.pc as usize + 1;
                                prev = 0;
                                continue 'run;
                            }
                        }
                    }
                    K_TUP => {
                        if f.cur < f.end {
                            let (capl, ncaps, w) = ((f.x >> 32) as usize, ((f.x >> 16) & 0xffff) as usize, (f.x & 0xffff) as u32);
                            for k in 0..ncaps {
                                let val = self.cache.x.tup[f.cur as usize + k];
                                r!(pool[capl + k]) = val;
                            }
                            f.cur += w;
                            if f.flag & FL_VISIT != 0 {
                                visits += 1;
                            }
                            pc = f.pc as usize + 1;
                            prev = 0;
                            continue 'run;
                        }
                        if f.flag & FL_RECLAIM != 0 {
                            // tuples nothing else keeps: the arena gives them back when they are its tail
                            let tup = &mut self.cache.x.tup;
                            if tup.len() == f.end as usize {
                                tup.truncate(f.src as usize);
                            }
                        }
                    }
                    K_FORK => {
                        let generation = self.cache.x.generation;
                        while f.cur < f.end {
                            let at = f.src as usize + 4 * f.cur as usize;
                            f.cur += 1;
                            let cid = pool[at + 1] as usize;
                            let m = self.cache.x.fork[cid];
                            let on = if m >> 1 == generation {
                                m & 1 == 1
                            } else {
                                let slots = &pool[pool[at + 2] as usize..(pool[at + 2] + pool[at + 3]) as usize];
                                let on = slots.iter().any(|&s| self.cache.enabled[s as usize]);
                                self.cache.x.fork[cid] = (generation << 1) | u32::from(on);
                                on
                            };
                            if on {
                                pc = pool[at] as usize;
                                prev = 0;
                                continue 'run;
                            }
                        }
                    }
                    _ => {}
                }
                let _ = fr.pop();
            }
        }
    }
}

// ------------------------------------------------------------------ the pack's compact section

/// The compact program's side tables for the pack, as words (the three big tables are stored as they sit in memory).
pub(crate) fn pack_meta(nav: &NavProgram) -> Vec<u32> {
    let x = &nav.x;
    let mut w: Vec<u32> = Vec::new();
    let list = |w: &mut Vec<u32>, xs: &[u32]| {
        w.push(xs.len() as u32);
        w.extend_from_slice(xs);
    };
    w.push(x.strs.len() as u32);
    for s in &x.strs {
        let b = s.as_bytes();
        w.push(b.len() as u32);
        for c in b.chunks(4) {
            let mut q = [0u8; 4];
            q[..c.len()].copy_from_slice(c);
            w.push(u32::from_le_bytes(q));
        }
    }
    w.push(x.consts.len() as u32);
    for c in &x.consts {
        w.extend([u32::from(c.0), u32::from(c.1), u32::from(c.2), c.3]);
    }
    w.push(x.fidx.len() as u32);
    for c in &x.fidx {
        w.extend([u32::from(c.0), u32::from(c.1)]);
    }
    w.push(x.tidx.len() as u32);
    for c in &x.tidx {
        w.extend([u32::from(c.0), u32::from(c.1)]);
    }
    w.push(x.f2.len() as u32);
    for c in &x.f2 {
        w.extend([u32::from(c.0), u32::from(c.1), u32::from(c.2)]);
    }
    list(&mut w, &x.group_entry);
    list(&mut w, &x.group_out);
    list(&mut w, &x.group_rule);
    list(&mut w, &x.shared_entry);
    w.extend(x.sites);
    for (m, om) in x.fact_e.iter().zip(&x.om_e) {
        list(&mut w, m);
        list(&mut w, &om.0);
        list(&mut w, &om.1);
    }
    for r in &x.aux_e {
        w.push(r.len() as u32);
        for a in r {
            list(&mut w, a);
        }
    }
    w
}

pub(crate) fn unpack_meta(nav: &mut NavProgram, w: &[u32]) {
    let mut at = 0usize;
    let u = |at: &mut usize| -> u32 {
        let v = w[*at];
        *at += 1;
        v
    };
    let list = |at: &mut usize| -> Vec<u32> {
        let n = w[*at] as usize;
        let v = w[*at + 1..*at + 1 + n].to_vec();
        *at += 1 + n;
        v
    };
    let mut x = std::mem::take(&mut nav.x);
    let ns = u(&mut at);
    for _ in 0..ns {
        let len = u(&mut at) as usize;
        let mut b: Vec<u8> = Vec::with_capacity(len + 4);
        for _ in 0..len.div_ceil(4) {
            b.extend_from_slice(&u(&mut at).to_le_bytes());
        }
        b.truncate(len);
        x.strs.push(String::from_utf8(b).expect("utf8"));
    }
    let nc = u(&mut at);
    for _ in 0..nc {
        let (a, b, c, d) = (u(&mut at), u(&mut at), u(&mut at), u(&mut at));
        x.consts.push((a as u8, b as u8, c != 0, d));
    }
    let nf = u(&mut at);
    for _ in 0..nf {
        let (a, b) = (u(&mut at), u(&mut at));
        x.fidx.push((a as u16, b as u8));
    }
    let nt = u(&mut at);
    for _ in 0..nt {
        let (a, b) = (u(&mut at), u(&mut at));
        x.tidx.push((a as u8, b as u8));
    }
    let n2 = u(&mut at);
    for _ in 0..n2 {
        let (a, b, c) = (u(&mut at), u(&mut at), u(&mut at));
        x.f2.push((a as u16, b as u8, c as u8));
    }
    x.group_entry = list(&mut at);
    x.group_out = list(&mut at);
    x.group_rule = list(&mut at);
    x.shared_entry = list(&mut at);
    for k in 0..6 {
        x.sites[k] = u(&mut at);
    }
    for _ in 0..nav.facts.len() {
        x.fact_e.push(list(&mut at));
        let l = list(&mut at);
        x.om_e.push((l, list(&mut at)));
    }
    for _ in 0..nav.rules.len() {
        let n = u(&mut at);
        x.aux_e.push((0..n).map(|_| list(&mut at)).collect());
    }
    assert_eq!(at, w.len(), "compact meta read whole");
    let mut xcode = vec![14u8; NXOPS];
    for o in XOPS {
        xcode[o.0 as usize] = o.2 as u8;
    }
    x.xcode = xcode;
    x.on = true;
    nav.x = x;
}

/// Table elements the pack stores as little-endian bytes and reads back with one read per table.
pub(crate) trait Packed: Copy + Default {
    const SIZE: usize;
    fn put(&self, out: &mut Vec<u8>);
    fn get(b: &[u8]) -> Self;
}

impl Packed for u8 {
    const SIZE: usize = 1;
    fn put(&self, out: &mut Vec<u8>) {
        out.push(*self);
    }
    fn get(b: &[u8]) -> Self {
        b[0]
    }
}

impl Packed for u32 {
    const SIZE: usize = 4;
    fn put(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
    fn get(b: &[u8]) -> Self {
        u32::from_le_bytes([b[0], b[1], b[2], b[3]])
    }
}

impl Packed for u64 {
    const SIZE: usize = 8;
    fn put(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
    fn get(b: &[u8]) -> Self {
        u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
    }
}

impl Packed for XIns {
    const SIZE: usize = 16;
    fn put(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&[self.op, self.f]);
        out.extend_from_slice(&self.a.to_le_bytes());
        out.extend_from_slice(&self.b.to_le_bytes());
        out.extend_from_slice(&self.c.to_le_bytes());
        out.extend_from_slice(&self.x.to_le_bytes());
        out.extend_from_slice(&self.y.to_le_bytes());
    }
    fn get(b: &[u8]) -> Self {
        XIns {
            op: b[0],
            f: b[1],
            a: u16::from_le_bytes([b[2], b[3]]),
            b: u16::from_le_bytes([b[4], b[5]]),
            c: u16::from_le_bytes([b[6], b[7]]),
            x: u32::from_le_bytes([b[8], b[9], b[10], b[11]]),
            y: u32::from_le_bytes([b[12], b[13], b[14], b[15]]),
        }
    }
}

/// a table's bytes for the pack
pub(crate) fn bytes_of<T: Packed>(xs: &[T]) -> Vec<u8> {
    let mut out = Vec::with_capacity(xs.len() * T::SIZE);
    for x in xs {
        x.put(&mut out);
    }
    out
}

/// Reads `n` elements of a table in 64 KiB pieces (safe Rust has no way to read into a typed table in place): one
/// fixed-width copy per element, no allocation per element, nothing interpreted, and no second copy of the table in memory.
pub(crate) fn read_table<T: Packed>(f: &mut std::fs::File, n: usize) -> Vec<T> {
    use std::io::Read as _;
    let mut out: Vec<T> = Vec::with_capacity(n);
    let per = (65536 / T::SIZE).max(1);
    let mut b = vec![0u8; per * T::SIZE];
    let mut left = n;
    while left > 0 {
        let take = left.min(per);
        let piece = &mut b[..take * T::SIZE];
        f.read_exact(piece).expect("pack section");
        out.extend(piece.chunks_exact(T::SIZE).map(T::get));
        left -= take;
    }
    out
}

/// The process's resident set and its peak, in bytes (/proc/self/status).
pub(crate) fn rss() -> (u64, u64) {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let field = |name: &str| -> u64 { s.lines().find(|l| l.starts_with(name)).and_then(|l| l.split_whitespace().nth(1)).and_then(|x| x.parse::<u64>().ok()).unwrap_or(0) * 1024 };
    (field("VmRSS:"), field("VmHWM:"))
}
