//! THROWAWAY (ryo-fmx3.1, step 6b): the finish stage. Each rule's WHOLE query, lowered by `scripts6b/lower.py` to one shared
//! plan of generic steps, run per file over the node table's columns and the provider's fills. No rule id is read here: the
//! plan names the emits, and the steps are the language's operators.
//!
//! Relations are column tables (one u64 a cell, typed per column). Base relations of the tree (`syntax-tree`,
//! `semantic-syntax`, `semantic-child`, `syntax-child`, `source-location`) are not tables: they are read in place from the
//! node table, by position (a join on their key is a position read; a scan with a kind test reads the kind buckets).
//! `token-link` comes from the provider's token order (`TokenOrder`). Every other relation is the provider's fill, read once a file when a step asks.

use ryo::languages::{self, CellKind, KindBits, NodeTable, Position, node_table};
use ryo_relations::domain::labels::types::NameRef;
use ryo_relations::domain::relations::service as catalog;
use rustc_hash::FxHashMap;

pub(crate) use ryo_operators::profile as operators;

#[path = "finish/plan_load.rs"]
mod plan_load;
#[path = "finish/plan_args.rs"]
mod plan_args;
#[path = "finish/plan_select.rs"]
mod plan_select;
pub(crate) use plan_load::from_typed;
use plan_args::{Args, Param, s};
use std::cell::RefCell;
use std::rc::Rc;

/// 7: a mechanism switch (`VEC_*` in the environment), read once a run at its call site, not once a step.
macro_rules! sw {
    ($n:literal) => {{
        static S: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *S.get_or_init(|| std::env::var_os($n).is_some())
    }};
}

/// 8: the per-step thread clock (a system call each read) only under `VEC_STEP_TIME`; else 0 (the stats' time column empty).
/// redo-int: one definition for engine-vec's and redo-union pass 2's step clock (the same switch in both).
#[inline]
pub(super) fn step_clock() -> f64 {
    // 9: `VEC_STEP_TICKS`: the monotonic clock (vDSO, no system call), for a per-step split that the clock does not distort
    if sw!("VEC_STEP_TICKS") { return select_kernel::ticks(); }
    if sw!("VEC_STEP_TIME") { super::thread_cpu() } else { 0.0 }
}

#[allow(dead_code, reason = "file, native-reference and project hosts read different request fields")]
#[path = "json_input.rs"]
pub(crate) mod json_input;
#[allow(dead_code, reason = "benchmark hosts select lint locations or full query output")]
#[path = "json_output.rs"]
pub(crate) mod json_output;
#[path = "finish/symbol_predicate.rs"]
mod symbol_predicate;
use symbol_predicate::{Bound, TextTest};

#[path = "finish/file_version.rs"]
mod file_version;
pub(super) use file_version::FileVersion;

#[path = "finish/answer.rs"]
pub(super) mod answer;
#[path = "finish/recycle.rs"]
mod recycle;
#[path = "finish/inline_key.rs"]
mod inline_key;
#[path = "finish/col/mod.rs"]
mod col;
#[path = "finish/select_kernel.rs"]
mod select_kernel;
#[path = "band.rs"]
pub(super) mod band;
#[path = "walk.rs"]
mod walk;
#[path = "filter_order.rs"]
mod filter_order;
#[path = "finish/union_kernel.rs"]
mod union_kernel;
#[path = "finish/antijoin_kernel.rs"]
mod antijoin_kernel;
#[path = "tree_class.rs"]
mod tree_class;
#[path = "finish/frame.rs"]
mod frame;
#[path = "finish/known.rs"]
mod known;
#[path = "finish/set_ops.rs"]
mod set_ops;
#[path = "finish/empty_why.rs"]
pub(super) mod empty_why;
#[path = "finish/empty_sel.rs"]
pub(super) mod empty_sel;
#[path = "prefixes.rs"]
mod prefixes;
#[path = "finish/ordered_dfs.rs"]
mod ordered_dfs;
#[path = "finish/worklist.rs"]
mod worklist;
#[path = "finish/lazy_view.rs"]
mod lazy_view;
#[path = "finish/fill_len.rs"]
pub(super) mod fill_len;
#[path = "finish/host.rs"]
pub(super) mod host;

#[path = "finish/capture.rs"]
mod capture;
#[path = "finish/fill_prof.rs"]
pub(super) mod fill_prof;
#[path = "finish/view_cols.rs"]
mod view_cols;
/// redo-empty: emptiness for the file, by propagation over the plan.
#[path = "finish/emptyprop.rs"]
pub(super) mod emptyprop;
#[path = "ordering.rs"]
mod ordering;
#[path = "finish/leaf_scan.rs"]
mod leaf_scan;
#[path = "finish/cold_bucket.rs"]
mod cold_bucket;
#[path = "finish/view_join/probe.rs"]
mod view_probe;
#[path = "finish/view_join/choice.rs"]
mod view_choice;
#[path = "finish/view_join/key.rs"]
mod view_key;
#[path = "finish/view_join/pair.rs"]
mod view_pair;
#[path = "finish/view_join/entry.rs"]
mod view_entry;
#[path = "finish/view_join/input.rs"]
mod view_input;
#[path = "finish/cold_text.rs"]
mod cold_text;
#[path = "finish/cold_list.rs"]
mod cold_list;
#[path = "finish/cold_entries.rs"]
mod cold_entries;
#[path = "finish/cold_test.rs"]
mod cold_test;
#[path = "finish/cold_pred.rs"]
mod cold_pred;
#[path = "finish/cold_word.rs"]
mod cold_word;
#[path = "finish/keyed.rs"]
mod keyed;

#[allow(dead_code, reason = "file-only benchmark binaries share the executor modules")]
#[path = "finish/session.rs"]
pub(super) mod session;
#[allow(dead_code, reason = "file-only benchmark binaries have no project frontier")]
#[path = "finish/population.rs"]
pub(super) mod population;

#[allow(dead_code, reason = "file-only benchmark binaries have no project frontier")]
#[path = "finish/subject_scope.rs"]
mod subject_scope;
#[allow(dead_code)]
#[path = "finish/partition.rs"]
pub(super) mod partition;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, serde::Deserialize, serde::Serialize)]
pub(super) enum Ty {
    Node,
    Sym,
    File,
    Span,
    Num,
    Flag,
}

pub(super) fn ty_of(k: CellKind) -> Ty {
    match k {
        CellKind::Node => Ty::Node,
        CellKind::Symbol => Ty::Sym,
        CellKind::File => Ty::File,
        CellKind::Span => Ty::Span,
        CellKind::Number => Ty::Num,
        CellKind::Flag => Ty::Flag,
    }
}

/// A value: a typed cell.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub(super) struct V(Ty2, u64);
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
enum Ty2 {
    T(u8),
}
impl V {
    fn new(t: Ty, v: u64) -> Self {
        V(Ty2::T(t as u8), v)
    }
    fn ty(self) -> Ty {
        let Ty2::T(t) = self.0;
        [Ty::Node, Ty::Sym, Ty::File, Ty::Span, Ty::Num, Ty::Flag][t as usize]
    }
}

pub(super) const NULL: u64 = u64::MAX;
/// redo-alloc: a placeholder cell for an argument array's unused places (never read).
const NULL_V: V = V(Ty2::T(5), NULL);

/// 7: a table's column names and types, shared with its step's schema (never copied per table).
pub(super) type Names = Rc<Vec<String>>;
pub(super) type Tys = Rc<Vec<Ty>>;

/// A column table.
#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Table {
    pub(super) names: Names,
    pub(super) tys: Tys,
    pub(super) cols: Vec<Vec<u64>>,
    pub(super) n: usize,
}

thread_local! {
    /// 8: the worker's column buffers: a dropped table's columns come back here (cleared, capacity kept) and the next
    /// table takes them, so a file's executions reuse the same memory instead of allocating and freeing per execution.
    /// redo-alloc: by size class (a column of room 2^c..2^(c+1) in class c), so a small table never holds a large column.
    static POOL: RefCell<([Vec<Vec<u64>>; CLASSES], usize)> = const { RefCell::new(([const { Vec::new() }; CLASSES], 0)) };
}
/// 8: the pool keeps at most this many cells in all (32 MB), and no single column above a sixteenth of it (redo-alloc c8
/// tried 64 MB: no fewer misses, 37 MB more live).
const POOL_CELLS: usize = 1 << 22;
/// redo-alloc: size classes of the pool (2^0 .. 2^19 cells: the largest pooled column is POOL_CELLS / 16 = 2^18).
const CLASSES: usize = 20;

/// redo-alloc: a pooled column with room for `n` cells (no more than 32 times that), or a new one with the requested
/// capacity; for an unknown size (`n` 0) one of the small classes (up to 512
/// cells) or an empty one.
#[inline]
fn take_col(p: &mut ([Vec<Vec<u64>>; CLASSES], usize), n: usize) -> Vec<u64> {
    let (lo, hi) = if n == 0 { (0, 8) } else { let c = (usize::BITS - (n - 1).leading_zeros()) as usize; (c.saturating_sub(1), (c + 4).min(CLASSES - 1)) };
    for c in lo..=hi.min(CLASSES - 1) {
        if let Some(v) = p.0[c].pop() {
            p.1 -= v.capacity();
            if v.capacity() >= n {
                return v;
            }
            // a column just below the room asked: grown once here
            let mut v = v;
            v.reserve_exact(n);
            return v;
        }
    }
    Vec::with_capacity(n)
}

impl Drop for Table {
    fn drop(&mut self) {
        self.release();
        // redo-alloc: the emptied list of columns kept too (recycle.rs)
        recycle::give_cols(std::mem::take(&mut self.cols));
    }
}

impl Table {
    pub(super) fn retained(names: Names, tys: Tys, rows: usize) -> Self {
        let cols = (0..names.len()).map(|_| Vec::with_capacity(rows)).collect();
        Self { names, tys, cols, n: 0 }
    }

    fn with(names: impl Into<Names>, tys: impl Into<Tys>) -> Self {
        Self::with_rows(names, tys, 0)
    }
    /// redo-alloc: `with` for a table about to hold `n` rows (its columns taken with that room; 0: unknown).
    pub(super) fn with_rows(names: impl Into<Names>, tys: impl Into<Tys>, n: usize) -> Self {
        let (names, tys) = (names.into(), tys.into());
        let k = names.len();
        // redo-alloc: the list of columns from the kept ones (recycle.rs), each column from the pool
        let mut cols = recycle::take_cols(k);
        POOL.with(|p| {
            let p = &mut *p.borrow_mut();
            cols.extend((0..k).map(|_| take_col(p, n)));
        });
        Table { names, tys, cols, n: 0 }
    }
    /// redo-alloc: a copy of the table in the worker's pooled list and columns (`clone` makes both new).
    pub(super) fn copy(&self) -> Table {
        let mut cols = recycle::take_cols(self.cols.len());
        POOL.with(|p| {
            let p = &mut *p.borrow_mut();
            for c in &self.cols {
                let mut v = take_col(p, c.len());
                v.extend_from_slice(c);
                cols.push(v);
            }
        });
        Table { names: self.names.clone(), tys: self.tys.clone(), cols, n: self.n }
    }
    /// redo-alloc: a column of `n` cells `x` appended, its room from the pool.
    pub(super) fn push_const_col(&mut self, x: u64, n: usize) {
        let mut v = POOL.with(|p| take_col(&mut p.borrow_mut(), n));
        v.resize(n, x);
        self.cols.push(v);
    }
    /// 8: the table's columns back in the pool (cleared, capacity kept); the table is left with no rows and no columns.
    pub(super) fn release(&mut self) {
        self.n = 0;
        if self.cols.is_empty() {
            return;
        }
        let _ = POOL.try_with(|p| {
            let (p, cells) = &mut *p.borrow_mut();
            for mut c in self.cols.drain(..) {
                let cap = c.capacity();
                if cap == 0 || cap > POOL_CELLS / 16 || *cells + cap > POOL_CELLS {
                    continue;
                }
                c.clear();
                *cells += cap;
                p[(usize::BITS - 1 - cap.leading_zeros()) as usize].push(c);
            }
        });
    }
    pub(super) fn col(&self, name: &str) -> Result<usize, String> {
        self.names.iter().position(|c| c == name).ok_or_else(|| format!("no column {name} in {:?}", self.names))
    }
    fn push_row(&mut self, row: &[u64]) {
        if self.cols.len() < self.names.len() {
            self.cols.resize(self.names.len(), Vec::new());
        }
        for (c, v) in row.iter().enumerate() {
            self.cols[c].push(*v);
        }
        self.n += 1;
    }
    fn row(&self, r: usize, out: &mut Vec<u64>) {
        out.clear();
        out.extend(self.cols.iter().map(|c| c[r]));
    }
}

/// Schema preparation can share compiled patterns across host declarations.
/// Predicate answers still belong to the dictionary of the individual run.
#[derive(Clone, Default)]
pub(super) struct Patterns(Rc<RefCell<FxHashMap<String, Rc<regex::bytes::Regex>>>>);

/// The run: texts, regexes, kind and field names, file paths.
#[derive(Default)]
pub(super) struct Run {
    /// Relations whose provider exposes a population other than this native file table.
    pub(super) reader_tables: rustc_hash::FxHashSet<String>,
    cold: cold_list::Plan,
    cold_entries: cold_entries::Audit,
    cold_text: cold_text::Plan,
    pub(super) texts: Vec<Rc<[u8]>>,
    pub(super) ids: FxHashMap<Rc<[u8]>, u32>,
    regexes: FxHashMap<String, Rc<Bound<Rc<regex::bytes::Regex>>>>,
    compiled_patterns: Option<Patterns>,
    glob_lists: FxHashMap<Vec<u32>, TextTest>,
    lower_symbols: FxHashMap<u32, u32>,
    symbol_sets: RefCell<FxHashMap<Box<[u64]>, Rc<symbol_predicate::SymbolSet>>>,
    folded_literals: FxHashMap<u32, Rc<Bound<Rc<[u8]>>>>,
    prepared_globs: FxHashMap<u32, Rc<symbol_predicate::PreparedGlob>>,
    pub(super) kind_sym: Vec<u32>,
    pub(super) field_sym: Vec<u32>,
    /// The provider updates its field-code dictionary when reading each file.
    pub(super) file_field_codes: bool,
    pub(super) paths: std::sync::Arc<Vec<String>>,
    answer_words: Option<std::sync::Arc<std::sync::RwLock<answer::Words>>>,
    pub(super) empty: u32,
    /// The SDK symbol without a published value, distinct from an empty string.
    pub(super) absent_word: Option<u32>,
    /// Provider symbols with a diagnostic name but no text for a scalar comparison.
    pub(super) unresolved_words: Vec<u32>,
    /// redo-general: the provider's token order (`token-link`, read in place) and, by kind code, whether an element of the
    /// kind carries its source text (`syntax-tree`'s `text`), both as the provider states them (`TokenOrder`).
    pub(super) tokens: Option<Rc<dyn TokenOrder>>,
    pub(super) text_kind: Vec<bool>,
    pub(super) rels: FxHashMap<String, (Option<languages::Relation>, Names, Tys)>,
    /// redo-fills pass 2: the relations the run's suppliers declared (`declare`).
    pub(super) declared: FxHashMap<String, Declared>,
    pub(super) link_sym: [u32; 3],
    /// The project's root on disk (the host's, for its file facts).
    pub(super) root_dir: String,
    /// Supplied files carry the caller's observation instead of a disk path to stat.
    pub(super) supplied_permission: Option<(u32, ryo_values::domain::files::types::ExecutablePermission)>,
    /// redo-general: the answers of the relations the provider answers for a project (`project.rs`), kept for the run.
    /// Exact refusal returned by the current provider call on bound inputs.
    pub(super) provider_refusal: Option<String>,
    /// Functions a step asked that are not in the step set (the rule is refused).
    pub(super) missing: std::collections::BTreeSet<String>,
    /// 6h: the provider relations read in place (`Native::Fact`), with their column counts.
    pub(super) fact_rels: Vec<(String, usize)>,
    /// pass 2: the kind codes of each kind name's symbol (made at the first ask, after `kind_sym` is set).
    pub(super) kind_codes: std::cell::OnceCell<FxHashMap<u32, Box<[u16]>>>,
    /// redo-fills c2: the run-wide symbol of each provider word: the run's base words (the same in every file) and this
    /// file's own words (`own_words_cleared` at each file's start); `u32::MAX`: not seen yet. A fill's text cells become
    /// symbols by one array read, not a text copy and a hash a cell.
    pub(super) base_word_sym: Vec<u32>,
    pub(super) own_word_sym: Vec<u32>,
    /// redo-fills pass 2: the symbol of each closed-set word (`cell_text`'s blocks, by number from 0xFFFF_0000).
    pub(super) closed_word_sym: Vec<u32>,
}

impl Run {
    pub(super) fn links(&mut self) {
        self.link_sym = [self.sym(b"next-token"), self.sym(b"matching-delimiter"), self.sym(b"enclosing-delimiter")];
    }
    pub(super) fn sym(&mut self, t: &[u8]) -> u32 {
        if let Some(i) = self.ids.get(t) {
            return *i;
        }
        let i = self.texts.len() as u32;
        let text = Rc::<[u8]>::from(t);
        self.texts.push(Rc::clone(&text));
        self.ids.insert(text, i);
        i
    }
    fn lower_symbol(&mut self, symbol: u32) -> u32 {
        if let Some(&lower) = self.lower_symbols.get(&symbol) {
            return lower;
        }
        let text = String::from_utf8_lossy(self.text(symbol)).to_lowercase();
        let lower = self.sym(text.as_bytes());
        self.lower_symbols.insert(symbol, lower);
        lower
    }
    pub(crate) fn text(&self, s: u32) -> &[u8] {
        &self.texts[s as usize]
    }
    pub(super) fn schema_run(patterns: &Patterns) -> Self {
        Self { compiled_patterns: Some(patterns.clone()), ..Self::default() }
    }

    fn re(&mut self, p: &str) -> Result<Rc<Bound<Rc<regex::bytes::Regex>>>, String> {
        if let Some(r) = self.regexes.get(p) {
            return Ok(r.clone());
        }
        let compile = || regex::bytes::Regex::new(p).map(Rc::new).map_err(|e| format!("regex {p}: {e}"));
        let pattern = if let Some(cache) = &self.compiled_patterns {
            let mut patterns = cache.0.borrow_mut();
            if let Some(pattern) = patterns.get(p) {
                Rc::clone(pattern)
            } else {
                let pattern = compile()?;
                patterns.insert(p.to_owned(), Rc::clone(&pattern));
                pattern
            }
        } else {
            compile()?
        };
        let r = Rc::new(Bound::new(pattern));
        self.regexes.insert(p.to_owned(), r.clone());
        Ok(r)
    }
    /// A relation's catalog entry: the reader's relation when the provider's relations catalog has it (None: a relation
    /// the provider declared at the run's start, `declare`), its columns and their types.
    pub(super) fn catalog(&mut self, name: &str) -> Option<(Option<languages::Relation>, Names, Tys)> {
        if let Some(r) = self.rels.get(name) {
            return Some(r.clone());
        }
        if let Some(d) = self.declared.get(name) {
            let r = (None, d.names.clone(), d.tys.clone());
            self.rels.insert(name.to_owned(), r.clone());
            return Some(r);
        }
        let relation = catalog::relation_named(NameRef(name))?;
        let names: Names = Rc::new(catalog::columns(relation).into_iter().map(|c| catalog::column_name(c).0.to_owned()).collect());
        let tys: Tys = Rc::new(catalog::kinds(relation).iter().map(|k| ty_of(*k)).collect());
        self.rels.insert(name.to_owned(), (Some(relation), names.clone(), tys.clone()));
        Some((Some(relation), names, tys))
    }

    /// redo-fills pass 2: a relation the run's suppliers declare (the provider's own facts beyond its relations catalog, the
    /// host's, the executor's steps), with its columns and who supplies its rows. A relation not declared here and in the
    /// provider's relations catalog is the provider's.
    pub(super) fn declare(&mut self, name: &str, cols: &[(&str, Ty)], supplier: Supplier) {
        let names: Names = Rc::new(cols.iter().map(|c| c.0.to_owned()).collect());
        let tys: Tys = Rc::new(cols.iter().map(|c| c.1).collect());
        self.rels.remove(name);
        self.declared.insert(name.to_owned(), Declared { names, tys, supplier });
    }

    /// Who supplies `rel`'s rows (the provider unless declared otherwise).
    pub(super) fn supplier(&self, rel: &str) -> Supplier {
        self.declared.get(rel).map_or(Supplier::Provider, |d| d.supplier)
    }
}

/// redo-general: the provider's token order of a file, the `token-link` relation's rows as events (the executor keeps them
/// in place by position): each token in order, then the links it takes part in, in the catalog's link kinds.
pub(super) trait TokenOrder {
    /// Whether an element of kind `kind` carries its source text (the others' `text` is empty).
    fn has_text(&self, kind: u16) -> bool;
    /// The file's tokens in order and their links.
    fn links(&self, table: &NodeTable, each: &mut dyn FnMut(TokenEvent));
}

/// One step of a file's token order: a token, or one of its links (source, target).
#[derive(Clone, Copy)]
pub(super) enum TokenEvent {
    Token(u32),
    /// `next-token`: the token after the source.
    Next(u32, u32),
    /// `matching-delimiter`: from an opener to its closer.
    Matching(u32, u32),
    /// `enclosing-delimiter`: from a token to the opener of its innermost open delimiter.
    Enclosing(u32, u32),
}

impl Run {
    /// redo-general: the provider's token order, set once a run.
    pub(super) fn set_tokens(&mut self, tokens: Rc<dyn TokenOrder>, kinds: usize) {
        self.text_kind = (0..kinds).map(|k| tokens.has_text(k as u16)).collect();
        self.tokens = Some(tokens);
    }
}

/// redo-fills pass 2: who supplies a relation's rows, as the run's catalog declares it.
#[derive(Clone, Copy)]
pub(super) enum Supplier {
    /// The language provider (`Provider::fill`), for a scope.
    Provider,
    /// The host, for a file (the file system's facts: `host::host_fill`).
    Host,
    /// The host, for the run: the files of the run (one row a file, the same in every file).
    #[allow(dead_code, reason = "redo-int round 33: no relation is declared RunFiles since filesystem.file is the file's row (redo-binding c32); kept as redo-fills' run-scoped supplier")]
    RunFiles,
}

/// A declared relation.
#[derive(Clone)]
pub(super) struct Declared {
    names: Names,
    tys: Tys,
    supplier: Supplier,
}

/// 6f probe: a made table's rows as text lines (`label`, path, then `column=value` cells; a node as `id:line`).
pub(super) fn dump_rows(f: &File<'_>, run: &Run, t: &Table, label: &str, path: &str, out: &mut String) {
    use std::fmt::Write as _;
    for r in 0..t.n {
        let _ = write!(out, "{label}\t{path}");
        for c in 0..t.names.len() {
            let v = t.cols[c][r];
            if t.tys[c] == Ty::File {
                continue;
            }
            let _ = write!(out, "\t{}=", t.names[c]);
            if v == NULL {
                out.push_str("null");
                continue;
            }
            match t.tys[c] {
                Ty::Node => {
                    let line = if (v as usize) < f.n { f.line(f.start[v as usize]) } else { 0 };
                    let _ = write!(out, "{v}:{line}");
                }
                Ty::Sym => out.push_str(&String::from_utf8_lossy(run.text(v as u32)).replace(['\t', '\n'], " ")),
                Ty::Span => {
                    let _ = write!(out, "{}-{}", v >> 32, v & 0xffff_ffff);
                }
                _ => {
                    let _ = write!(out, "{}", v as i64);
                }
            }
        }
        out.push('\n');
    }
}

/// A plan node, as lowered.
pub(super) struct PNode {
    args: std::sync::OnceLock<Args>,
    pub(super) inputs: plan_load::StepInputs,
}

pub(super) struct Plan {
    frames: std::sync::OnceLock<Vec<frame::Frame>>,
    consumers: std::sync::OnceLock<Vec<Vec<usize>>>,
    pub(super) nodes: Vec<PNode>,
    pub(super) emits: Vec<(String, usize, String, Vec<String>)>,

}

impl Plan {
    fn frame(&self, node: usize) -> frame::Frame {
        self.frames.get_or_init(|| frame::frames(self))[node]
    }

    fn consumers(&self, node: usize) -> &[usize] {
        &self.consumers.get_or_init(|| {
            let mut uses = vec![Vec::new(); self.nodes.len()];
            for (index, step) in self.nodes.iter().enumerate() {
                for &input in &step.inputs {
                    if uses[input].last() != Some(&index) {
                        uses[input].push(index);
                    }
                }
            }
            uses
        })[node]
    }
}

/// Visit typed extension values in their declared order.
fn extend_values(args: &Args, mut visit: impl FnMut(&str, &Param) -> R<()>) -> R<()> {
    let values = args["values"].as_object().ok_or("extend values")?;
    for name in strs(&args["value-order"])? {
        visit(name, values.get(name).ok_or("undeclared extend column")?)?;
    }
    Ok(())
}

/// The relations of the tree read in place, by position.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub(super) enum Native {
    /// `syntax-tree` / `semantic-syntax`: a row an element.
    Tree,
    /// `semantic-child` / `syntax-child`: a row an element with a parent.
    Child,
    /// `source-location`: a row an element.
    Location,
    /// `token-link`: a row a (source token, link kind), from the token order; row = source * 3 + kind.
    Token,
    /// 6h: a provider relation read in place by key: the file's fill, never copied into a step; a probe on any column reads
    /// that column's keyed list (made from the fill on the first ask in a file), and `col = literal` enters by its bucket.
    /// The number is the relation's place in `Run::fact_rels`.
    Fact(u16),
}

/// 6h: the rows of the keyed lists made from provider fills (one entry a fill row a column asked), for the report.
pub(super) static FACT_IX_ROWS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

const TREE_COLS: [(&str, Ty); 5] = [("id", Ty::Node), ("kind", Ty::Sym), ("text", Ty::Sym), ("file", Ty::File), ("span", Ty::Span)];
const CHILD_COLS: [(&str, Ty); 5] = [("parent", Ty::Node), ("child", Ty::Node), ("ordinal", Ty::Num), ("field", Ty::Sym), ("file", Ty::File)];
const LOC_COLS: [(&str, Ty); 9] = [
    ("id", Ty::Node),
    ("file", Ty::File),
    ("span", Ty::Span),
    ("start-line", Ty::Num),
    ("end-line", Ty::Num),
    ("start-byte", Ty::Num),
    ("end-byte", Ty::Num),
    ("start-column", Ty::Num),
    ("end-column", Ty::Num),
];
const TOKEN_COLS: [(&str, Ty); 5] = [("source", Ty::Node), ("target", Ty::Node), ("link-kind", Ty::Sym), ("file", Ty::File), ("span", Ty::Span)];

fn native_of(run: &Run, rel: &str) -> Option<Native> {
    if run.reader_tables.contains(rel) { return None; }
    match rel {
        "syntax-tree" | "semantic-syntax" => Some(Native::Tree),
        "semantic-child" | "syntax-child" => Some(Native::Child),
        "source-location" => Some(Native::Location),
        "token-link" if run.tokens.is_some() => Some(Native::Token),
        _ => None,
    }
}

fn native_cols(n: Native) -> &'static [(&'static str, Ty)] {
    match n {
        Native::Tree => &TREE_COLS,
        Native::Child => &CHILD_COLS,
        Native::Location => &LOC_COLS,
        Native::Token => &TOKEN_COLS,
        Native::Fact(_) => &[],
    }
}

/// 6h: a provider relation's place among the facts read in place (`VEC_NO_FACTVIEW` turns this off): every catalog
/// relation the provider or the host fills per file; never the run's file list or a relation a step makes.
fn fact_of(run: &mut Run, rel: &str) -> Option<Native> {
    if sw!("VEC_NO_FACTVIEW") || native_of(run, rel).is_some() || matches!(run.supplier(rel), Supplier::RunFiles) {
        return None;
    }
    let (_, names, _) = run.catalog(rel)?;
    let at = match run.fact_rels.iter().position(|(r, _)| r == rel) {
        Some(i) => i,
        None => {
            run.fact_rels.push((rel.to_owned(), names.len()));
            run.fact_rels.len() - 1
        }
    };
    Some(Native::Fact(at as u16))
}

const NONE: u32 = u32::MAX;

/// The token order's links, one pass a file, made the first time a step reads `token-link` in that file: next token,
/// matching delimiter (both ways) and enclosing delimiter by position, and the tokens each opener encloses.
pub(super) struct Tokens {
    next: Vec<u32>,
    prev: Vec<u32>,
    pair: Vec<u32>,
    opener: Vec<bool>,
    enc: Vec<u32>,
    enclosed: FxHashMap<u32, inline_key::List<u32>>,
    order: Vec<u32>,
}

/// One file: the node table's columns, the source, the fills and the made tables.
pub(super) struct File<'a> {
    cold: cold_list::PerFile,
    keyed_sizes: FxHashMap<usize, usize>,
    owed_fills: Vec<bool>,
    version: &'a FileVersion<'a>,
    cold_text: cold_text::Dictionary,
    after_last_file: bool,
    project_inputs: Option<&'a population::Inputs>,
    memo: Vec<Option<Res>>,
    vrows: cold_entries::Rows,
    vmat: FxHashMap<usize, Rc<Table>>,
    cursor_views: rustc_hash::FxHashSet<usize>,
    index: FxHashMap<usize, inline_key::List<(inline_key::Key, Rc<Index>)>>,
    fills: FxHashMap<String, Rc<Table>>,
    lazies: FxHashMap<usize, Rc<Lazy>>,
    /// 6h: the fills read in place, by `Native::Fact` place, and their keyed lists by (place, column).
    facts: Vec<Option<Rc<Table>>>,
    fact_ix: RefCell<FxHashMap<(u16, usize), Rc<Index>>>,
    /// redo-union pass 2: whether a step is known empty in this file from its kinds and per-file tests (`known`).
    known: Vec<u8>,
    /// pass 2: the steps whose memo entry `res` set: cleared at the file's end, and the memo (and `known`) kept for the
    /// next file (no 30 k-entry table made and dropped a file).
    touched: Vec<u32>,
    /// redo-fills (`VEC_FILL_COUNT`): per fact slot, whether a step read the fill's rows (not only its size).
    fact_read: Vec<std::cell::Cell<bool>>,
}

thread_local! {
    static SPARE: RefCell<(Vec<Option<Res>>, Vec<u8>, Vec<u32>)> = const { RefCell::new((Vec::new(), Vec::new(), Vec::new())) };
    /// redo-alloc: the last file's buffers and maps, emptied (capacity kept), for the next file.
    static SPARE_BUFS: RefCell<Option<FileBufs>> = const { RefCell::new(None) };
}

/// redo-alloc: a file's own buffers and maps, emptied at the file's end and handed to the next file (no per-file make
/// and free of each; their answers given back to the pools, recycle.rs).
#[derive(Default)]
struct FileBufs {
    vrows: cold_entries::Rows,
    vmat: FxHashMap<usize, Rc<Table>>,
    index: FxHashMap<usize, inline_key::List<(inline_key::Key, Rc<Index>)>>,
    fills: FxHashMap<String, Rc<Table>>,
    lazies: FxHashMap<usize, Rc<Lazy>>,
    facts: Vec<Option<Rc<Table>>>,
    fact_ix: FxHashMap<(u16, usize), Rc<Index>>,
}

/// A map's tables given back (recycle.rs) and the map emptied, its room kept.
fn give_tables<K>(m: &mut FxHashMap<K, Rc<Table>>) {
    for (_, t) in m.drain() {
        recycle::recycle(t);
    }
}

impl Drop for File<'_> {
    fn drop(&mut self) {
        if fill_prof::counting() {
            for (i, t) in self.facts.iter().enumerate() {
                if t.is_some() {
                    fill_prof::fact_filled(i, self.fact_read.get(i).is_some_and(std::cell::Cell::get));
                }
            }
        }
        let mut memo = std::mem::take(&mut self.memo);
        let mut known = std::mem::take(&mut self.known);
        let mut touched = std::mem::take(&mut self.touched);
        for n in touched.drain(..) {
            // redo-alloc: the answer given back for the next (recycle.rs), not freed
            if let Some(Res::Tab(t)) = memo[n as usize].take() {
                recycle::recycle(t);
            }
        }
        // a substitution's restore never leaves an entry `res` did not set (checked where it is cheap)
        debug_assert!(memo.iter().all(Option::is_none));
        known.fill(0);
        SPARE.with(|s| *s.borrow_mut() = (memo, known, touched));
        // redo-alloc: the file's buffers and maps emptied and kept for the next file
        let mut b = FileBufs {
            vrows: std::mem::take(&mut self.vrows),
            vmat: std::mem::take(&mut self.vmat),
            index: std::mem::take(&mut self.index),
            fills: std::mem::take(&mut self.fills),
            lazies: std::mem::take(&mut self.lazies),
            facts: std::mem::take(&mut self.facts),
            fact_ix: std::mem::take(self.fact_ix.get_mut()),
        };
        for (_, r) in b.vrows.drain() {
            r.recycle();
        }
        give_tables(&mut b.vmat);
        // redo-alloc: the indexes given back (their own allocation and buffers kept)
        for (_, mut v) in b.index.drain() {
            v.drain_each(|(_, ix)| recycle_index(ix));
        }
        give_tables(&mut b.fills);
        b.lazies.clear();
        for t in b.facts.drain(..).flatten() {
            recycle::recycle(t);
        }
        b.fact_ix.clear();
        // a map grown far past a usual file is not kept (its room would stay with the worker)
        if b.vrows.capacity() > 1 << 12 || b.index.capacity() > 1 << 12 || b.fact_ix.capacity() > 1 << 12 {
            return;
        }
        SPARE_BUFS.with(|s| *s.borrow_mut() = Some(b));
    }
}

/// redo-fills pass 2: what a provider's answer covers: one file of the run, or a project (a set of the run's files) for a
/// relation whose rows relate files to each other. Cells name files by the run's file number, never "this file".
#[derive(Clone, Copy, Debug)]
pub(super) enum Scope<'s> {
    File(u32),
    Files(&'s [u32]),
}

/// How a step asks the provider for a relation's rows: the visit's facts.
pub(super) trait Provider {
    /// Parser-supplied text symbols in node-position order, when the parse carries them directly.
    fn syntax_text(&self) -> Option<&[u32]> { None }

    /// Every row of `rel` in `scope`, each cell as the run names it (a file cell is the run's number of the file it names).
    fn fill(&mut self, run: &mut Run, rel: &str, scope: Scope<'_>) -> Result<Table, String>;
    fn shared_fill(&mut self, run: &mut Run, rel: &str, scope: Scope<'_>) -> Result<Rc<Table>, String> {
        self.fill(run,rel,scope).map(recycle::rc)
    }
    /// redo-fills pass 2: how many rows `fill(rel, scope)` answers, where known without making them; None: not known.
    fn fill_len(&mut self, _run: &mut Run, _rel: &str, _scope: Scope<'_>) -> Option<usize> {
        None
    }
    /// A node's text in a syntax role: `Ok(None)` where the role cannot hold it.
    fn render(&mut self, node: u32, context: &str) -> Result<Option<Vec<u8>>, String>;
}

impl<'a> std::ops::Deref for File<'a> {
    type Target = FileVersion<'a>;
    fn deref(&self) -> &Self::Target { self.version }
}

impl File<'_> {
    #[inline]
    fn population(&self,node:usize) -> Option<&Rc<population::Population>> {
        let input=self.project_inputs?.get(&node)?;
        match &self.memo[node] {
            None => Some(input),
            Some(Res::Tab(t)) if input.is_materialized(t) => Some(input),
            _ => None,
        }
    }

    /// The reusable slots belong to this frame until its destructor returns them empty.
    fn remember(&mut self, node: usize, answer: Res) {
        if self.memo[node].replace(answer).is_none() {
            self.touched.push(node as u32);
        }
    }

    pub(super) fn new<'a>(version: &'a FileVersion<'a>, nodes: usize) -> File<'a> {
        let b = SPARE_BUFS.with(|s| s.borrow_mut().take()).unwrap_or_default();
        let mut f = File {
            version,
            after_last_file: false,
            project_inputs: None,
            memo: Vec::new(),
            vrows: b.vrows,
            cold: cold_list::PerFile::default(),
            keyed_sizes: FxHashMap::default(),
            owed_fills: Vec::new(),
            cold_text: cold_text::Dictionary::default(),
            vmat: b.vmat,
            cursor_views: Default::default(),
            index: b.index,
            fills: b.fills,
            lazies: b.lazies,
            facts: b.facts,
            fact_ix: RefCell::new(b.fact_ix),
            known: Vec::new(),
            touched: Vec::new(),
            fact_read: Vec::new(),
        };
        SPARE.with(|sp| {
            let (memo, known, touched) = std::mem::take(&mut *sp.borrow_mut());
            (f.memo, f.known, f.touched) = if memo.len() == nodes && known.len() == nodes { (memo, known, touched) } else { (vec![None; nodes], vec![0; nodes], Vec::new()) };
        });
        if sw!("VEC_COLD_COUNT") {
            let roots = version.extra_roots();
            cold_bucket::roots(roots.len(), roots.capacity());
        }
        f
    }

    fn has_text(run: &Run, k: u16) -> bool {
        run.text_kind.get(usize::from(k)).copied().unwrap_or(false)
    }
    /// The provider-decoded bytes, or the source spelling of a text-bearing element.
    fn text_bytes(&self, p: u32) -> &[u8] {
        match node_table::decoded_of(self.table, Position(p)) {
            Some(text) => text.0.as_bytes(),
            None => &self.src[self.start[p as usize] as usize..self.end[p as usize] as usize],
        }
    }
    fn elem_text(&self, run: &mut Run, p: u32) -> u32 {
        let mut memo = self.text_memo.borrow_mut();
        if let Some(&m) = memo.get(p as usize) {
            if m != u32::MAX { return m; }
        } else {
            memo.resize(self.n, u32::MAX);
        }
        let k = self.kind[p as usize];
        let s = if Self::has_text(run, k) {
            run.sym(self.text_bytes(p))
        } else {
            run.empty
        };
        memo[p as usize] = s;
        s
    }
    fn has_parent(&self, p: u32) -> bool {
        p != 0 && self.parent[p as usize] != p
    }
    fn toks(&self, run: &Run) -> Rc<Tokens> {
        if let Some(t) = &*self.tokens.borrow() {
            return t.clone();
        }
        let n = self.n;
        let mut t = Tokens { next: vec![NONE; n], prev: vec![NONE; n], pair: vec![NONE; n], opener: vec![false; n], enc: vec![NONE; n], enclosed: FxHashMap::default(), order: Vec::new() };
        if let Some(order) = &run.tokens {
            order.links(self.table, &mut |e| match e {
                TokenEvent::Token(at) => t.order.push(at),
                TokenEvent::Next(p, at) => {
                    t.next[p as usize] = at;
                    t.prev[at as usize] = p;
                }
                TokenEvent::Matching(open, at) => {
                    t.pair[open as usize] = at;
                    t.pair[at as usize] = open;
                    t.opener[open as usize] = true;
                }
                TokenEvent::Enclosing(at, e) => {
                    t.enc[at as usize] = e;
                    t.enclosed.entry(e).or_default().push(at);
                }
            });
        }
        let rc = Rc::new(t);
        *self.tokens.borrow_mut() = Some(rc.clone());
        rc
    }
    /// A native relation's cell.
    fn native_get(&self, run: &mut Run, nat: Native, row: u32, col: usize) -> u64 {
        fill_prof::read(0);
        fill_prof::read_at(match nat { Native::Tree => 0, Native::Child => 1, Native::Location => 2, Native::Token => 3, Native::Fact(_) => 4 }, col);
        let r = row as usize;
        match (nat, col) {
            (Native::Tree, 0) | (Native::Location, 0) => u64::from(row),
            (Native::Tree, 1) => u64::from(run.kind_sym[usize::from(self.kind[r])]),
            (Native::Tree, 2) => u64::from(self.elem_text(run, row)),
            (Native::Tree, 3) | (Native::Location, 1) | (Native::Child, 4) | (Native::Token, 3) => u64::from(self.file),
            (Native::Tree, 4) | (Native::Location, 2) => (u64::from(self.start[r]) << 32) | u64::from(self.end[r]),
            (Native::Child, 0) => u64::from(self.parent[r]),
            (Native::Child, 1) => u64::from(row),
            (Native::Child, 2) => u64::from(node_table::ordinal_of(self.table, Position(row)).0),
            (Native::Child, 3) => {
                let f = usize::from(node_table::field_of(self.table, Position(row)).0);
                u64::from(run.field_sym.get(f).copied().unwrap_or(run.empty))
            }
            (Native::Location, 3) => self.line(self.start[r]) as u64,
            (Native::Location, 4) => self.line(self.end[r]) as u64,
            (Native::Location, 5) => u64::from(self.start[r]),
            (Native::Location, 6) => u64::from(self.end[r]),
            (Native::Location, 7) => self.column(self.start[r]) as u64,
            (Native::Location, 8) => self.column(self.end[r]) as u64,
            (Native::Fact(i), c) => {
                if fill_prof::counting() {
                    self.mark_read(i);
                }
                self.facts[usize::from(i)].as_ref().map_or(NULL, |t| t.cols[c][r])
            }
            (Native::Token, _) => {
                let (src, k) = (row / 3, row % 3);
                match col {
                    0 => u64::from(src),
                    1 => {
                        let t = self.toks(run);
                        u64::from(match k {
                            0 => t.next[src as usize],
                            1 => t.pair[src as usize],
                            _ => t.enc[src as usize],
                        })
                    }
                    2 => u64::from(run.link_sym[k as usize]),
                    4 => (u64::from(self.start[src as usize]) << 32) | u64::from(self.end[src as usize]),
                    _ => NULL,
                }
            }
            _ => NULL,
        }
    }
    fn token_row(t: &Tokens, src: u32, k: u32) -> bool {
        let s = src as usize;
        match k {
            0 => t.next[s] != NONE,
            1 => t.opener[s],
            _ => t.enc[s] != NONE,
        }
    }
    /// redo-alloc: `native_rows` into `out` (empty).
    fn native_rows_into(&self, run: &Run, nat: Native, out: &mut Vec<u32>) {
        match nat {
            Native::Tree | Native::Location => out.extend(0..self.n as u32),
            Native::Child => out.extend((0..self.n as u32).filter(|p| self.has_parent(*p))),
            Native::Token => {
                let t = self.toks(run);
                for &p in &t.order {
                    for k in 0..3 {
                        if Self::token_row(&t, p, k) {
                            out.push(p * 3 + k);
                        }
                    }
                }
            }
            Native::Fact(i) => out.extend(0..self.facts[usize::from(i)].as_ref().map_or(0, |t| t.n as u32)),
        }
    }
    fn native_rows(&self, run: &Run, nat: Native) -> Vec<u32> {
        fill_prof::read(5);
        match nat {
            Native::Tree | Native::Location => (0..self.n as u32).collect(),
            Native::Child => (0..self.n as u32).filter(|p| self.has_parent(*p)).collect(),
            Native::Token => {
                let t = self.toks(run);
                let mut out = Vec::new();
                for &p in &t.order {
                    for k in 0..3 {
                        if Self::token_row(&t, p, k) {
                            out.push(p * 3 + k);
                        }
                    }
                }
                out
            }
            Native::Fact(i) => {
                if fill_prof::counting() {
                    self.mark_read(i);
                }
                (0..self.facts[usize::from(i)].as_ref().map_or(0, |t| t.n as u32)).collect()
            }
        }
    }
    fn mark_read(&self, i: u16) {
        if let Some(c) = self.fact_read.get(usize::from(i)) {
            c.set(true);
        }
    }
    /// 6h: a fill's keyed list on one column (made on the first ask in a file).
    fn fact_list(&self, i: u16, col: usize) -> Rc<Index> {
        if let Some(ix) = self.fact_ix.borrow().get(&(i, col)) {
            return ix.clone();
        }
        let m = match &self.facts[usize::from(i)] {
            Some(t) => {
                FACT_IX_ROWS.fetch_add(t.n as u64, std::sync::atomic::Ordering::Relaxed);
                Index::build(t.n, |r| t.cols[col][r])
            }
            None => Index::build(0, |_| 0),
        };
        let rc = Rc::new(m);
        self.fact_ix.borrow_mut().insert((i, col), rc.clone());
        rc
    }
    /// Whether a native relation is probed in place on a column.
    fn probes(nat: Native, col: usize) -> bool {
        matches!((nat, col), (Native::Tree | Native::Location, 0) | (Native::Child, 0 | 1) | (Native::Location, 3..=6) | (Native::Token, 0 | 1) | (Native::Fact(_), _))
    }
    /// Rows of a native relation whose column holds `v`.
    fn native_probe(&self, run: &Run, nat: Native, col: usize, v: u64, out: &mut Vec<u32>) {
        fill_prof::read(4);
        if sw!("VEC_COLD_COUNT") { run.cold.probe(self, None); }
        let ok = v < self.n as u64;
        match (nat, col) {
            (Native::Tree, 0) | (Native::Location, 0) => {
                if ok {
                    out.push(v as u32);
                }
            }
            (Native::Child, 1) => {
                if ok && self.has_parent(v as u32) {
                    out.push(v as u32);
                }
            }
            (Native::Child, 0) => {
                if ok {
                    for c in node_table::children_of(self.table, Position(v as u32)) {
                        if self.parent[c.0 as usize] == v as u32 && c.0 != 0 {
                            out.push(c.0);
                        }
                    }
                }
            }
            (Native::Location, 5) | (Native::Location, 6) => {
                let cell = if col == 5 { &self.by_start } else { &self.by_end };
                let mut b = cell.borrow_mut();
                let idx = b.get_or_insert_with(|| {
                    let src = if col == 5 { &self.start } else { &self.end };
                    Index::build(src.len(), |p| u64::from(src[p]))
                });
                if let Some(rows) = idx.get(&v) {
                    out.extend_from_slice(rows);
                }
            }
            (Native::Location, 3) | (Native::Location, 4) => {
                let mut b = self.by_line[col - 3].borrow_mut();
                let idx = b.get_or_insert_with(|| {
                    let src = if col == 3 { &self.start } else { &self.end };
                    Index::build(src.len(), |p| u64::from(self.line(src[p]) as u32))
                });
                if let Some(rows) = idx.get(&(u64::from(v as u32))) {
                    out.extend_from_slice(rows);
                }
            }
            (Native::Token, 0) => {
                if ok {
                    let t = self.toks(run);
                    for k in 0..3 {
                        if Self::token_row(&t, v as u32, k) {
                            out.push(v as u32 * 3 + k);
                        }
                    }
                }
            }
            (Native::Fact(i), c) => {
                if fill_prof::counting() {
                    self.mark_read(i);
                }
                if let Some(rows) = self.fact_list(i, c).get(&v) {
                    out.extend_from_slice(rows);
                }
            }
            (Native::Token, 1) => {
                if ok {
                    let t = self.toks(run);
                    let x = v as usize;
                    if t.prev[x] != NONE {
                        out.push(t.prev[x] * 3);
                    }
                    if t.pair[x] != NONE && !t.opener[x] {
                        out.push(t.pair[x] * 3 + 1);
                    }
                    if let Some(es) = t.enclosed.get(&(v as u32)) {
                        out.extend(es.iter().map(|e| e * 3 + 2));
                    }
                }
            }
            _ => {}
        }
    }
}

/// Counters a run keeps: per node, own time, calls, rows out and rows touched.
#[derive(Default)]
pub(super) struct Stats {
    pub(super) total_runs: u64,
    pub(super) total_rows: u64,
    pub(super) time: Vec<f64>,
    pub(super) rows: Vec<u64>,
    pub(super) calls: Vec<u64>,
    pub(super) touched: Vec<u64>,
    pub(super) empty: Vec<u64>,
    /// redo-union pass 2: runs (whole or by key) answered empty before the bracket (`known`), and the seconds of the
    /// bracketed runs that returned nothing.
    pub(super) known: Vec<u64>,
    pub(super) empty_time: Vec<f64>,
    /// pass 2 (`VEC_STEP_TIME`): seconds deciding the per-file static part (`known::static_empty_`, outermost calls).
    pub(super) static_time: f64,
    pub(super) static_depth: u32,
    pub(super) static_visits: u64,
    /// redo-whole pass 2: visits of steps with no leaf of their own (decided by their inputs alone).
    pub(super) static_pass: u64,
    pub(super) static_file_tests: u64,
    pub(super) rec_time: f64,
    pub(super) rec_busy: bool,
    /// redo-whole pass 2 (keyed.rs): runs by key of a step read in place of made whole, those that returned nothing, and
    /// the rows of tables a step builds (a table handed on from an input counts 0).
    pub(super) keyed: Vec<u64>,
    pub(super) keyed_empty: Vec<u64>,
    pub(super) built: Vec<u64>,
}

impl Stats {
    fn ran(&mut self, node: usize) {
        self.calls[node] += 1;
        self.total_runs += 1;
    }

    fn produced(&mut self, node: usize, rows: usize) {
        self.rows[node] += rows as u64;
        self.total_rows += rows as u64;
    }
}

// ------------------------------------------------------------------------------------------------ expressions

#[derive(Clone, Debug)]
enum E {
    Col(usize, usize, Ty),
    Lit(V),
    Text(u32),
    Num(i64),
    List(Vec<E>),
    Call(String, Vec<E>),
    Cmp(Op, Box<E>, Box<E>),
    In(Box<E>, Box<E>, bool),
    And(Box<E>, Box<E>),
    Or(Box<E>, Box<E>),
    Not(Box<E>),
    /// A text tested against a constant regex (from `matches` or `glob_any`), compiled once.
    TextTest(Box<E>, TextTest),
}

/// 8: a comparison operator, resolved once a run when the plan's expression is compiled (never compared as text per row).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(super) enum Op {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}
/// A node's column names and types, known from the plan before any file.
pub(super) struct Schema {
    names: Names,
    tys: Tys,
}

/// A select over one tree relation (or another such view) with only column picks and tests: never made whole; its rows are
/// read in place by position, entered by a kind bucket or a keyed probe, and tested as they are read.
pub(super) struct ViewSpec {
    base: Native,
    map: Vec<usize>,
    filter: Vec<E>,
    /// redo-fills c4: `filter` compiled once (when the view is made), read as columns (`view_cols`).
    tests: Vec<view_cols::Test>,
    cold_test: std::cell::OnceCell<Rc<cold_test::Kernel>>,
    cold_bucket: std::cell::OnceCell<cold_list::Handoff>,
    cold_file_test: bool,
}

impl ViewSpec {
    fn new(run: &Run, base: Native, map: Vec<usize>, filter: Vec<E>) -> Self {
        let tests = view_cols::compile(run, &filter);
        let cold_file_test = filter.iter().any(|e| filter_order::file_const(e) && !matches!(e, E::Lit(_)));
        ViewSpec { base, map, filter, tests, cold_file_test, cold_test: std::cell::OnceCell::new(), cold_bucket: std::cell::OnceCell::new() }
    }
}

/// What a node answers in a file.
#[derive(Clone)]
pub(super) enum Res {
    Tab(Rc<Table>),
    View(Rc<ViewSpec>),
}

/// A select compiled once a run.
struct SelPlan {
    srcs: plan_load::StepInputs,
    conj: Vec<E>,
    refs: Vec<u64>,
    items: Vec<(String, E)>,
}

/// One source of a select in a file.
enum Src {
    Tab(Rc<Table>, usize),
    View(Rc<ViewSpec>, usize),
    Base(Native),
    Lazy(Rc<Lazy>),
}

/// A closure that is never made whole: it answers its sources' reach when probed by either end.
pub(super) struct Lazy {
    keys: Vec<usize>,
    adj: FxHashMap<Vec<u64>, Vec<u64>>,
    radj: FxHashMap<Vec<u64>, Vec<u64>>,
    node: usize,
    names: Names,
    tys: Tys,
    cache: RefCell<FxHashMap<Vec<u64>, Rc<Vec<u64>>>>,
    rcache: RefCell<FxHashMap<Vec<u64>, Rc<Vec<u64>>>>,
}

fn truthy(v: V) -> bool {
    v.ty() == Ty::Flag && v.1 == 1
}

// ------------------------------------------------------------------------------------------------ the executor

pub(super) struct Exec<'p> {
    answer_layouts: FxHashMap<(usize, usize), std::sync::Arc<answer::Layout>>,
    pub(super) plan: &'p Plan,
    pub(super) stats: Stats,
    schemas: Vec<Option<Rc<Schema>>>,
    views: Vec<Option<Option<Rc<ViewSpec>>>>,
    sels: Vec<Option<Rc<SelPlan>>>,
    // Each reader group prepares its own column indices and interned names.
    select_preps: Vec<Option<Rc<select_kernel::Prep>>>,
    union_preps: Vec<Option<Rc<union_kernel::Prep>>>,
    antijoin_preps: Vec<Option<Rc<antijoin_kernel::Prep>>>,
    subject_scope: subject_scope::Scope,
    keyed_forms: Vec<Option<keyed::Form>>,
    band_shapes: FxHashMap<(usize, usize), Rc<band::Shape>>,
    leaves: emptyprop::Leaves,
    inner: f64,
    /// Steps whose inputs are all run-wide (the file list, written rows): made once a run.
    constant: Vec<Option<bool>>,
    run_memo: Vec<Option<Rc<Table>>>,
    emitted: rustc_hash::FxHashSet<usize>,
    pub(super) tally: band::Tally,
    /// 8: row-number buffers of the join (tuples a source), reused across executions; the join's key set, reused.
    u32_pool: Vec<Vec<u32>>,
    /// redo-union pass 2: what each step needs to return anything (`known`), and its shared empty answer.
    needs: Vec<Option<Rc<known::Need>>>,
    flat: known::Flat,
    empties: Vec<Option<Rc<Table>>>,
    /// The view rows a keyed read of a view finds (reused).
    scratch_rows: Vec<u32>,
    /// `VEC_STEP_TIME`: each step run under the step clock (`timed_body`).
    timing: bool,
    col: col::Kernels,
    cold_compiler: cold_test::Compiler,
}

type R<T> = Result<T, String>;

/// Borrow names from this plan argument in their declared order.
fn strs(j: &Param) -> R<impl DoubleEndedIterator<Item = &str> + Clone> {
    let values = match j {
        Param::List(items) => items.as_slice(),
        Param::Text(_) => std::slice::from_ref(j),
        Param::Missing => &[],
        _ => return Err(format!("not a list: {j:?}")),
    };
    if let Some(invalid) = values.iter().find(|item| item.as_str().is_none()) {
        return Err(format!("not a text: {invalid:?}"));
    }
    Ok(values.iter().filter_map(Param::as_str))
}

fn pos(names: &[String], c: &str) -> R<usize> {
    names.iter().position(|x| x == c).ok_or_else(|| format!("no column {c} in {names:?}"))
}

impl<'p> Exec<'p> {
    pub(super) fn new(plan: &'p Plan) -> Self {
        let k = plan.nodes.len();
        Exec {
            plan,
            stats: Stats { total_runs: 0, total_rows: 0, time: vec![0.0; k], rows: vec![0; k], calls: vec![0; k], touched: vec![0; k], empty: vec![0; k], known: vec![0; k], empty_time: vec![0.0; k], static_time: 0.0, static_depth: 0, static_visits: 0, static_pass: 0, static_file_tests: 0, rec_time: 0.0, rec_busy: false, keyed: vec![0; k], keyed_empty: vec![0; k], built: vec![0; k] },
            schemas: vec![None; k],
            views: vec![None; k],
            sels: vec![None; k],
            select_preps: vec![None; k],
            union_preps: vec![None; k],
            antijoin_preps: vec![None; k],
            subject_scope: Default::default(),
            keyed_forms: vec![None; k],
            band_shapes: FxHashMap::default(),
            leaves: emptyprop::Leaves::default(),
            inner: 0.0,
            constant: vec![None; k],
            answer_layouts: FxHashMap::default(),
            run_memo: vec![None; k],
            emitted: plan.emits.iter().map(|e| e.1).collect(),
            tally: band::Tally::default(),
            u32_pool: Vec::new(),
            needs: vec![None; k],
            flat: known::Flat::new(k),
            empties: vec![None; k],
            scratch_rows: Vec::new(),
            timing: sw!("VEC_STEP_TIME"),
            col: col::Kernels::default(),
            cold_compiler: Default::default(),
        }
    }

    #[allow(dead_code, reason = "only a project session keeps the executor across requests")]
    pub(super) fn release_request(&mut self) { self.run_memo.fill(None); }

    /// A closed population can hold several files. Its file equalities must
    /// survive both column-use analysis and preparation for execution.
    fn closed_inputs(&mut self) {
        self.views.fill(Some(None));
        self.constant.fill(Some(true));
    }

    /// Whether a step's rows are the same in every file: it reads only run-wide relations, through steps that read no file.
    fn is_constant(&mut self, run: &Run, node: usize) -> bool {
        if let Some(c) = self.constant[node] {
            return c;
        }
        let pn = &self.plan.nodes[node];
        let c = match pn.op() {
            // redo-binding c32 (filesystem.file per file) through redo-fills' catalog: it is declared a host relation of the
            // file (fact_catalog.rs), so only a RunFiles relation is constant, and none is declared now
            "base" => pn.base().is_some_and(|r| matches!(run.supplier(r), Supplier::RunFiles)),
            "rows" => true,
            "select" | "union" | "antijoin" | "extend" | "project" | "group-count" | "group-min" | "group-max" | "capture" | "replace" => {
                !pn.inputs.is_empty() && pn.inputs.iter().all(|i| self.is_constant(run, *i))
            }
            _ => false,
        };
        self.constant[node] = Some(c);
        c
    }

    fn arg_rel(&self, node: usize, name: &str) -> R<usize> {
        // 7: the step's relation arguments, read from the plan once (`PNode::rels`)
        self.plan.nodes[node].relation_input(name).ok_or_else(|| format!("no relation argument {name}"))
    }

    // -------------------------------------------------------------------------------------------- static: schemas, views

    fn schema(&mut self, run: &mut Run, node: usize) -> R<Rc<Schema>> {
        if let Some(sc) = &self.schemas[node] {
            return Ok(sc.clone());
        }
        let plan: &'p Plan = self.plan;
        let pn = &plan.nodes[node];
        let op = pn.op();
        let from = |me: &mut Self, run: &mut Run, name: &str| -> R<Rc<Schema>> {
            let n = me.arg_rel(node, name)?;
            me.schema(run, n)
        };
        let mk = |names: Vec<String>, tys: Vec<Ty>| Rc::new(Schema { names: Rc::new(names), tys: Rc::new(tys) });
        let sc = match op {
            "base" => {
                let rel = pn.base().ok_or("base relation")?;
                if let Some(nat) = native_of(run, rel) {
                    mk(native_cols(nat).iter().map(|c| c.0.to_owned()).collect(), native_cols(nat).iter().map(|c| c.1).collect())
                } else {
                    let (_, names, tys) = run.catalog(rel).ok_or_else(|| format!("no relation {rel}"))?;
                    Rc::new(Schema { names, tys })
                }
            }
            "select" => {
                let sp = self.sel(run, node)?;
                mk(sp.items.iter().map(|i| i.0.clone()).collect(), sp.items.iter().map(|i| static_ty(&i.1)).collect())
            }
            "intersection" | "difference" => from(self, run, "left")?,
            "union" | "antijoin" | "group-min" | "group-max" => from(self, run, if op == "union" || op == "antijoin" { "left" } else { "from" })?,
            "extend" => {
                let a = pn.args();
                let f = from(self, run, "from")?;
                let (mut names, mut tys) = ((*f.names).clone(), (*f.tys).clone());
                extend_values(a, |k, v| {
                    names.push(k.to_owned());
                    tys.push(match v {
                        Param::Number(_) => Ty::Num,
                        Param::Text(t) if t == "true" || t == "false" => Ty::Flag,
                        _ => Ty::Sym,
                    });
                    Ok(())
                })?;
                mk(names, tys)
            }
            "project" => {
                let a = pn.args();
                let f = from(self, run, "from")?;
                let cols: Vec<usize> = strs(&a["columns"])?.map(|c| pos(&f.names, c)).collect::<R<_>>()?;
                mk(cols.iter().map(|c| f.names[*c].clone()).collect(), cols.iter().map(|c| f.tys[*c]).collect())
            }
            "axis" => {
                let f = from(self, run, "from")?;
                let (mut names, mut tys) = ((*f.names).clone(), (*f.tys).clone());
                names.push("related".into());
                tys.push(Ty::Node);
                mk(names, tys)
            }
            "group-count" | "group-digest" => {
                let a = pn.args();
                let f = from(self, run, "from")?;
                let keys: Vec<usize> = strs(&a["keys"])?.map(|c| pos(&f.names, c)).collect::<R<_>>()?;
                let mut names: Vec<String> = keys.iter().map(|c| f.names[*c].clone()).collect();
                let mut tys: Vec<Ty> = keys.iter().map(|c| f.tys[*c]).collect();
                if op == "group-digest" {
                    pos(&f.names, s(&a["distinct"])?)?;
                    names.push(s(&a["digest"])?.to_owned());
                    tys.push(Ty::Num);
                }
                names.push(s(&a["count"])?.to_owned());
                tys.push(Ty::Num);
                mk(names, tys)
            }
            "worklist" => {
                let a = pn.args();
                let roots = from(self, run, "from")?;
                let edges = from(self, run, "edges")?;
                let lattice = from(self, run, "lattice")?;
                let (names, types) = worklist::schema(a, &roots, &edges, &lattice)?;
                mk(names, types)
            }
            "cycles" | "closure" | "follow" => {
                let a = pn.args();
                let e = from(self, run, "edges")?;
                let keys: Vec<usize> = strs(&a["keys"])?.map(|c| pos(&e.names, c)).collect::<R<_>>()?;
                let (si, di) = (pos(&e.names, s(&a["source"])?)?, pos(&e.names, s(&a["destination"])?)?);
                let mut names: Vec<String> = keys.iter().map(|c| e.names[*c].clone()).collect();
                let mut tys: Vec<Ty> = keys.iter().map(|c| e.tys[*c]).collect();
                names.extend([e.names[si].clone(), e.names[di].clone()]);
                tys.extend([e.tys[si], e.tys[di]]);
                mk(names, tys)
            }
            "ordered-match" | "as-of" => {
                let a = pn.args();
                let (l, r) = (from(self, run, "left")?, from(self, run, "right")?);
                let mut names = Vec::new();
                let mut tys = Vec::new();
                for (name, v) in a["output"].as_object().ok_or("output")? {
                    let side = if s(&v[0])? == "left" { &l } else { &r };
                    names.push(name.clone());
                    tys.push(side.tys[pos(&side.names, s(&v[1])?)?]);
                }
                mk(names, tys)
            }
            "span-between" | "replace" => {
                let a = pn.args();
                let f = from(self, run, "from")?;
                let (mut names, mut tys) = ((*f.names).clone(), (*f.tys).clone());
                names.push(s(&a["output"])?.to_owned());
                tys.push(if op == "replace" { Ty::Sym } else { Ty::Span });
                mk(names, tys)
            }
            "rank" => {
                let a = pn.args();
                let input = from(self, run, "from")?;
                let (mut names, mut types) = ((*input.names).clone(), (*input.tys).clone());
                pos(&names, s(&a["column"])?)?;
                names.push(s(&a["output"])?.to_owned());
                types.push(Ty::Num);
                mk(names, types)
            }
            "words" => {
                let a = pn.args();
                let source = from(self, run, "from")?;
                let _text = pos(&source.names, s(&a["text"])?)?;
                let mut names = (*source.names).clone();
                let mut types = (*source.tys).clone();
                names.push(s(&a["word"])?.to_owned());
                types.push(Ty::Sym);
                mk(names, types)
            }
            "span-bounds" => {
                let a = pn.args();
                let f = from(self, run, "from")?;
                let (mut names, mut tys) = ((*f.names).clone(), (*f.tys).clone());
                names.extend([s(&a["start-output"])?.to_owned(), s(&a["end-output"])?.to_owned()]);
                tys.extend([Ty::Num, Ty::Num]);
                mk(names, tys)
            }
            "source-matches" => {
                let a = pn.args();
                let f = from(self, run, "from")?;
                let (mut names, mut tys) = ((*f.names).clone(), (*f.tys).clone());
                names.push(s(&a["output"])?.to_owned());
                tys.push(Ty::Span);
                if let Some(lo) = a.get("line-output").and_then(Param::as_str) {
                    names.push(lo.to_owned());
                    tys.push(Ty::Num);
                }
                if a.get("capture").and_then(Param::as_str).is_some() {
                    names.push(s(&a["capture-output"])?.to_owned());
                    tys.push(Ty::Span);
                    if let Some(ct) = a.get("capture-text-output").and_then(Param::as_str) {
                        names.push(ct.to_owned());
                        tys.push(Ty::Sym);
                    }
                    if let Some(cl) = a.get("capture-line-output").and_then(Param::as_str) {
                        names.push(cl.to_owned());
                        tys.push(Ty::Num);
                    }
                }
                // redo-binding pass 6: `capture-texts={group: column, ...}`, the text of each named group ("" when it did
                // not take part), so one scan answers several spellings of a span
                if let Some(ct) = a.get("capture-texts").and_then(Param::as_object) {
                    for v in ct.values() {
                        names.push(s(v)?.to_owned());
                        tys.push(Ty::Sym);
                    }
                }
                mk(names, tys)
            }
            "capture" => {
                let a = pn.args();
                let f = from(self, run, "from")?;
                let (mut names, mut tys) = ((*f.names).clone(), (*f.tys).clone());
                for c in strs(&a["captures"])? {
                    names.push(c.to_owned());
                    tys.push(Ty::Sym);
                }
                mk(names, tys)
            }
            "rows" => {
                let a = pn.args();
                let names: Vec<_> = strs(&a["columns"])?.map(str::to_owned).collect();
                let k = names.len();
                mk(names, vec![Ty::Sym; k])
            }
            "render-syntax" => {
                let a = pn.args();
                let f = from(self, run, "from")?;
                let (mut names, mut tys) = ((*f.names).clone(), (*f.tys).clone());
                names.push(s(&a["output"])?.to_owned());
                tys.push(Ty::Sym);
                mk(names, tys)
            }
            "fits-rendered" => from(self, run, "from")?,
            "prefixes" => {
                let a = pn.args();
                let f = from(self, run, "from")?;
                let (mut names, mut tys) = ((*f.names).clone(), (*f.tys).clone());
                names.push(s(&a["output"])?.to_owned());
                tys.push(Ty::Sym);
                mk(names, tys)
            }
            "subtree-class" => {
                let a = pn.args();
                let f = from(self, run, "from")?;
                let (mut names, mut tys) = ((*f.names).clone(), (*f.tys).clone());
                names.push(s(&a["output"])?.to_owned());
                tys.push(Ty::Num);
                mk(names, tys)
            }
            "group-ordinal" => {
                let a = pn.args();
                let f = from(self, run, "from")?;
                let (mut names, mut tys) = ((*f.names).clone(), (*f.tys).clone());
                names.push(s(&a["output"])?.to_owned());
                tys.push(Ty::Num);
                mk(names, tys)
            }
            "ordered-dfs" => {
                let a = pn.args();
                let (f, e) = (from(self, run, "from")?, from(self, run, "edges")?);
                let (names, tys) = ordered_dfs::schema(a, (&f.names, &f.tys), (&e.names, &e.tys))?;
                mk(names, tys)
            }
            "order" => {
                let a = pn.args();
                let f = from(self, run, "from")?;
                let (mut names, mut tys) = ((*f.names).clone(), (*f.tys).clone());
                names.extend([s(&a["rank"])?.to_owned(), s(&a["step"])?.to_owned()]);
                tys.extend([Ty::Num, Ty::Num]);
                mk(names, tys)
            }
            other => return Err(format!("operator {other} not in the step set")),
        };
        self.schemas[node] = Some(sc.clone());
        Ok(sc)
    }

    /// A select's compiled form, once a run.
    fn sel(&mut self, run: &mut Run, node: usize) -> R<Rc<SelPlan>> {
        if let Some(sp) = &self.sels[node] {
            return Ok(sp.clone());
        }
        let plan: &'p Plan = self.plan;
        let pn = &plan.nodes[node];
        let srcs = pn.inputs.clone();
        let names = srcs.iter().enumerate().map(|(i, &source)| {
            Ok((pn.alias(i).map(str::to_owned), self.schema(run, source)?))
        }).collect::<R<Vec<_>>>()?;
        let mut conj = Vec::new();
        if let Some(e) = self.select_condition(run, pn, &names)
            .map_err(|e| format!("{e} in step {node} over {srcs:?}"))?
        {
            flatten(e, &mut conj);
        }
        // a test of two file columns always holds: every relation here is the file's
        // redo-honest: except a source whose rows are the run's (`is_constant`: `filesystem.file` and what reads only it),
        // whose file column names any file of the run
        let wide: Vec<bool> = srcs.iter().map(|n| self.is_constant(run, *n)).collect();
        conj.retain(|c| !matches!(c, E::Cmp(Op::Eq, a, b) if matches!(**a, E::Col(sa, _, Ty::File) if !wide[sa]) && matches!(**b, E::Col(sb, _, Ty::File) if !wide[sb])));
        let refs = conj.iter().map(refs_of).collect();
        let items: Vec<(String, E)> = match self.select_items(run, pn, &names)
            .map_err(|e| format!("{e} in step {node} over {srcs:?}"))?
        {
            None => {
                let mut v: Vec<(String, E)> = Vec::new();
                for (i, (_, sc)) in names.iter().enumerate() {
                    for (c, nm) in sc.names.iter().enumerate() {
                        if !v.iter().any(|(x, _)| x == nm) {
                            v.push((nm.clone(), E::Col(i, c, sc.tys[c])));
                        }
                    }
                }
                v
            }
            Some(items) => items,
        };
        let sp = Rc::new(SelPlan { srcs, conj, refs, items });
        self.sels[node] = Some(sp.clone());
        Ok(sp)
    }

    /// The view a node is, when it is one: a tree relation, or a select of column picks and tests over one view.
    #[inline]
    fn view(&mut self, run: &mut Run, node: usize) -> R<Option<Rc<ViewSpec>>> {
        match &self.views[node] {
            Some(None) => Ok(None),
            Some(v) => Ok(v.clone()),
            None => self.view_slow(run, node),
        }
    }

    #[inline(never)]
    fn view_slow(&mut self, run: &mut Run, node: usize) -> R<Option<Rc<ViewSpec>>> {
        let pn = &self.plan.nodes[node];
        let v = match pn.op() {
            "base" => {
                let rel = pn.base().unwrap_or("").to_owned();
                match native_of(run, &rel) {
                    Some(nat) => Some(Rc::new(ViewSpec::new(run, nat, (0..native_cols(nat).len()).collect(), Vec::new()))),
                    None => fact_of(run, &rel).map(|nat| {
                        let Native::Fact(i) = nat else { unreachable!() };
                        Rc::new(ViewSpec::new(run, nat, (0..run.fact_rels[usize::from(i)].1).collect(), Vec::new()))
                    }),
                }
            }
            "select" if pn.inputs.len() == 1 => {
                let inner = pn.inputs[0];
                match self.view(run, inner)? {
                    Some(iv) => {
                        let sp = self.sel(run, node)?;
                        let plain = sp.items.iter().all(|(_, e)| matches!(e, E::Col(0, _, _)));
                        if plain {
                            let map = sp.items.iter().map(|(_, e)| if let E::Col(_, c, _) = e { iv.map[*c] } else { 0 }).collect();
                            let mut filter = iv.filter.clone();
                            filter.extend(sp.conj.iter().map(|e| remap(e, &iv.map)));
                            Some(Rc::new(ViewSpec::new(run, iv.base, map, filter)))
                        } else {
                            None
                        }
                    }
                    None => None,
                }
            }
            _ => None,
        };
        if let Some(view) = &v {
            self.cold_compiler.bind(run, node, view);
        }
        self.views[node] = Some(v.clone());
        Ok(v)
    }

    // -------------------------------------------------------------------------------------------- per file

    /// A node's answer in this file: its slot, else made once (`make`).
    #[inline]
    pub(super) fn res(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Res> {
        if let Some(start) = operators::enter(node, 0) {
            let fresh = f.memo[node].is_none() && self.run_memo[node].is_none();
            let out = self.res_body(f, run, p, node);
            operators::leave(start, self.plan.nodes[node].op(), match &out { Ok(Res::Tab(t)) => t.n, _ => 0 }, fresh && matches!(&out, Ok(Res::Tab(_))));
            return out;
        }
        self.res_body(f, run, p, node)
    }

    #[inline]
    fn res_body(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Res> {
        match &f.memo[node] {
            Some(Res::Tab(t)) => Ok(Res::Tab(t.clone())),
            Some(Res::View(v)) => {
                // redo-fills pass 2: a view a lazy select source left unfilled is filled at its first read (lazy_view.rs)
                if let Native::Fact(x) = v.base
                    && f.facts.get(usize::from(x)).is_none_or(Option::is_none)
                {
                    let v = v.clone();
                    let was = fill_prof::site("view");
                    self.load_fact(f, run, p, v.base)?;
                    fill_prof::site(was);
                    return Ok(Res::View(v));
                }
                Ok(Res::View(v.clone()))
            }
            None => self.make(f, run, p, node),
        }
    }

    /// A node made in this file and put in its slot (listed in `touched`, cleared at the file's end).
    #[inline(never)]
    fn make(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Res> {
        let r = if cold_list::auditing() { self.cold_make(f, run, p, node)? } else { self.make_(f, run, p, node)? };
        f.remember(node, r.clone());
        Ok(r)
    }

    #[inline]
    fn make_(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Res> {
        if let Some(input) = f.population(node) {
            let sc = self.schema(run,node)?;
            return Ok(Res::Tab(input.full(&sc.names,&sc.tys)));
        }
        if let Some(v) = self.view(run, node)? {
            let was = fill_prof::site("view");
            self.load_fact(f, run, p, v.base)?;
            fill_prof::site(was);
            return Ok(Res::View(v));
        }
        if let Some(t) = &self.run_memo[node] {
            return Ok(Res::Tab(t.clone()));
        }
        if !sw!("VEC_NO_KNOWN") && self.known_empty(f, run, p, node) {
            self.stats.known[node] += 1;
            self.skipped(f, node);
            return Ok(Res::Tab(self.empty_rc(run, node)?));
        }
        // 8: a step over an empty input answers its shared empty table before any set-up
        let gate = self.plan.frame(node).gate;
        if gate < frame::MISSING && match f.population(gate as usize) {
            Some(input) => input.n == 0,
            None => self.eval(f, run, p, gate as usize)?.n == 0,
        } {
            self.stats.ran(node);
            self.stats.empty[node] += 1;
            let e = self.empty_rc(run, node)?;
            if self.is_constant(run, node) {
                self.run_memo[node] = Some(e.clone());
            }
            return Ok(Res::Tab(e));
        }
        let out = if self.timing { self.timed_body(f, run, p, node)? } else { self.body(f, run, p, node)? };
        emptyprop::ran();
        self.stats.ran(node);
        self.stats.produced(node, out.n);
        keyed::built(self, f, node, &out);
        if out.n == 0 {
            self.stats.empty[node] += 1;
        }
        if self.is_constant(run, node) {
            self.run_memo[node] = Some(out.clone());
        }
        Ok(Res::Tab(out))
    }

    /// `body` under the step clock (`VEC_STEP_TIME`): the step's own seconds (its inputs' taken off), and the empty
    /// runs' classes (`VEC_EMPTY_WHY`).
    #[inline(never)]
    fn timed_body(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Rc<Table>> {
        let ew = if sw!("VEC_EMPTY_WHY") { Some(empty_why::class(self, f, run, node)) } else { None };
        let t0 = step_clock();
        let saved = self.inner;
        self.inner = 0.0;
        let out = self.body(f, run, p, node);
        let dt = step_clock() - t0;
        let own = dt - self.inner;
        self.stats.time[node] += own;
        self.inner = saved + dt;
        let out = out?;
        if out.n == 0 {
            self.stats.empty_time[node] += own;
            emptyprop::empty_run(node, false, own);
        }
        if let Some(c) = ew {
            empty_why::tally(self, f, node, c, out.n == 0);
        }
        Ok(out)
    }

    /// The rows of `node` whose column `col` holds one of `keys`. Every step is made whole once a file (`res`); this
    /// reads that table through its index on `col` (made once a file). A view is read in place where its base has a
    /// keyed read for the column; a closure is read through its adjacency (`lazy`), not made whole.
    fn probe(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, col: usize, keys: &[u64]) -> R<Rc<Table>> {
        if let Some(start) = operators::enter(node, keys.len()) {
            let fresh = true;
            let out = self.probe_body(f, run, p, node, col, keys);
            operators::leave(start, self.plan.nodes[node].op(), out.as_ref().map_or(0, |t| t.n), fresh && out.is_ok());
            return out;
        }
        self.probe_body(f, run, p, node, col, keys)
    }

    #[inline]
    fn probe_body(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, col: usize, keys: &[u64]) -> R<Rc<Table>> {
        if let Some(input) = f.population(node) {
            let sc=self.schema(run,node)?;
            if input.can_probe(col) { return Ok(input.probe(&sc.names,&sc.tys,col,keys)); }
            let table=input.full(&sc.names,&sc.tys);
            return Ok(recycle::rc(subset(f,node,&table,col,keys)));
        }
        if sw!("VEC_COLD_COUNT") { run.cold.probe(f, Some(keys.len())); }
        if keys.is_empty() {
            return self.empty_rc(run, node);
        }
        // redo-whole pass 2: a step not made in this file whose one consumer reads it by key is read by key (keyed.rs)
        if f.memo[node].is_none()
            && let Some(t) = self.keyed(f, run, p, node, col, keys, false)?
        {
            return Ok(t);
        }
        if let Some(v) = self.view(run, node)?
            && File::probes(v.base, v.map[col])
        {
            if let Res::Tab(t) = self.res(f, run, p, node)? {
                // known empty in this file
                return Ok(t);
            }
            let sc = self.schema(run, node)?;
            let mut rows = std::mem::take(&mut self.scratch_rows);
            rows.clear();
            for k in keys {
                f.native_probe(run, v.base, v.map[col], *k, &mut rows);
            }
            if !v.filter.is_empty() {
                view_cols::retain(f, run, &v, &mut rows);
            }
            if rows.is_empty() {
                self.scratch_rows = rows;
                return self.empty_rc(run, node);
            }
            let mut out = Table::with_rows(sc.names.clone(), sc.tys.clone(), rows.len());
            for c in 0..v.map.len() {
                view_cols::gather(f, run, v.base, v.map[c], &rows, &mut out.cols[c]);
            }
            out.n = rows.len();
            self.stats.touched[node] += rows.len() as u64;
            self.scratch_rows = rows;
            return Ok(recycle::rc(out));
        }
        if !f.after_last_file && self.plan.frame(node).op == frame::Op::Closure
            && let Some(t) = self.closure_at(f, run, p, node, col, keys)?
        {
            return Ok(recycle::rc(t));
        }
        let t = self.eval(f, run, p, node)?;
        if t.n == 0 {
            return Ok(t);
        }
        Ok(recycle::rc(subset(f, node, &t, col, keys)))
    }

    /// A closure's rows whose source (`col` the first after its keys) or destination holds one of `keys`, read through
    /// its adjacency; None where its keys are not all the file (it is then made whole).
    fn closure_at(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, col: usize, keys: &[u64]) -> R<Option<Table>> {
        let l = self.lazy(f, run, p, node)?;
        let nk = l.keys.len();
        if col < nk || l.tys[..nk].iter().any(|t| *t != Ty::File) {
            return Ok(None);
        }
        let forward = col == nk;
        let mut out = Table::with(l.names.clone(), l.tys.clone());
        let mut key: Vec<u64> = vec![u64::from(f.file); nk + 1];
        let mut row: Vec<u64> = vec![u64::from(f.file); nk + 2];
        for k in keys {
            key[nk] = *k;
            for d in reach_dir(&l, &key, forward).iter() {
                (row[nk], row[nk + 1]) = if forward { (*k, *d) } else { (*d, *k) };
                out.push_row(&row);
            }
        }
        Ok(Some(out))
    }

    /// A node's rows as a table (a view is made whole here, once a file).
    pub(super) fn eval(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Rc<Table>> {
        if let Some(start) = operators::enter(node, 0) {
            let fresh = !matches!(&f.memo[node], Some(Res::Tab(_))) && !f.vmat.contains_key(&node) && self.run_memo[node].is_none();
            let out = self.eval_body(f, run, p, node);
            operators::leave(start, self.plan.nodes[node].op(), out.as_ref().map_or(0, |t| t.n), fresh && out.is_ok());
            return out;
        }
        self.eval_body(f, run, p, node)
    }

    #[inline]
    fn eval_body(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Rc<Table>> {
        match self.res(f, run, p, node)? {
            Res::Tab(t) => Ok(t),
            Res::View(v) => {
                if let Some(t) = f.vmat.get(&node) {
                    return Ok(t.clone());
                }
                let t0 = leaf_scan::table_clock();
                let rows = view_rows_for(f, run, &v, node, true);
                let rc = if rows.is_empty() {
                    self.empty_rc(run, node)?
                } else {
                    let sc = self.schema(run, node)?;
                    let mut out = Table::with_rows(sc.names.clone(), sc.tys.clone(), rows.len());
                    for c in 0..v.map.len() {
                        view_cols::gather_ids(f, run, v.base, v.map[c], rows.iter().copied(), &mut out.cols[c]);
                    }
                    out.n = rows.len();
                    recycle::rc(out)
                };
                f.vmat.insert(node, rc.clone());
                let dt = leaf_scan::table_clock() - t0;
                self.stats.time[node] += dt;
                if self.timing || !sw!("VEC_LEAF_CLOCK") {
                    self.inner += dt;
                }
                // A row-stream reader may already have produced this logical result.
                if !f.cursor_views.remove(&node) {
                    self.stats.built[node] += rc.n as u64;
                    self.stats.ran(node);
                    self.stats.produced(node, rc.n);
                }
                Ok(rc)
            }
        }
    }

    /// 8: a cleared row-number buffer from the worker's pool.
    fn take_u32(&mut self) -> Vec<u32> {
        self.u32_pool.pop().unwrap_or_default()
    }

    fn give_u32(&mut self, mut v: Vec<u32>) {
        if v.capacity() > 0 && v.capacity() <= 1 << 20 && self.u32_pool.len() < 256 {
            v.clear();
            self.u32_pool.push(v);
        }
    }

    /// The step's shared empty answer (made once a run).
    #[inline]
    pub(super) fn empty_rc(&mut self, run: &mut Run, node: usize) -> R<Rc<Table>> {
        if let Some(t) = &self.empties[node] {
            return Ok(t.clone());
        }
        let sc = self.schema(run, node)?;
        let t = Rc::new(Table::with(sc.names.clone(), sc.tys.clone()));
        self.empties[node] = Some(t.clone());
        Ok(t)
    }

    fn empty(&mut self, run: &mut Run, node: usize) -> R<Table> {
        self.empty_rows(run, node, 0)
    }

    /// redo-alloc: `empty` for a table about to hold `n` rows.
    pub(super) fn empty_rows(&mut self, run: &mut Run, node: usize, n: usize) -> R<Table> {
        let sc = self.schema(run, node)?;
        Ok(Table::with_rows(sc.names.clone(), sc.tys.clone(), n))
    }

    fn lazy(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Rc<Lazy>> {
        if let Some(l) = f.lazies.get(&node) {
            return Ok(l.clone());
        }
        let edges = self.edge_input(f, run, p, self.arg_rel(node, "edges")?)?;
        let t0 = step_clock();
        let a = self.plan.nodes[node].args();
        let keys: Vec<usize> = strs(&a["keys"])?.map(|k| edges.col(k)).collect::<R<_>>()?;
        let (si, di) = (edges.col(s(&a["source"])?)?, edges.col(s(&a["destination"])?)?);
        let (adj, radj) = edges.adjacency(f, run, &keys, si, di);
        let sc = self.schema(run, node)?;
        let l = Rc::new(Lazy { keys, adj, radj, node, names: sc.names.clone(), tys: sc.tys.clone(), cache: RefCell::new(FxHashMap::default()), rcache: RefCell::new(FxHashMap::default()) });
        let dt = step_clock() - t0;
        self.stats.time[node] += dt;
        self.inner += dt;
        f.lazies.insert(node, l.clone());
        Ok(l)
    }

    /// A step's run: the kernels' dispatch, then what ends the step before anything else (the frame, prepared at plan
    /// load), then the operator's body. redo-union pass 2: the answer is shared (`Rc`): a kernel's passed-through input
    /// and the step's empty answer are handed on, not copied.
    fn body(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Rc<Table>> {
        let fr = self.plan.frame(node);
        match fr.op {
            frame::Op::Union if !sw!("VEC_OLD_UNION") => return union_kernel::whole(self, f, run, p, node),
            frame::Op::Antijoin if !sw!("VEC_OLD_ANTIJOIN") => return antijoin_kernel::whole(self, f, run, p, node),
            _ => {}
        }
        if fr.counted && !f.after_last_file
            && let Some(t) = self.counted(f, run, p, node)?
        {
            return Ok(recycle::rc(t));
        }
        if fr.first == frame::MISSING {
            return Err(format!("no relation argument of {}", self.plan.nodes[node].op()));
        }
        // an empty answer is the step's shared one (the body's own table is dropped, nothing kept a run); the select's
        // dispatch point (by the frame's code)
        if fr.op == frame::Op::Select {
            if let Some(index) = self.projection(run, node) {
                return self.col_select(f, run, p, node, index);
            }
            return self.select_entry(f, run, p, node);
        }
        let t = self.step(f, run, p, node)?;
        if t.n == 0 { self.empty_rc(run, node) } else { Ok(recycle::rc(t)) }
    }

    /// A group count over a counted select: its kept rows and their counts, grouped (None: the band does not apply).
    fn counted(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Option<Table>> {
        let plan: &'p Plan = self.plan;
        let a = plan.nodes[node].args();
        {
            let fnode = self.arg_rel(node, "from")?;
            let pb = plan.nodes[fnode].semi().ok_or("count without semi")?;
            if let Some((from, counts)) = self.band_rows(f, run, p, fnode, pb, true)? {
                self.stats.produced(fnode, from.n);
                let keys: Vec<usize> = strs(&a["keys"])?.map(|c| from.col(c)).collect::<R<_>>()?;
                let mut groups: FxHashMap<inline_key::Key, u64> = FxHashMap::default();
                let mut order: Vec<inline_key::Key> = Vec::new();
                // redo-alloc: a row's key read into one buffer; lists made only for a key not seen (same groups, same order)
                let mut kb: Vec<u64> = Vec::with_capacity(keys.len() + 1);
                for i in 0..from.n {
                    kb.clear();
                    kb.extend(keys.iter().map(|c| from.cols[*c][i]));
                    match groups.get_mut(&kb[..]) {
                        Some(e) => *e += counts[i],
                        None => {
                            order.push(inline_key::Key::of(&kb));
                            groups.insert(inline_key::Key::of(&kb), counts[i]);
                        }
                    }
                }
                let mut out = self.empty(run, node)?;
                for k in order {
                    kb.clear();
                    kb.extend_from_slice(k.cells());
                    kb.push(groups[&k]);
                    out.push_row(&kb);
                }
                return Ok(Some(out));
            }
        }
        Ok(None)
    }

    /// The operator's body (its first input is not empty).
    fn step(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Table> {
        let plan: &'p Plan = self.plan;
        let pn = &plan.nodes[node];
        let op = pn.op();
        let a = pn.args();
        match op {
            "base" => self.base(f, run, p, pn.base().ok_or("base relation")?),
            "select" => self.select_k(f, run, p, node),
            "union" => {
                let l = self.eval(f, run, p, self.arg_rel(node, "left")?)?;
                let r = self.eval(f, run, p, self.arg_rel(node, "right")?)?;
                let mut out = l.copy();
                let map: Vec<usize> = l.names.iter().map(|c| r.col(c)).collect::<R<_>>()?;
                for (c, m) in map.iter().enumerate() {
                    out.cols[c].extend_from_slice(&r.cols[*m]);
                }
                out.n += r.n;
                Ok(distinct(out))
            }
            "intersection" | "difference" => {
                let left = self.eval(f, run, p, self.arg_rel(node, "left")?)?;
                let right = self.eval(f, run, p, self.arg_rel(node, "right")?)?;
                set_ops::select_rows(&left, &right, op == "intersection")
            }
            "group-digest" => {
                let input = self.eval(f, run, p, self.arg_rel(node, "from")?)?;
                let mut out = self.empty(run, node)?;
                set_ops::digest(run, f.file, a, &input, &mut out)?;
                Ok(out)
            }
            "antijoin" => self.antijoin(f, run, p, node, a),
            "extend" => {
                let from = self.eval(f, run, p, (self.plan.frame(node).first as usize))?;
                let mut out = from.copy();
                extend_values(a, |_, v| {
                    let x = match v {
                        Param::Number(_) => v.as_i64().ok_or("extend number")? as u64,
                        Param::Text(t) if t == "true" || t == "false" => u64::from(t == "true"),
                        Param::Text(t) => u64::from(run.sym(t.as_bytes())),
                        _ => return Err(format!("extend value {v:?}")),
                    };
                    // redo-alloc: the constant column from the pool
                    out.push_const_col(x, from.n);
                    Ok(())
                })?;
                let sc = self.schema(run, node)?;
                (out.names, out.tys) = (sc.names.clone(), sc.tys.clone());
                Ok(out)
            }
            "project" => {
                let from = self.eval(f, run, p, (self.plan.frame(node).first as usize))?;
                let cols: Vec<usize> = strs(&a["columns"])?.map(|c| from.col(c)).collect::<R<_>>()?;
                let osc = self.schema(run, node)?;
                let mut out = Table::with(osc.names.clone(), osc.tys.clone());
                let mut seen: rustc_hash::FxHashSet<inline_key::Key> = rustc_hash::FxHashSet::default();
                // redo-alloc: a row read into one buffer; a set entry made only for a row not seen
                let mut row: Vec<u64> = Vec::with_capacity(cols.len());
                for i in 0..from.n {
                    row.clear();
                    row.extend(cols.iter().map(|c| from.cols[*c][i]));
                    if !seen.contains(&row[..]) {
                        seen.insert(inline_key::Key::of(&row));
                        out.push_row(&row);
                    }
                }
                Ok(out)
            }
            "axis" => self.axis(f, run, p, node, a),
            "group-count" => {
                let from = self.eval(f, run, p, (self.plan.frame(node).first as usize))?;
                let keys: Vec<usize> = strs(&a["keys"])?.map(|c| from.col(c)).collect::<R<_>>()?;
                let distinct = match &a["distinct"] {
                    Param::Text(d) => Some(from.col(d)?),
                    _ => None,
                };
                let mut groups: FxHashMap<inline_key::Key, (u64, rustc_hash::FxHashSet<u64>)> = FxHashMap::default();
                let mut order: Vec<inline_key::Key> = Vec::new();
                // redo-alloc: a row's key read into one buffer; lists made only for a key not seen (same groups, same order)
                let mut kb: Vec<u64> = Vec::with_capacity(keys.len() + 1);
                for i in 0..from.n {
                    kb.clear();
                    kb.extend(keys.iter().map(|c| from.cols[*c][i]));
                    if !groups.contains_key(&kb[..]) {
                        order.push(inline_key::Key::of(&kb));
                        groups.insert(inline_key::Key::of(&kb), (0, rustc_hash::FxHashSet::default()));
                    }
                    let e = groups.get_mut(&kb[..]).expect("the group just made");
                    match distinct {
                        Some(d) => {
                            e.1.insert(from.cols[d][i]);
                        }
                        None => e.0 += 1,
                    }
                }
                let mut out = self.empty(run, node)?;
                for k in order {
                    let g = &groups[&k];
                    let c = if distinct.is_some() { g.1.len() as u64 } else { g.0 };
                    kb.clear();
                    kb.extend_from_slice(k.cells());
                    kb.push(c);
                    out.push_row(&kb);
                }
                Ok(out)
            }
            "group-min" | "group-max" => {
                let from = self.eval(f, run, p, (self.plan.frame(node).first as usize))?;
                let keys: Vec<usize> = strs(&a["keys"])?.map(|c| from.col(c)).collect::<R<_>>()?;
                let oc = from.col(s(&a["order"])?)?;
                let max = op == "group-max";
                let mut best: FxHashMap<inline_key::Key, i64> = FxHashMap::default();
                // redo-alloc: a row's key read into one buffer; a key list made only for a key not seen
                let mut kb: Vec<u64> = Vec::with_capacity(keys.len());
                for i in 0..from.n {
                    kb.clear();
                    kb.extend(keys.iter().map(|c| from.cols[*c][i]));
                    let v = from.cols[oc][i] as i64;
                    match best.get_mut(&kb[..]) {
                        Some(e) => {
                            if (max && v > *e) || (!max && v < *e) {
                                *e = v;
                            }
                        }
                        None => {
                            best.insert(inline_key::Key::of(&kb), v);
                        }
                    }
                }
                let mut out = Table::with(from.names.clone(), from.tys.clone());
                let mut row = Vec::new();
                for i in 0..from.n {
                    kb.clear();
                    kb.extend(keys.iter().map(|c| from.cols[*c][i]));
                    if best[&kb[..]] == from.cols[oc][i] as i64 {
                        from.row(i, &mut row);
                        out.push_row(&row);
                    }
                }
                Ok(out)
            }
            "cycles" => {
                let edges = self.eval(f, run, p, self.arg_rel(node, "edges")?)?;
                let mut out = self.empty(run, node)?;
                populations::cyclic(&edges, edges.col(s(&a["source"])?)?, edges.col(s(&a["destination"])?)?, &mut out)?;
                Ok(out)
            }
            "rank" => {
                let input = self.eval(f, run, p, self.arg_rel(node, "from")?)?;
                let mut out = self.empty(run, node)?;
                populations::ranked(run, &input, a, &mut out)?;
                Ok(out)
            }
            "closure" => {
                let l = self.lazy(f, run, p, node)?;
                let mut out = Table::with(l.names.clone(), l.tys.clone());
                let starts: Vec<Vec<u64>> = l.adj.keys().cloned().collect();
                for k in starts {
                    let reach = reach_dir(&l, &k, true);
                    for d in reach.iter() {
                        let mut row = k.clone();
                        row.push(*d);
                        out.push_row(&row);
                    }
                }
                Ok(out)
            }
            "follow" => {
                let edges = self.eval(f, run, p, self.arg_rel(node, "edges")?)?;
                let (sn, dn) = (s(&a["source"])?, s(&a["destination"])?);
                let keys: Vec<usize> = strs(&a["keys"])?.map(|k| edges.col(k)).collect::<R<_>>()?;
                let (si, di) = (edges.col(sn)?, edges.col(dn)?);
                let mut adj: FxHashMap<Vec<u64>, Vec<u64>> = FxHashMap::default();
                let mut fixed: rustc_hash::FxHashSet<Vec<u64>> = rustc_hash::FxHashSet::default();
                for r in 0..edges.n {
                    let mut k: Vec<u64> = keys.iter().map(|c| edges.cols[*c][r]).collect();
                    k.push(edges.cols[si][r]);
                    if edges.cols[si][r] == edges.cols[di][r] {
                        fixed.insert(k.clone());
                    }
                    adj.entry(k).or_default().push(edges.cols[di][r]);
                }
                let mut out = self.empty(run, node)?;
                let starts: Vec<Vec<u64>> = adj.keys().cloned().collect();
                for k in starts {
                    let kp = &k[..k.len() - 1];
                    let mut seen = rustc_hash::FxHashSet::default();
                    let mut stack = vec![k[k.len() - 1]];
                    let mut found = std::collections::BTreeSet::new();
                    seen.insert(k[k.len() - 1]);
                    while let Some(x) = stack.pop() {
                        let mut kx = kp.to_vec();
                        kx.push(x);
                        if fixed.contains(&kx) {
                            found.insert(x);
                        }
                        if let Some(ns) = adj.get(&kx) {
                            for y in ns {
                                if seen.insert(*y) {
                                    stack.push(*y);
                                }
                            }
                        }
                    }
                    for x in found {
                        let mut row = k.clone();
                        row.push(x);
                        out.push_row(&row);
                    }
                }
                Ok(out)
            }
            "ordered-match" | "as-of" => self.ordered(f, run, p, node, a, op == "as-of", None),
            "span-between" => {
                let from = self.eval(f, run, p, (self.plan.frame(node).first as usize))?;
                let (sc, ec) = (from.col(s(&a["start"])?)?, from.col(s(&a["end"])?)?);
                let mut out = from.copy();
                let col: Vec<u64> = (0..from.n)
                    .map(|i| {
                        let (x, y) = (from.cols[sc][i], from.cols[ec][i]);
                        if x == NULL || y == NULL { NULL } else { (x & 0xffff_ffff_0000_0000) | (y & 0xffff_ffff) }
                    })
                    .collect();
                out.cols.push(col);
                let sc = self.schema(run, node)?;
                (out.names, out.tys) = (sc.names.clone(), sc.tys.clone());
                Ok(out)
            }
            "span-bounds" => {
                let from = self.eval(f, run, p, (self.plan.frame(node).first as usize))?;
                let sc = from.col(s(&a["span"])?)?;
                let mut out = from.copy();
                out.cols.push(from.cols[sc].iter().map(|x| x >> 32).collect());
                out.cols.push(from.cols[sc].iter().map(|x| x & 0xffff_ffff).collect());
                let osc = self.schema(run, node)?;
                (out.names, out.tys) = (osc.names.clone(), osc.tys.clone());
                Ok(out)
            }
            "words" => {
                let from = self.eval(f, run, p, self.arg_rel(node, "from")?)?;
                let text = from.col(s(&a["text"])?)?;
                let mut out = self.empty(run, node)?;
                words::write(run, &from, text, &mut out)?;
                Ok(out)
            }
            "source-matches" => self.source_matches(f, run, p, node, a),
            "replace" => {
                let from = self.eval(f, run, p, (self.plan.frame(node).first as usize))?;
                let tc = from.col(s(&a["text"])?)?;
                let re = run.re(s(&a["pattern"])?)?;
                let rep = s(&a["replacement"])?.as_bytes().to_vec();
                let mut out = from.copy();
                let col: Vec<u64> = (0..from.n)
                    .map(|i| {
                        let t = run.text(from.cols[tc][i] as u32).to_vec();
                        let r = re.pattern.replace_all(&t, regex::bytes::NoExpand(&rep)).into_owned();
                        u64::from(run.sym(&r))
                    })
                    .collect();
                out.cols.push(col);
                let sc = self.schema(run, node)?;
                (out.names, out.tys) = (sc.names.clone(), sc.tys.clone());
                Ok(out)
            }
            "capture" => {
                let from = self.eval(f, run, p, (self.plan.frame(node).first as usize))?;
                let pc = from.col(s(&a["path"])?)?;
                let pattern = s(&a["pattern"])?;
                let Param::CaptureColumns(at) = &a["capture-columns"] else {
                    return Err("unbound capture columns".into());
                };
                let separator = a.get("separator").and_then(Param::as_str);
                let mut out = self.empty(run, node)?;
                let equal = if f.after_last_file {
                    capture::required_equalities(self, run, node)
                } else {
                    Vec::new()
                };
                capture::write(run, &from, pc, pattern, separator, at, f.after_last_file, &equal, &mut out)?;
                Ok(out)
            }
            "group-ordinal" => {
                // the distinct rows, each with how many distinct rows of its group have a strictly smaller order
                let from = self.eval(f, run, p, (self.plan.frame(node).first as usize))?;
                let keys: Vec<usize> = strs(&a["keys"])?.map(|c| from.col(c)).collect::<R<_>>()?;
                let oc = from.col(s(&a["order"])?)?;
                let mut seen = rustc_hash::FxHashSet::default();
                let mut distinct = Vec::new();
                let mut row = Vec::new();
                for i in 0..from.n {
                    from.row(i, &mut row);
                    if seen.insert(row.clone()) {
                        distinct.push(i);
                    }
                }
                let mut groups: FxHashMap<Vec<u64>, Vec<i64>> = FxHashMap::default();
                for &i in &distinct {
                    groups.entry(keys.iter().map(|c| from.cols[*c][i]).collect()).or_default().push(from.cols[oc][i] as i64);
                }
                for g in groups.values_mut() {
                    g.sort_unstable();
                }
                let mut out = self.empty(run, node)?;
                for &i in &distinct {
                    let g = &groups[&keys.iter().map(|c| from.cols[*c][i]).collect::<Vec<u64>>()];
                    let place = g.partition_point(|x| *x < from.cols[oc][i] as i64);
                    from.row(i, &mut row);
                    row.push(place as u64);
                    out.push_row(&row);
                }
                Ok(out)
            }
            "render-syntax" => {
                let from = self.eval(f, run, p, (self.plan.frame(node).first as usize))?;
                let nc = from.col(s(&a["node"])?)?;
                let ctx = s(&a["context"])?;
                let mut out = self.empty(run, node)?;
                let mut row = Vec::new();
                for i in 0..from.n {
                    if let Some(t) = p.render(from.cols[nc][i] as u32, ctx)? {
                        from.row(i, &mut row);
                        row.push(u64::from(run.sym(&t)));
                        out.push_row(&row);
                    }
                }
                Ok(out)
            }
            "subtree-class" => self.subtree_class(f, run, p, node),
            "prefixes" => self.prefixes(f, run, p, node),
            "order" => self.order(f, run, p, node),
            "fits-rendered" => {
                let from = self.eval(f, run, p, (self.plan.frame(node).first as usize))?;
                let texts: Vec<usize> = strs(&a["texts"])?.map(|c| from.col(c)).collect::<R<_>>()?;
                let lits = strs(&a["literals"])?;
                let sc = from.col(s(&a["span"])?)?;
                let max = a["max-line"].as_u64().ok_or("max-line")? as usize;
                let tab = a["tab-width"].as_u64().ok_or("tab-width")? as usize;
                let mut out = self.empty(run, node)?;
                let mut row = Vec::new();
                let src = std::str::from_utf8(f.src).map_err(|_| "a source that is no text")?;
                for i in 0..from.n {
                    let sp = from.cols[sc][i];
                    let start = (sp >> 32) as usize;
                    let line_start = src[..start].rfind(['\r', '\n']).map_or(0, |at| at + 1);
                    let mut cand = String::new();
                    for (l, c) in lits.clone().zip(&texts) {
                        cand.push_str(l);
                        cand.push_str(&String::from_utf8_lossy(run.text(from.cols[*c][i] as u32)));
                    }
                    if let Some(last) = lits.clone().next_back() {
                        cand.push_str(last);
                    }
                    if fits(&cand, &src[line_start..start], max, tab) {
                        from.row(i, &mut row);
                        out.push_row(&row);
                    }
                }
                Ok(out)
            }
            "rows" => {
                if let ryo_values::domain::finish_plans::types::FinishStep::Rows(literal) = pn.step() {
                    let mut out = self.empty_rows(run, node, literal.rows.len())?;
                    for row in &literal.rows {
                        if row.cells.len() != out.cols.len() { return Err("literal row width".into()); }
                        for (column, value) in out.cols.iter_mut().zip(&row.cells) {
                            column.push(u64::from(run.sym(value.0.as_bytes())));
                        }
                        out.n += 1;
                    }
                    return Ok(out);
                }
                let mut out = self.empty(run, node)?;
                for r in a["from"].as_array().ok_or("rows from")? {
                    let row: Vec<u64> = r.as_array().ok_or("row")?.iter().map(|x| u64::from(run.sym(x.as_str().unwrap_or("").as_bytes()))).collect();
                    out.push_row(&row);
                }
                Ok(out)
            }
            "worklist" => {
                let edges = self.eval(f, run, p, self.arg_rel(node, "edges")?)?;
                let lattice = self.eval(f, run, p, self.arg_rel(node, "lattice")?)?;
                let roots = self.eval(f, run, p, self.arg_rel(node, "from")?)?;
                let mut out = self.empty(run, node)?;
                worklist::trace(a, &roots, &edges, &lattice, &mut out)?;
                Ok(out)
            }
            "ordered-dfs" => {
                let from = self.eval(f, run, p, self.arg_rel(node, "from")?)?;
                let edges = self.eval(f, run, p, self.arg_rel(node, "edges")?)?;
                let mut out = self.empty(run, node)?;
                ordered_dfs::walked(a, &from, &edges, &mut out)?;
                Ok(out)
            }
            other => Err(format!("operator {other} not in the step set")),
        }
    }

    /// 6h: a fact view's fill, asked once a file (the same fill a whole `base` step reads).
    fn load_fact(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, nat: Native) -> R<()> {
        let Native::Fact(i) = nat else { return Ok(()) };
        let i = usize::from(i);
        if f.facts.get(i).is_some_and(Option::is_some) {
            return Ok(());
        }
        let rel = run.fact_rels[i].0.clone();
        if fill_prof::counting() {
            // redo-int round 37: redo-whole deleted the run stack this diagnostic read; the asker is not known now
            let by = u32::MAX;
            fill_prof::asked(i, by);
            if sw!("VEC_ASKER_PLAN") && by != u32::MAX && fill_prof::first_asker(by) {
                eprintln!("ASKER-SITE {by} {} {}", fill_prof::SITE.with(std::cell::Cell::get), std::backtrace::Backtrace::force_capture().to_string().lines().filter(|l| l.contains("Exec") || l.contains("kernel") || l.contains("finish")).take(12).map(str::trim).collect::<Vec<_>>().join(" < "));
                let pn = &self.plan.nodes[by as usize];
                let ins: Vec<String> = pn.inputs.iter().map(|x| format!("{x}:{}:{}", self.plan.nodes[*x].op(), self.plan.nodes[*x].summary_text().chars().take(160).collect::<String>())).collect();
                eprintln!("ASKER-PLAN {by} {} {} INPUTS {}", pn.op(), pn.summary_text().chars().take(900).collect::<String>(), ins.join(" | "));
            }
        }
        if !f.fills.contains_key(&rel) {
            let t = fill_len::fill_or_empty(run, p, &rel, f.file)?;
            f.fills.insert(rel.clone(), t);
        }
        if f.facts.len() <= i {
            f.facts.resize(i + 1, None);
        }
        if f.fact_read.len() <= i {
            f.fact_read.resize_with(i + 1, Default::default);
        }
        f.facts[i] = f.fills.get(&rel).cloned();
        Ok(())
    }

    fn base(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, rel: &str) -> R<Table> {
        // redo-fills pass 2: by who supplies the relation, as the run's catalog declares it
        match run.supplier(rel) {
            Supplier::RunFiles => {
                let (_, names, tys) = run.catalog(rel).ok_or_else(|| format!("no relation {rel}"))?;
                let mut out = Table::with(names, tys);
                for i in 0..run.paths.len() {
                    out.push_row(&[i as u64]);
                }
                return Ok(out);
            }
            Supplier::Provider | Supplier::Host => {}
        }
        if let Some(t) = f.fills.get(rel) {
            return Ok(t.copy());
        }
        let was = fill_prof::site("base");
        let t = fill_len::fill_or_empty(run, p, rel, f.file);
        fill_prof::site(was);
        let t = t?;
        let owned = t.copy();
        f.fills.insert(rel.to_owned(), t);
        Ok(owned)
    }

    fn antijoin(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, a: &Args) -> R<Table> {
        let l = self.eval(f, run, p, self.arg_rel(node, "left")?)?;
        let rn = self.arg_rel(node, "right")?;
        let lk: Vec<usize> = strs(&a["left-on"])?.map(|c| l.col(c)).collect::<R<_>>()?;
        let rsc = self.schema(run, rn)?;
        let rk: Vec<usize> = strs(&a["right-on"])?.map(|c| pos(&rsc.names, c)).collect::<R<_>>()?;
        let mut out = Table::with(l.names.clone(), l.tys.clone());
        let mut row = Vec::new();
        // a right side that is a view probed in place on a key: a knock-out per left row, nothing made
        if self.view(run, rn)?.is_some()
            && let Res::View(v) = self.res(f, run, p, rn)?
            && let Some(ki) = (0..rk.len()).find(|i| File::probes(v.base, v.map[rk[*i]]))
        {
            let mut hits = Vec::new();
            for i in 0..l.n {
                hits.clear();
                f.native_probe(run, v.base, v.map[rk[ki]], l.cols[lk[ki]][i], &mut hits);
                let knocked = hits.iter().any(|h| {
                    view_holds(f, run, &v, *h) && (0..rk.len()).all(|k| view_cols::cell(f, run, v.base, *h, v.map[rk[k]]) == l.cols[lk[k]][i] || rsc.tys[rk[k]] == Ty::File)
                });
                self.stats.touched[node] += hits.len() as u64;
                if !knocked {
                    l.row(i, &mut row);
                    out.push_row(&row);
                }
            }
            return Ok(out);
        }
        let r = self.eval(f, run, p, rn)?;
        if r.n == 0 {
            return Ok(l.copy());
        }
        self.stats.touched[node] += r.n as u64;
        if rk.len() == 1 {
            let set: rustc_hash::FxHashSet<u64> = r.cols[rk[0]].iter().copied().collect();
            for i in 0..l.n {
                if !set.contains(&l.cols[lk[0]][i]) {
                    l.row(i, &mut row);
                    out.push_row(&row);
                }
            }
            return Ok(out);
        }
        let set: rustc_hash::FxHashSet<Vec<u64>> = (0..r.n).map(|i| rk.iter().map(|c| r.cols[*c][i]).collect()).collect();
        let mut key = Vec::new();
        for i in 0..l.n {
            key.clear();
            key.extend(lk.iter().map(|c| l.cols[*c][i]));
            if !set.contains(&key) {
                l.row(i, &mut row);
                out.push_row(&row);
            }
        }
        Ok(out)
    }

    fn src_of(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Src> {
        if self.plan.nodes[node].op() == "closure" && f.population(node).is_none() {
            // a closure made whole is read as its table, else through its adjacency
            if let Some(Res::Tab(t)) = &f.memo[node] {
                return Ok(Src::Tab(t.clone(), node));
            }
            return Ok(Src::Lazy(self.lazy(f, run, p, node)?));
        }
        Ok(match self.res(f, run, p, node)? {
            Res::Tab(t) => Src::Tab(t, node),
            Res::View(v) => Src::View(v, node),
        })
    }

    fn select(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Table> {
        {
            let sp = self.sel(run, node)?;
            if filter_order::file_fails(run, f, &sp.conj) {
                *self.tally.back.entry((node, "a per-file test fails")).or_default() += 1;
                empty_sel::at(empty_sel::stage::FILE_TEST);
                return self.empty(run, node);
            }
        }
        if let Some(pb) = self.plan.nodes[node].semi()
            && !self.plan.nodes[node].counted()
            && !sw!("VEC_NO_SEMI")
            && let Some(t) = self.semi(f, run, p, node, pb as usize)?
        {
            empty_sel::at(empty_sel::stage::SEMI);
            return Ok(t);
        }
        let sp = self.sel(run, node)?;
        let k = sp.srcs.len();
        let conj = &sp.conj;
        let refs = &sp.refs;
        // the made inputs first, in order: an empty one ends the step
        let mut srcs: Vec<Src> = Vec::with_capacity(k);
        for (i, n) in sp.srcs.iter().enumerate() {
            let src = self.src_of(f, run, p, *n)?;
            if let Src::Tab(t, _) = &src
                && t.n == 0
            {
                empty_sel::with(|m| {
                    m.stage = empty_sel::stage::SOURCE_EMPTY;
                    m.src = i as u8;
                    m.srcn = *n as u32;
                });
                return self.empty(run, node);
            }
            srcs.push(src);
        }
        let mut driver = None;
        // the driver: the smallest entry that is already a count, a made table's rows or a view's kind bucket
        let mut best = usize::MAX;
        for (i, sr) in srcs.iter().enumerate() {
            let size = match sr {
                Src::Tab(t, _) => Some(t.n),
                Src::View(v, _) => kind_set(run, &v.filter, 0, v.base, &[])
                    .or_else(|| kind_set(run, conj, i, v.base, &v.map))
                    .map(|ks| bucket_len(f, run, &ks))
                    .or_else(|| lit_len(f, &v.filter, 0, v.base, &[]).or_else(|| lit_len(f, conj, i, v.base, &v.map))),
                _ => None,
            };
            if let Some(n) = size
                && n < best
            {
                best = n;
                driver = Some(i);
            }
        }
        let own_of = |d: usize| -> Vec<usize> { (0..conj.len()).filter(|c| refs[*c] & !(1u64 << d) == 0).collect() };
        // the driver: a made table (the smallest), else a view entered by a kind bucket, else a step not made yet (made
        // now: the first in order), else a view read whole
        let driver = match driver {
            Some(d) => d,
            None => match (0..k).find(|i| match &srcs[*i] {
                Src::View(v, _) => kind_set(run, &v.filter, 0, v.base, &[]).is_some() || kind_set(run, conj, *i, v.base, &v.map).is_some() || lit_applies(f, &v.filter, 0, v.base, &[]) || lit_applies(f, conj, *i, v.base, &v.map),
                _ => false,
            }) {
                Some(d) => d,
                None => match (0..k).find_map(|i| match &srcs[i] {
                    Src::Lazy(l) => Some((i, l.node)),
                    _ => None,
                }) {
                    Some((d, n)) => {
                        // a closure that no made or bucketed source drives is made whole and drives
                        let t = self.eval(f, run, p, n)?;
                        if t.n == 0 {
                            empty_sel::at(empty_sel::stage::PENDING_EMPTY);
                            return self.empty(run, node);
                        }
                        srcs[d] = Src::Tab(t, n);
                        d
                    }
                    None => (0..k).find(|i| matches!(srcs[*i], Src::View(..))).ok_or("a select with no source to start from")?,
                },
            },
        };
        let mut used = vec![false; conj.len()];
        let mut bound: u64 = 1 << driver;
        let mut tuples: Vec<Vec<u32>> = (0..k).map(|_| self.take_u32()).collect();
        let own = own_of(driver);
        let mut cur = vec![0u32; k];
        let first: cold_bucket::Rows = match &srcs[driver] {
            Src::Tab(t, _) => {
                // 8: a made table drives by position: its own tests run over its columns, no row list made
                let n = t.n;
                self.stats.touched[node] += n as u64;
                let mut kept = self.take_u32();
                if own.is_empty() {
                    kept.extend(0..n as u32);
                } else {
                    for r in 0..n as u32 {
                        cur[driver] = r;
                        if own.iter().all(|c| truthy(eval(run, f, &srcs, &cur, &conj[*c]))) {
                            kept.push(r);
                        }
                    }
                }
                tuples[driver] = kept;
                Rc::new(Vec::new()).into()
            }
            Src::View(v, vn) => {
                // the select's own kind tests narrow the view's entry too
                match kind_set(run, conj, driver, v.base, &v.map) {
                    Some(ks) if kind_set(run, &v.filter, 0, v.base, &[]).is_none() => {
                        let mut rows = bucket(f, run, &ks);
                        view_cols::retain(f, run, v, &mut rows);
                        Rc::new(rows).into()
                    },
                    _ => match lit_rows(f, conj, driver, v.base, &v.map) {
                        Some(mut rows) if !lit_applies(f, &v.filter, 0, v.base, &[]) => {
                            view_cols::retain(f, run, v, &mut rows);
                            Rc::new(rows).into()
                        }
                        _ => view_rows(f, run, v, *vn),
                    },
                }
            }
            _ => return Err("a select starts from no table or view".to_owned()),
        };
        self.stats.touched[node] += first.len() as u64;
        for &r in first.iter() {
            cur[driver] = r;
            if own.iter().all(|c| truthy(eval(run, f, &srcs, &cur, &conj[*c]))) {
                tuples[driver].push(r);
            }
        }
        for c in own {
            used[c] = true;
        }
        let mut count = tuples[driver].len();
        empty_sel::with(|m| {
            m.d0 = match &srcs[driver] {
                Src::Tab(t, _) => t.n as u32,
                _ => first.len() as u32,
            };
            m.d1 = count as u32;
            m.stage = if k == 1 { empty_sel::stage::ONE_SOURCE_FILTERED } else { empty_sel::stage::DRIVER_FILTERED };
        });
        while bound.count_ones() as usize != k {
            if count == 0 {
                for t in tuples {
                    self.give_u32(t);
                }
                return self.empty(run, node);
            }
            // the next source: one an unused equality joins to the bound ones, one probed in place first
            let mut pick: Option<(usize, usize, usize, bool)> = None;
            for (ci, c) in conj.iter().enumerate() {
                if used[ci] {
                    continue;
                }
                if let E::Cmp(Op::Eq, x, y) = c
                {
                    for (a, b) in [(x, y), (y, x)] {
                        if let E::Col(si, col, _) = **a
                            && bound & (1 << si) == 0
                            && refs_of(b) & !bound == 0
                            && refs_of(b) != 0
                        {
                            let native = match &srcs[si] {
                                Src::View(v, _) => File::probes(v.base, v.map[col]),
                                Src::Lazy(l) => col + 2 >= l.names.len(),
                                _ => false,
                            };
                            if pick.is_none_or(|q| native && !q.3) {
                                pick = Some((si, col, ci, native));
                            }
                        }
                    }
                }
            }
            let mut next: Vec<Vec<u32>> = (0..k).map(|_| self.take_u32()).collect();
            let fill_cur = |cur: &mut Vec<u32>, tuples: &Vec<Vec<u32>>, ti: usize, bound: u64| {
                for (s2, cur_s) in cur.iter_mut().enumerate() {
                    if bound & (1 << s2) != 0 {
                        *cur_s = tuples[s2][ti];
                    }
                }
            };
            match pick {
                Some((si, col, ci, native)) => {
                    used[ci] = true;
                    let other: &E = match &conj[ci] {
                        E::Cmp(_, x, y) => {
                            if matches!(**x, E::Col(a, b, _) if a == si && b == col) { y } else { x }
                        }
                        _ => unreachable!(),
                    };
                    // a view probed on a column it cannot be probed on in place is made whole, then indexed
                    if let Src::View(_, vn) = &srcs[si]
                        && !native
                    {
                        let vn = *vn;
                        srcs[si] = Src::Tab(self.eval(f, run, p, vn)?, vn);
                    }
                    // a closure is made for the ends this join asks, and only those
                    if let Src::Lazy(l) = &srcs[si] {
                        let l = l.clone();
                        let nk = l.keys.len();
                        let mut key_exprs: Vec<E> = Vec::new();
                        // redo-honest: a key column this join does not bind makes the closure whole (as a crossed closure
                        // is made), instead of refusing the rule
                        let mut unkeyed = false;
                        for kc in 0..nk {
                            let found = conj.iter().enumerate().find_map(|(cj, c)| match c {
                                E::Cmp(Op::Eq, x, y) if !used[cj] => {
                                    let hit = |a: &E, b: &E| matches!(a, E::Col(s3, c3, _) if *s3 == si && *c3 == kc) && refs_of(b) & !bound == 0;
                                    if hit(x, y) {
                                        Some((**y).clone())
                                    } else if hit(y, x) {
                                        Some((**x).clone())
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            });
                            match found {
                                Some(e) => key_exprs.push(e),
                                None if l.tys[kc] == Ty::File => key_exprs.push(E::Lit(V::new(Ty::File, u64::from(f.file)))),
                                None => {
                                    unkeyed = true;
                                    break;
                                }
                            }
                        }
                        if unkeyed {
                            srcs[si] = Src::Tab(self.eval(f, run, p, l.node)?, l.node);
                        } else {
                        let mut t = Table::with(l.names.clone(), l.tys.clone());
                        let mut asked = rustc_hash::FxHashSet::default();
                        let forward = col + 2 == l.names.len();
                        for ti in 0..count {
                            fill_cur(&mut cur, &tuples, ti, bound);
                            let mut key: Vec<u64> = key_exprs.iter().map(|e| eval(run, f, &srcs, &cur, e).1).collect();
                            let at = eval(run, f, &srcs, &cur, other).1;
                            key.push(at);
                            if asked.insert(key.clone()) {
                                for d in reach_dir(&l, &key, forward).iter() {
                                    let mut row = key[..nk].to_vec();
                                    if forward {
                                        row.extend([at, *d]);
                                    } else {
                                        row.extend([*d, at]);
                                    }
                                    t.push_row(&row);
                                }
                            }
                        }
                        self.stats.touched[node] += t.n as u64;
                        srcs[si] = Src::Tab(recycle::rc(t), usize::MAX - si);
                        }
                    }
                    let nb = bound | (1 << si);
                    // a made table is probed on every column an equality binds, at once
                    let mut tab_cols = vec![col];
                    let mut tab_exprs: Vec<&E> = vec![other];
                    if let Src::Tab(..) = &srcs[si] {
                        for (cj, c) in conj.iter().enumerate() {
                            if used[cj] {
                                continue;
                            }
                            if let E::Cmp(Op::Eq, x, y) = c
                            {
                                for (a2, b2) in [(x, y), (y, x)] {
                                    if let E::Col(s3, c3, _) = **a2
                                        && s3 == si
                                        && refs_of(b2) & !bound == 0
                                        && refs_of(b2) != 0
                                        && !tab_cols.contains(&c3)
                                    {
                                        tab_cols.push(c3);
                                        tab_exprs.push(&**b2);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    if tab_cols.len() > 1 {
                        used[ci] = false;
                    }
                    let checks: Vec<usize> = (0..conj.len()).filter(|c| !used[*c] && refs[*c] & !nb == 0).collect();
                    empty_sel::with(|m| {
                        m.o = match &srcs[si] {
                            Src::Tab(t, _) => t.n as u32,
                            _ => u32::MAX,
                        };
                        m.built |= match &srcs[si] {
                            Src::Tab(_, tn) if *tn > usize::MAX - 64 => empty_sel::stage::B_INDEX_FRESH,
                            Src::Tab(..) => empty_sel::stage::B_INDEX_FILE,
                            _ => empty_sel::stage::B_VIEW_PROBE,
                        };
                    });
                    let mut any_hit = false;
                    let index = if let Src::Tab(t, tn) = &srcs[si] {
                        Some(if *tn > usize::MAX - 64 { Rc::new(index_of(t, &tab_cols)) } else { table_index(f, *tn, t, &tab_cols) })
                    } else {
                        None
                    };
                    let mut probe_key: Vec<u64> = Vec::new();
                    let mut hits = Vec::new();
                    // 8: a key that is a made table's column is read from it, not through the expression
                    let direct: Option<(&[u64], usize)> = match other {
                        E::Col(s2, c2, _) => match &srcs[*s2] {
                            Src::Tab(t, _) => Some((&t.cols[*c2][..], *s2)),
                            _ => None,
                        },
                        _ => None,
                    };
                    for ti in 0..count {
                        fill_cur(&mut cur, &tuples, ti, bound);
                        let v = match direct {
                            Some((col_v, s2)) => {
                                let x = col_v[tuples[s2][ti] as usize];
                                V::new(Ty::Node, x)
                            }
                            None => eval(run, f, &srcs, &cur, other),
                        };
                        hits.clear();
                        match &srcs[si] {
                            Src::View(vs, _) => {
                                f.native_probe(run, vs.base, vs.map[col], v.1, &mut hits);
                                view_cols::retain(f, run, vs, &mut hits);
                            }
                            Src::Tab(..) => {
                                probe_key.clear();
                                probe_key.push(v.1);
                                for e in tab_exprs[1..].iter().copied() {
                                    probe_key.push(eval(run, f, &srcs, &cur, e).1);
                                }
                                if let Some(rows) = index.as_ref().and_then(|ix| ix.get(&key_of(probe_key.iter().copied()))) {
                                    hits.extend_from_slice(rows);
                                }
                            }
                            _ => unreachable!(),
                        }
                        self.stats.touched[node] += hits.len() as u64;
                        any_hit |= !hits.is_empty();
                        for h in &hits {
                            cur[si] = *h;
                            if checks.iter().all(|c| truthy(eval(run, f, &srcs, &cur, &conj[*c]))) {
                                for s2 in 0..k {
                                    if nb & (1 << s2) != 0 {
                                        next[s2].push(cur[s2]);
                                    }
                                }
                            }
                        }
                    }
                    for c in checks {
                        used[c] = true;
                    }
                    empty_sel::at(if any_hit { empty_sel::stage::RESIDUAL } else { empty_sel::stage::KEYS_MISS });
                    bound = nb;
                }
                None => {
                    // no key joins the next source: every pair
                    let si = (0..k).find(|s2| bound & (1 << s2) == 0).expect("an unbound source");
                    if let Src::Lazy(l) = &srcs[si] {
                        let ln = l.node;
                        srcs[si] = Src::Tab(self.eval(f, run, p, ln)?, ln);
                    }
                    let nb = bound | (1 << si);
                    empty_sel::at(empty_sel::stage::CROSS);
                    let checks: Vec<usize> = (0..conj.len()).filter(|c| !used[*c] && refs[*c] & !nb == 0).collect();
                    let rows: cold_bucket::Rows = match &srcs[si] {
                        Src::Tab(t, _) => Rc::new((0..t.n as u32).collect::<Vec<_>>()).into(),
                        Src::View(v, vn) => view_rows(f, run, v, *vn),
                        _ => unreachable!(),
                    };
                    for ti in 0..count {
                        fill_cur(&mut cur, &tuples, ti, bound);
                        self.stats.touched[node] += rows.len() as u64;
                        for r in rows.iter() {
                            cur[si] = *r;
                            if checks.iter().all(|c| truthy(eval(run, f, &srcs, &cur, &conj[*c]))) {
                                for s2 in 0..k {
                                    if nb & (1 << s2) != 0 {
                                        next[s2].push(cur[s2]);
                                    }
                                }
                            }
                        }
                    }
                    for c in checks {
                        used[c] = true;
                    }
                    bound = nb;
                }
            }
            count = next[driver].len();
            for t in std::mem::replace(&mut tuples, next) {
                self.give_u32(t);
            }
        }
        if used.iter().any(|u| !u) {
            return Err("a test no source binds".to_owned());
        }
        let mut out = self.empty(run, node)?;
        for (c, (_, e)) in sp.items.iter().enumerate() {
            let col = &mut out.cols[c];
            col.reserve(count);
            // 8: a picked column is gathered through its source's row list
            if let E::Col(si, sc, _) = e {
                match &srcs[*si] {
                    Src::Tab(t, _) => {
                        let src = &t.cols[*sc][..];
                        col.extend(tuples[*si][..count].iter().map(|r| src[*r as usize]));
                        continue;
                    }
                    Src::View(v, _) => {
                        let (base, bc) = (v.base, v.map[*sc]);
                        view_cols::gather(f, run, base, bc, &tuples[*si][..count], col);
                        continue;
                    }
                    _ => {}
                }
            }
            for ti in 0..count {
                for (s2, cur_s) in cur.iter_mut().enumerate() {
                    *cur_s = tuples[s2][ti];
                }
                col.push(eval(run, f, &srcs, &cur, e).1);
            }
        }
        out.n = count;
        for t in tuples {
            self.give_u32(t);
        }
        // 7: a projection of one source that drops columns is a set, as the engine's relations are (`VEC_SELECT_BAG`: carried as a bag, the interpreter before 7)
        if !sw!("VEC_SELECT_BAG")
            && k == 1
            && sp.items.iter().all(|(_, e)| matches!(e, E::Col(0, _, _)))
            && match &srcs[0] {
                Src::Tab(t, _) => sp.items.len() < t.names.len(),
                Src::View(v, _) => sp.items.len() < v.map.len(),
                _ => false,
            }
        {
            return Ok(distinct(out));
        }
        Ok(out)
    }

    fn axis(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, a: &Args) -> R<Table> {
        let from = self.eval(f, run, p, self.arg_rel(node, "from")?)?;
        self.axis_over(f, run, p, node, a, &from)
    }

    /// The hierarchy an axis walks: the tree, a view probed by its child and parent columns, or a made table indexed once.
    fn hier(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, rel: usize) -> R<Hier> {
        if !f.after_last_file && self.plan.nodes[rel].op() == "base" && native_of(run, self.plan.nodes[rel].base().unwrap_or("")) == Some(Native::Child) {
            return Ok(Hier::Tree);
        }
        let rsc = self.schema(run, rel)?;
        let (pc, cc) = (pos(&rsc.names, "parent")?, pos(&rsc.names, "child")?);
        if let Some(input)=f.population(rel) {
            return Ok(Hier::Population(input.clone(),pc,cc));
        }
        Ok(match self.res(f, run, p, rel)? {
            Res::View(v) if File::probes(v.base, v.map[pc]) && File::probes(v.base, v.map[cc]) => Hier::View(v, pc, cc),
            _ => {
                let t = self.eval(f, run, p, rel)?;
                let mut up: FxHashMap<u64, Vec<u64>> = FxHashMap::default();
                let mut down: FxHashMap<u64, Vec<u64>> = FxHashMap::default();
                for r in 0..t.n {
                    up.entry(t.cols[cc][r]).or_default().push(t.cols[pc][r]);
                    down.entry(t.cols[pc][r]).or_default().push(t.cols[cc][r]);
                }
                Hier::Maps(up, down)
            }
        })
    }

    /// The domain an axis admits by: everything, a set, or a view probed by id.
    fn dom(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize) -> R<Dom> {
        let plan: &'p Plan = self.plan;
        let Ok(dn) = self.arg_rel(node, "domain") else { return Ok(Dom::All) };
        let sc = self.schema(run, dn)?;
        let c = pos(&sc.names, "id")?;
        Ok(match self.res(f, run, p, dn)? {
            Res::View(v) if File::probes(v.base, v.map[c]) => Dom::View(v, c),
            _ => {
                let d = self.eval(f, run, p, dn)?;
                Dom::Set(d.cols[c].iter().copied().collect())
            }
        })
    }

    /// Which of `xs` the domain admits, asked as one batch.
    fn admit(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, dom: &Dom, xs: &[u64]) -> R<rustc_hash::FxHashSet<u64>> {
        Ok(match dom {
            Dom::All => xs.iter().copied().collect(),
            Dom::Set(s) => xs.iter().copied().filter(|x| s.contains(x)).collect(),
            Dom::View(v, c) => {
                let mut hits = Vec::new();
                let mut out = rustc_hash::FxHashSet::default();
                for x in xs {
                    hits.clear();
                    f.native_probe(run, v.base, v.map[*c], *x, &mut hits);
                    if hits.iter().any(|h| view_holds(f, run, v, *h)) {
                        out.insert(*x);
                    }
                }
                out
            }
        })
    }

    /// The axis over given start rows (made, or a probed subset).
    fn axis_over(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, a: &Args, from: &Table) -> R<Table> {
        let rel = self.arg_rel(node, "relation")?;
        let dir = s(&a["direction"])?.to_owned();
        let mut out = self.empty(run, node)?;
        if from.n == 0 {
            return Ok(out);
        }
        if let Some(t) = self.walk_axis(f, run, p, node, a, from)? {
            return Ok(t);
        }
        let dom = self.dom(f, run, p, node)?;
        let hier = self.hier(f, run, p, rel)?;
        let ic = from.col("id")?;
        let mut row = Vec::new();
        let mut nbrs = Vec::new();
        // nearest ancestor: walked level by level for all starts at once, the domain asked once a level
        if dir == "nearest-ancestor" {
            let mut found: Vec<u64> = vec![NULL; from.n];
            let mut frontier: Vec<(usize, u64)> = (0..from.n).map(|i| (i, from.cols[ic][i])).collect();
            for _ in 0..4096 {
                if frontier.is_empty() {
                    break;
                }
                let mut next: Vec<(usize, u64)> = Vec::new();
                for (i, x) in &frontier {
                    hier_step(f, run, &hier, *x, true, &mut nbrs);
                    for q in &nbrs {
                        next.push((*i, *q));
                    }
                }
                self.stats.touched[node] += next.len() as u64;
                let qs: Vec<u64> = next.iter().map(|x| x.1).collect();
                let ok = self.admit(f, run, p, &dom, &qs)?;
                frontier.clear();
                for (i, q) in next {
                    if found[i] != NULL {
                        continue;
                    }
                    if ok.contains(&q) {
                        found[i] = q;
                    } else {
                        frontier.push((i, q));
                    }
                }
                frontier.retain(|(i, _)| found[*i] == NULL);
            }
            for (i, q) in found.iter().enumerate() {
                if *q != NULL {
                    from.row(i, &mut row);
                    row.push(*q);
                    out.push_row(&row);
                }
            }
            return Ok(out);
        }
        let mut all: Vec<(usize, u64)> = Vec::new();
        // redo-alloc: one walk's set and frontier, emptied a start (not made a start); the neighbours read in place
        let mut seen = rustc_hash::FxHashSet::default();
        let mut frontier: Vec<u64> = Vec::new();
        for i in 0..from.n {
            let x = from.cols[ic][i];
            match dir.as_str() {
                "parent" | "ancestor" => {
                    seen.clear();
                    frontier.clear();
                    frontier.push(x);
                    while let Some(at) = frontier.pop() {
                        hier_step(f, run, &hier, at, true, &mut nbrs);
                        for qi in 0..nbrs.len() {
                            let q = nbrs[qi];
                            if seen.insert(q) {
                                all.push((i, q));
                                if dir != "parent" {
                                    frontier.push(q);
                                }
                            }
                        }
                    }
                }
                "descendant" | "child" => {
                    if matches!(hier, Hier::Tree) && (x as usize) < f.n
                        && (dir == "child" || node_table::node_count(f.table).0 == f.n)
                    {
                        if dir == "child" {
                            for c in node_table::children_of(f.table, Position(x as u32)) {
                                all.push((i, u64::from(c.0)));
                            }
                        } else {
                            let end = node_table::subtree_end_of(f.table, Position(x as u32)).0;
                            all.extend((x as u32 + 1..end).map(|q| (i, u64::from(q))));
                        }
                    } else {
                        seen.clear();
                        frontier.clear();
                        frontier.push(x);
                        while let Some(at) = frontier.pop() {
                            hier_step(f, run, &hier, at, false, &mut nbrs);
                            for ci in 0..nbrs.len() {
                                let c = nbrs[ci];
                                if seen.insert(c) {
                                    all.push((i, c));
                                    if dir != "child" {
                                        frontier.push(c);
                                    }
                                }
                            }
                        }
                    }
                }
                "next-sibling" | "previous-sibling" => {
                    if !matches!(hier, Hier::Tree) {
                        return Err("sibling axis requires the concrete ordered children".into());
                    }
                    if (x as usize) < f.n && f.has_parent(x as u32) {
                        let sibs = node_table::children_of(f.table, Position(f.parent[x as usize]));
                        if let Some(at) = sibs.iter().position(|c| u64::from(c.0) == x) {
                            let offset = if dir == "next-sibling" { 1 } else { -1 };
                            if let Some(sibling) = at.checked_add_signed(offset).and_then(|j| sibs.get(j)) {
                                all.push((i, u64::from(sibling.0)));
                            }
                        }
                    }
                }
                other => return Err(format!("axis direction {other}")),
            }
        }
        self.stats.touched[node] += all.len() as u64;
        let qs: Vec<u64> = all.iter().map(|x| x.1).collect();
        let ok = self.admit(f, run, p, &dom, &qs)?;
        for (i, q) in all {
            if ok.contains(&q) {
                from.row(i, &mut row);
                row.push(q);
                out.push_row(&row);
            }
        }
        Ok(out)
    }

    fn ordered(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, a: &Args, as_of: bool, incomplete: Option<&std::collections::BTreeSet<Option<u32>>>) -> R<Table> {
        let l = self.eval(f, run, p, self.arg_rel(node, "left")?)?;
        let rn = self.arg_rel(node, "right")?;
        let lk: Vec<usize> = strs(&a["left-on"])?.map(|c| l.col(c)).collect::<R<_>>()?;
        let rsc = self.schema(run, rn)?;
        let rk0: Vec<usize> = strs(&a["right-on"])?.map(|c| pos(&rsc.names, c)).collect::<R<_>>()?;
        let r = self.eval(f, run, p, rn)?;
        if r.n == 0 {
            return self.empty(run, node);
        }
        let rk: Vec<usize> = strs(&a["right-on"])?.map(|c| r.col(c)).collect::<R<_>>()?;
        let (lo, ro) = (l.col(s(&a["left-order"])?)?, r.col(s(&a["right-order"])?)?);
        let outm = a["output"].as_object().ok_or("output")?;
        let mut picks = Vec::new();
        for (_, v) in outm {
            let side = s(&v[0])?;
            picks.push((side == "left", if side == "left" { l.col(s(&v[1])?)? } else { r.col(s(&v[1])?)? }));
        }
        let mut out = self.empty(run, node)?;
        // redo-alloc: a row's key read into one buffer; a key list made only for a key not seen (same keys, same order)
        let mut kb: Vec<u64> = Vec::with_capacity(rk.len().max(lk.len()));
        // redo-alloc: keys inline (inline_key.rs: hashed as the Vec was, so the same order), a group's first row inline
        let mut groups: FxHashMap<inline_key::Key, inline_key::List<(i64, u32)>> = FxHashMap::default();
        let scoped_key = incomplete.and_then(|files| {
            rk.iter().copied().find(|&key| matches!(r.tys[key], Ty::File | Ty::Node))
                .map(|key| (files, key))
        });
        if incomplete.is_some() && scoped_key.is_none() {
            return Ok(out);
        }
        for i in 0..r.n {
            if let Some((files, key)) = scoped_key {
                let word = r.cols[key][i];
                let file = if r.tys[key] == Ty::Node { word >> 32 } else { word };
                if word == NULL || files.contains(&u32::try_from(file).ok()) { continue; }
            }
            kb.clear();
            kb.extend(rk.iter().map(|c| r.cols[*c][i]));
            let v = (r.cols[ro][i] as i64, i as u32);
            match groups.get_mut(&kb[..]) {
                Some(g) => g.push(v),
                None => {
                    groups.insert(inline_key::Key::of(&kb), inline_key::List::One([v]));
                }
            }
        }
        for g in groups.values_mut() {
            g.sort_unstable();
        }
        let mut lefts: FxHashMap<inline_key::Key, inline_key::List<(i64, u32)>> = FxHashMap::default();
        for i in 0..l.n {
            kb.clear();
            kb.extend(lk.iter().map(|c| l.cols[*c][i]));
            let v = (l.cols[lo][i] as i64, i as u32);
            match lefts.get_mut(&kb[..]) {
                Some(g) => g.push(v),
                None => {
                    lefts.insert(inline_key::Key::of(&kb), inline_key::List::One([v]));
                }
            }
        }
        let row = RefCell::new(Vec::with_capacity(picks.len()));
        let emit = |out: &mut Table, li: u32, ri: u32| {
            let mut row = row.borrow_mut();
            row.clear();
            row.extend(picks.iter().map(|(left, c)| if *left { l.cols[*c][li as usize] } else { r.cols[*c][ri as usize] }));
            out.push_row(&row);
        };
        if as_of {
            let bound = s(&a["bound"])?;
            for (k, ls) in &lefts {
                let Some(g) = groups.get(k) else { continue };
                for (o, li) in ls.iter() {
                    let (lo_i, hi_i) = (g.partition_point(|x| x.0 < *o), g.partition_point(|x| x.0 <= *o));
                    let target = match bound {
                        "before" => (lo_i > 0).then(|| g[lo_i - 1].0),
                        "at-or-before" => (hi_i > 0).then(|| g[hi_i - 1].0),
                        "after" => g.get(hi_i).map(|x| x.0),
                        "at-or-after" => g.get(lo_i).map(|x| x.0),
                        other => return Err(format!("as-of bound {other}")),
                    };
                    if let Some(t) = target {
                        let a0 = g.partition_point(|x| x.0 < t);
                        for x in &g[a0..] {
                            if x.0 != t {
                                break;
                            }
                            emit(&mut out, *li, x.1);
                        }
                    }
                }
            }
        } else {
            for (k, ls) in &mut lefts {
                let Some(g) = groups.get(k) else { continue };
                ls.sort_unstable();
                let mut at = 0;
                for (o, li) in ls.iter() {
                    while at < g.len() && g[at].0 <= *o {
                        at += 1;
                    }
                    if at < g.len() {
                        emit(&mut out, *li, g[at].1);
                        at += 1;
                    }
                }
            }
        }
        Ok(out)
    }

    fn source_matches(&mut self, f: &mut File<'_>, run: &mut Run, p: &mut dyn Provider, node: usize, a: &Args) -> R<Table> {
        let from = self.eval(f, run, p, self.arg_rel(node, "from")?)?;
        let sc = from.col(s(&a["span"])?)?;
        let re = run.re(s(&a["pattern"])?)?;
        let base_line = match a.get("base-line") {
            Some(Param::Text(b)) => Some(from.col(b)?),
            _ => None,
        };
        let line_out = a.get("line-output").and_then(Param::as_str).is_some();
        let cap = a.get("capture").and_then(Param::as_str).map(str::to_owned);
        let cap_text = a.get("capture-text-output").and_then(Param::as_str).is_some();
        let cap_line = a.get("capture-line-output").and_then(Param::as_str).is_some();
        let groups: Vec<String> = a.get("capture-texts").and_then(Param::as_object).map(|o| o.keys().cloned().collect()).unwrap_or_default();
        let mut out = self.empty(run, node)?;
        let mut row = Vec::new();
        for i in 0..from.n {
            let sp = from.cols[sc][i];
            if sp == NULL {
                continue;
            }
            let (st, en) = ((sp >> 32) as usize, (sp & 0xffff_ffff) as usize);
            let text = f.src.get(st..en).ok_or("source match span outside file")?;
            let line0 = base_line.map(|b| from.cols[b][i] as i64);
            // 7: the newlines before `off`, counted on from the last offset asked (matches come in order), not from the start
            let counted = std::cell::Cell::new((0usize, 0i64));
            let line_at = |off: usize| -> i64 {
                let (mut at, mut lines) = counted.get();
                if off < at {
                    (at, lines) = (0, 0);
                }
                lines += text[at..off].iter().filter(|c| **c == b'\n').count() as i64;
                counted.set((off, lines));
                line0.unwrap_or(0) + lines
            };
            for m in re.pattern.captures_iter(text) {
                let w = m.get(0).expect("whole");
                let g = match &cap {
                    Some(c) => match m.name(c) {
                        Some(g) => Some(g),
                        None => continue,
                    },
                    None => None,
                };
                from.row(i, &mut row);
                row.push((((st + w.start()) as u64) << 32) | (st + w.end()) as u64);
                if line_out {
                    row.push(line_at(w.start()) as u64);
                }
                if let Some(g) = g {
                    row.push((((st + g.start()) as u64) << 32) | (st + g.end()) as u64);
                    if cap_text {
                        row.push(u64::from(run.sym(g.as_bytes())));
                    }
                    if cap_line {
                        row.push(line_at(g.start()) as u64);
                    }
                }
                for c in &groups {
                    row.push(u64::from(run.sym(m.name(c).map_or(&b""[..], |x| x.as_bytes()))));
                }
                out.push_row(&row);
            }
        }
        Ok(out)
    }
}

/// The hierarchy an axis walks.
pub(super) enum Hier {
    Tree,
    View(Rc<ViewSpec>, usize, usize),
    Maps(FxHashMap<u64, Vec<u64>>, FxHashMap<u64, Vec<u64>>),
    Population(Rc<population::Population>,usize,usize),
}

/// The domain an axis admits by.
pub(super) enum Dom {
    All,
    Set(rustc_hash::FxHashSet<u64>),
    View(Rc<ViewSpec>, usize),
}

/// The other end of each edge from `x`: upward (its parents) or downward (its children).
fn hier_step(f: &File<'_>, run: &mut Run, h: &Hier, x: u64, up: bool, out: &mut Vec<u64>) {
    out.clear();
    match h {
        Hier::Tree => {
            if (x as usize) < f.n {
                if up {
                    if f.has_parent(x as u32) {
                        out.push(u64::from(f.parent[x as usize]));
                    }
                } else {
                    out.extend(node_table::children_of(f.table, Position(x as u32)).iter().map(|c| u64::from(c.0)));
                }
            }
        }
        Hier::View(v, pc, cc) => {
            // redo-alloc: the probe's rows in the worker's one buffer (not a list made a step)
            thread_local! {
                static HITS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
            }
            let mut hits = HITS.with(|h| std::mem::take(&mut *h.borrow_mut()));
            hits.clear();
            let (by, to) = if up { (*cc, *pc) } else { (*pc, *cc) };
            f.native_probe(run, v.base, v.map[by], x, &mut hits);
            for &h in &hits {
                if view_holds(f, run, v, h) {
                    out.push(view_cols::cell(f, run, v.base, h, v.map[to]));
                }
            }
            HITS.with(|b| *b.borrow_mut() = hits);
        }
        Hier::Population(input, parent, child) => {
            let (by,to) = if up { (*child,*parent) } else { (*parent,*child) };
            input.neighbors(by,to,x,out);
        }
        Hier::Maps(u, d) => {
            if let Some(xs) = if up { u.get(&x) } else { d.get(&x) } {
                out.extend_from_slice(xs);
            }
        }
    }
}

/// Whether a view's tests hold on one of its base rows.
fn view_holds(f: &File<'_>, run: &mut Run, v: &ViewSpec, row: u32) -> bool {
    fill_prof::read(1);
    if v.filter.is_empty() {
        return true;
    }
    view_cols::row_holds(f, run, v, row)
}

/// A view's rows in this file: its kind bucket when its tests name kinds, else every base row; tested; made once a file.
fn view_rows<'a>(f: &mut File<'a>, run: &mut Run, v: &ViewSpec, node: usize) -> cold_bucket::Rows<'a> {
    view_rows_for(f, run, v, node, false)
}

fn view_rows_for<'a>(f: &mut File<'a>, run: &mut Run, v: &ViewSpec, node: usize, materialize: bool) -> cold_bucket::Rows<'a> {
    fill_prof::read(3);
    if cold_list::auditing() { run.cold.ask(f, node); }
    if let Some(r) = f.vrows.slot(&node) {
        return r.resolve(f.table);
    }
    let cold_start = cold_list::clock();
    let start = if sw!("VEC_LEAF_PROFILE") { step_clock() } else { 0.0 };
    let mut entered = 0;
    let mut tests = 0;
    let slot = if v.cold_file_test && filter_order::file_fails(run, f, &v.filter) {
        cold_bucket::Slot::Empty
    } else if let Some(bucket) = v.cold_bucket.get() {
        let slot = bucket.select(f);
        if sw!("VEC_COLD_COUNT") {
            let mut original = match kind_set(run, &v.filter, 0, v.base, &[]) {
                Some(ks) => self::bucket(f, run, &ks),
                None => f.native_rows(run, v.base),
            };
            view_cols::retain_original(f, run, v, &mut original);
            assert_eq!(slot.resolve(f.table).shadow(), original, "bucket entry file {}, node {node}", f.file);
        }
        slot
    } else if let Some((rows, visits, attempts)) = cold_test::column_rows(f, run, v, node) {
        entered = visits;
        tests = attempts;
        rows.into()
    } else {
        recycle::rows_with(|out| {
            let before = if sw!("VEC_COLD_PHASE") { cold_list::clock() } else { 0.0 };
            match kind_set(run, &v.filter, 0, v.base, &[]) {
                Some(ks) => bucket_into(f, run, &ks, out),
                None => match lit_rows(f, &v.filter, 0, v.base, &[]) {
                    Some(r) => out.extend_from_slice(&r),
                    None => f.native_rows_into(run, v.base, out),
                },
            }
            // redo-fills: the view tests read as columns (view_cols)
            let middle = if sw!("VEC_COLD_PHASE") { cold_list::clock() } else { 0.0 };
            entered = out.len();
            tests = cold_test::retain(f, run, v, node, materialize, out);
            if sw!("VEC_COLD_PHASE") { run.cold.phase(node, middle - before, cold_list::clock() - middle); }
        }).into()
    };
    operators::rows_in(entered);
    let rows = slot.resolve(f.table);
    if sw!("VEC_LEAF_PROFILE") {
        leaf_scan::row_list(node, materialize, entered, rows.len(), start);
    }
    f.vrows.insert(node, slot);
    if cold_list::auditing() { run.cold.built(f, node, materialize, entered, tests, &rows, cold_list::clock() - cold_start); }
    rows
}

/// How many elements some kinds have.
fn bucket_len(f: &File<'_>, run: &Run, ks: &[u32]) -> usize {
    // engine-vec 8: many kinds by one scan of the kind table
    if ks.len() > 16 {
        return run.kind_sym.iter().enumerate().filter(|(_, s)| ks.contains(s)).map(|(c, _)| node_table::ids_of_kind(f.table, KindBits(c as u16)).len()).sum();
    }
    // pass 2: by the kinds' codes (not a scan of every kind a call); each code once, as the scan counted it
    let codes = kind_codes(run);
    let mut n = 0;
    for (i, s) in ks.iter().enumerate() {
        if ks[..i].contains(s) {
            continue;
        }
        for c in codes.get(s).map_or(&[][..], |b| &b[..]) {
            n += node_table::ids_of_kind(f.table, KindBits(*c)).len();
        }
    }
    n
}

/// The kind codes by kind-name symbol (once a run).
fn kind_codes(run: &Run) -> &FxHashMap<u32, Box<[u16]>> {
    run.kind_codes.get_or_init(|| {
        let mut m: FxHashMap<u32, Vec<u16>> = FxHashMap::default();
        for (c, s) in run.kind_sym.iter().enumerate() {
            m.entry(*s).or_default().push(c as u16);
        }
        m.into_iter().map(|(k, v)| (k, v.into_boxed_slice())).collect()
    })
}

/// The elements of some kinds, in position order.
fn bucket(f: &File<'_>, run: &Run, ks: &[u32]) -> Vec<u32> {
    let mut rows = Vec::new();
    bucket_into(f, run, ks, &mut rows);
    rows
}

/// redo-alloc: `bucket` into `rows` (empty).
fn bucket_into(f: &File<'_>, run: &Run, ks: &[u32], rows: &mut Vec<u32>) {
    let codes = kind_codes(run);
    for (i, s) in ks.iter().enumerate() {
        if ks[..i].contains(s) {
            continue;
        }
        for c in codes.get(s).map_or(&[][..], |b| &b[..]) {
            rows.extend(node_table::ids_of_kind(f.table, KindBits(*c)).iter().map(|p| p.0));
        }
    }
    rows.sort_unstable();
}

/// 6h: a fact view's rows where a test fixes one of its columns to literals (`col = "x"`, `col in [..]`, an or of such): that
/// column's keyed list read at those values, a per-file bucket. `map` takes a view's column to its base column (empty: the
/// base's own).
fn lit_rows(f: &File<'_>, conj: &[E], si: usize, base: Native, map: &[usize]) -> Option<Vec<u32>> {
    fill_prof::read(2);
    let (i, bc, vals) = lit_pick(f, conj, si, base, map)?;
    let ix = f.fact_list(i, bc);
    let mut rows: Vec<u32> = vals.iter().filter_map(|v| ix.get(&u64::from(*v))).flatten().copied().collect();
    rows.sort_unstable();
    rows.dedup();
    Some(rows)
}

/// redo-fills c5: whether `lit_rows` answers (a literal test on a symbol column of a fill), without making its rows.
fn lit_applies(f: &File<'_>, conj: &[E], si: usize, base: Native, map: &[usize]) -> bool {
    lit_pick(f, conj, si, base, map).is_some()
}

/// redo-fills c5: how many rows `lit_rows` answers, without making them (a row holds one value of the column, so the
/// distinct literals' row lists are disjoint).
fn lit_len(f: &File<'_>, conj: &[E], si: usize, base: Native, map: &[usize]) -> Option<usize> {
    let (i, bc, mut vals) = lit_pick(f, conj, si, base, map)?;
    vals.sort_unstable();
    vals.dedup();
    let ix = f.fact_list(i, bc);
    Some(vals.iter().filter_map(|v| ix.get(&u64::from(*v))).map(|r| r.len()).sum())
}

/// redo-alloc: `lit_rows` into `out` (empty; whether a test fixes a column), through redo-fills' `lit_pick`.
fn lit_rows_into(f: &File<'_>, conj: &[E], si: usize, base: Native, map: &[usize], rows: &mut Vec<u32>) -> bool {
    let Some((i, bc, vals)) = lit_pick(f, conj, si, base, map) else { return false };
    let ix = f.fact_list(i, bc);
    rows.extend(vals.iter().filter_map(|v| ix.get(&u64::from(*v))).flatten().copied());
    rows.sort_unstable();
    rows.dedup();
    true
}

/// The literal test `lit_rows` reads by: the fill, its base column and the literals.
fn lit_pick(f: &File<'_>, conj: &[E], si: usize, base: Native, map: &[usize]) -> Option<(u16, usize, Vec<u32>)> {
    let Native::Fact(i) = base else { return None };
    lit_pick_for(conj, si, map, |bc| f.facts[usize::from(i)].as_ref().is_some_and(|t| t.tys[bc] == Ty::Sym))
        .map(|(column, values)| (i, column, values))
}

fn lit_pick_for(conj: &[E], si: usize, map: &[usize], symbol_column: impl Fn(usize) -> bool) -> Option<(usize, Vec<u32>)> {
    fn cols(e: &E, si: usize, out: &mut Vec<usize>) {
        match e {
            E::Col(s, c, _) if *s == si => out.push(*c),
            E::Cmp(_, a, b) | E::And(a, b) | E::Or(a, b) | E::In(a, b, _) => {
                cols(a, si, out);
                cols(b, si, out);
            }
            _ => {}
        }
    }
    // redo-alloc: the columns and literals of each conjunct in the worker's two buffers (none made a conjunct)
    thread_local! {
        static BUF: RefCell<(Vec<usize>, Vec<u32>)> = const { RefCell::new((Vec::new(), Vec::new())) };
    }
    let (mut cs, mut vals) = BUF.with(|b| std::mem::take(&mut *b.borrow_mut()));
    let mut out = None;
    'conj: for e in conj {
        cs.clear();
        cols(e, si, &mut cs);
        for &c in &cs {
            vals.clear();
            if kinds_required_into(e, si, c, &mut vals) {
                let bc = if map.is_empty() { c } else { map[c] };
                // a text literal names a symbol cell only; a flag or number column is tested row by row
                if !symbol_column(bc) {
                    continue;
                }
                // redo-fills c5 (the pick without its rows) in redo-alloc's buffers: the literals copied out, the buffers kept
                out = Some((bc, vals.clone()));
                break 'conj;
            }
        }
    }
    BUF.with(|b| *b.borrow_mut() = (cs, vals));
    out
}

/// The kinds a conjunction requires of a tree source's `kind` column (a literal or an or of literals), if any. `map` takes a
/// view's column to its base column (empty: the columns are the base's).
fn kind_set(_run: &Run, conj: &[E], si: usize, base: Native, map: &[usize]) -> Option<Vec<u32>> {
    if base != Native::Tree {
        return None;
    }
    let is_kind = |c: usize| if map.is_empty() { c == 1 } else { map.get(c) == Some(&1) };
    fn kinds(e: &E, si: usize, is_kind: &dyn Fn(usize) -> bool, out: &mut Vec<u32>) -> bool {
        match e {
            E::Cmp(Op::Eq, a, b) => match (&**a, &**b) {
                (E::Col(s, c, _), E::Text(t)) | (E::Text(t), E::Col(s, c, _)) if *s == si && is_kind(*c) => {
                    out.push(*t);
                    true
                }
                _ => false,
            },
            E::Or(a, b) => kinds(a, si, is_kind, out) && kinds(b, si, is_kind, out),
            E::In(a, b, false) => match (&**a, &**b) {
                (E::Col(s, c, _), E::List(items)) if *s == si && is_kind(*c) => items.iter().all(|i| {
                    if let E::Text(t) = i {
                        out.push(*t);
                        true
                    } else {
                        false
                    }
                }),
                _ => false,
            },
            _ => false,
        }
    }
    for c in conj {
        let mut ks = Vec::new();
        if kinds(c, si, &is_kind, &mut ks) {
            return Some(ks);
        }
    }
    None
}

/// The kinds a test requires of column `kc` of source `si`: a literal, a list, an `or` of such, or an `and` holding one.
fn kinds_required(e: &E, si: usize, kc: usize) -> Option<Vec<u32>> {
    let mut v = Vec::new();
    kinds_required_into(e, si, kc, &mut v).then_some(v)
}

/// redo-alloc: `kinds_required` into `out` (appended; whether it holds, `out` as it was when not).
fn kinds_required_into(e: &E, si: usize, kc: usize, out: &mut Vec<u32>) -> bool {
    let at = out.len();
    let ok = match e {
        E::Cmp(Op::Eq, a, b) => match (&**a, &**b) {
            (E::Col(s, c, _), E::Text(t)) | (E::Text(t), E::Col(s, c, _)) if *s == si && *c == kc => {
                out.push(*t);
                true
            }
            _ => false,
        },
        E::In(a, b, false) => match (&**a, &**b) {
            (E::Col(s, c, _), E::List(items)) if *s == si && *c == kc => items.iter().all(|i| {
                if let E::Text(t) = i {
                    out.push(*t);
                    true
                } else {
                    false
                }
            }),
            _ => false,
        },
        E::Or(a, b) => kinds_required_into(a, si, kc, out) && kinds_required_into(b, si, kc, out),
        E::And(a, b) => kinds_required_into(a, si, kc, out) || {
            out.truncate(at);
            kinds_required_into(b, si, kc, out)
        },
        _ => false,
    };
    if !ok {
        out.truncate(at);
    }
    ok
}

/// An expression over a view's columns, rewritten over its base's.
fn remap(e: &E, map: &[usize]) -> E {
    let b = |x: &E| Box::new(remap(x, map));
    match e {
        E::Col(s, c, t) => E::Col(*s, map[*c], *t),
        E::List(v) => E::List(v.iter().map(|x| remap(x, map)).collect()),
        E::Call(n, v) => E::Call(n.clone(), v.iter().map(|x| remap(x, map)).collect()),
        E::Cmp(o, x, y) => E::Cmp(*o, b(x), b(y)),
        E::In(x, y, n) => E::In(b(x), b(y), *n),
        E::And(x, y) => E::And(b(x), b(y)),
        E::Or(x, y) => E::Or(b(x), b(y)),
        E::Not(x) => E::Not(b(x)),
        E::TextTest(x, r) => E::TextTest(b(x), r.clone()),
        other => other.clone(),
    }
}


/// A union's rows as a set, as the language's `union` is (`set_rows`): the first of equal rows kept, in order.
fn distinct(t: Table) -> Table {
    if sw!("VEC_UNION_ALL") {
        return t;
    }
    let mut keep = Vec::with_capacity(t.n);
    // 7: one or two columns are hashed as words, not as a list a row
    match t.cols.len() {
        1 => {
            let mut seen: rustc_hash::FxHashSet<u64> = rustc_hash::FxHashSet::default();
            keep.extend((0..t.n).filter(|r| seen.insert(t.cols[0][*r])));
        }
        2 => {
            let mut seen: rustc_hash::FxHashSet<(u64, u64)> = rustc_hash::FxHashSet::default();
            keep.extend((0..t.n).filter(|r| seen.insert((t.cols[0][*r], t.cols[1][*r]))));
        }
        _ => {
            // redo-alloc: a row read into one buffer, its key inline (inline_key.rs)
            let mut seen: rustc_hash::FxHashSet<inline_key::Key> = rustc_hash::FxHashSet::default();
            let mut row: Vec<u64> = Vec::with_capacity(t.cols.len());
            for r in 0..t.n {
                row.clear();
                row.extend(t.cols.iter().map(|c| c[r]));
                if !seen.contains(&row[..]) {
                    seen.insert(inline_key::Key::of(&row));
                    keep.push(r);
                }
            }
        }
    }
    if keep.len() == t.n {
        return t;
    }
    let mut out = Table::with_rows(t.names.clone(), t.tys.clone(), keep.len());
    for (c, col) in t.cols.iter().enumerate() {
        out.cols[c].extend(keep.iter().map(|r| col[*r]));
    }
    out.n = keep.len();
    out
}

fn reach_dir(l: &Lazy, key: &[u64], forward: bool) -> Rc<Vec<u64>> {
    let (cache, adj) = if forward { (&l.cache, &l.adj) } else { (&l.rcache, &l.radj) };
    if let Some(r) = cache.borrow().get(key) {
        return r.clone();
    }
    let kp = &key[..key.len() - 1];
    let mut seen = rustc_hash::FxHashSet::default();
    let mut out = Vec::new();
    let mut stack = vec![key[key.len() - 1]];
    while let Some(x) = stack.pop() {
        let mut kx = kp.to_vec();
        kx.push(x);
        if let Some(ns) = adj.get(&kx) {
            for y in ns {
                if seen.insert(*y) {
                    out.push(*y);
                    stack.push(*y);
                }
            }
        }
    }
    let rc = Rc::new(out);
    cache.borrow_mut().insert(key.to_vec(), rc.clone());
    rc
}

/// A made table's rows by the value of one column, or by a hash of several (a hash hit is tested again by the join).
/// 7: rows by key, compressed: one row array, each key's rows one range of it (ascending), found through an offset array
/// when the keys are small and dense (node ids, offsets), else through a flat hash of (start, length).
pub(super) struct Index {
    dense: bool,
    offs: Vec<u32>,
    at: FxHashMap<u64, (u32, u32)>,
    rows: Vec<u32>,
}

thread_local! {
    /// redo-alloc: an index's scratch (its keys, the fill cursor, the sorted pairs) and the kept buffers of dropped indexes.
    static IX_SCRATCH: RefCell<(Vec<u64>, Vec<u32>, Vec<(u64, u32)>)> = const { RefCell::new((Vec::new(), Vec::new(), Vec::new())) };
    static IX_KEPT: RefCell<(Vec<Vec<u32>>, Vec<FxHashMap<u64, (u32, u32)>>, usize)> = const { RefCell::new((Vec::new(), Vec::new(), 0)) };
}

/// redo-alloc: the kept index buffers' room, in all (cells): a worker keeps no more than this (4 MB).
const IX_CELLS: usize = 1 << 20;

/// redo-alloc: a kept row buffer (empty) or a new one.
fn ix_u32() -> Vec<u32> {
    IX_KEPT.with(|k| {
        let k = &mut *k.borrow_mut();
        let v = k.0.pop();
        if let Some(v) = &v {
            k.2 -= v.capacity();
        }
        v
    })
    .unwrap_or_default()
}

impl Drop for Index {
    fn drop(&mut self) {
        // redo-alloc: the buffers kept for the next index (a worker's working set; one far past it is freed)
        let _ = IX_KEPT.try_with(|k| {
            let k = &mut *k.borrow_mut();
            for mut v in [std::mem::take(&mut self.offs), std::mem::take(&mut self.rows)] {
                if v.capacity() > 0 && k.2 + v.capacity() <= IX_CELLS {
                    v.clear();
                    k.2 += v.capacity();
                    k.0.push(v);
                }
            }
            let mut m = std::mem::take(&mut self.at);
            if m.capacity() > 0 && m.capacity() <= 1 << 12 && k.1.len() < 64 {
                m.clear();
                k.1.push(m);
            }
        });
    }
}

impl Index {
    pub(super) fn build(n: usize, key: impl Fn(usize) -> u64) -> Index {
        // redo-alloc: the scratch and the index's own buffers from the worker's kept ones (same rows, same order)
        let (mut keys, mut fill, mut order) = IX_SCRATCH.with(|b| std::mem::take(&mut *b.borrow_mut()));
        keys.clear();
        keys.extend((0..n).map(key));
        let max = keys.iter().copied().max().unwrap_or(0);
        let ix = if n > 0 && max < (4 * n as u64).max(1024) {
            let m = max as usize + 1;
            let mut offs = ix_u32();
            offs.resize(m + 1, 0);
            for k in &keys {
                offs[*k as usize + 1] += 1;
            }
            for i in 0..m {
                offs[i + 1] += offs[i];
            }
            fill.clear();
            fill.extend_from_slice(&offs[..m]);
            let mut rows = ix_u32();
            rows.resize(n, 0);
            for (r, k) in keys.iter().enumerate() {
                let p = &mut fill[*k as usize];
                rows[*p as usize] = r as u32;
                *p += 1;
            }
            Index { dense: true, offs, at: FxHashMap::default(), rows }
        } else {
            order.clear();
            order.extend(keys.iter().enumerate().map(|(r, k)| (*k, r as u32)));
            order.sort_unstable();
            let mut at: FxHashMap<u64, (u32, u32)> = IX_KEPT.with(|k| k.borrow_mut().1.pop()).unwrap_or_default();
            at.reserve(n.min(1 << 16));
            let mut i = 0;
            while i < order.len() {
                let mut j = i;
                while j < order.len() && order[j].0 == order[i].0 {
                    j += 1;
                }
                at.insert(order[i].0, (i as u32, (j - i) as u32));
                i = j;
            }
            let mut rows = ix_u32();
            rows.extend(order.iter().map(|x| x.1));
            Index { dense: false, offs: Vec::new(), at, rows }
        };
        if keys.capacity() <= 1 << 16 && order.capacity() <= 1 << 16 {
            IX_SCRATCH.with(|b| *b.borrow_mut() = (keys, fill, order));
        }
        ix
    }
    pub(super) fn get(&self, k: &u64) -> Option<&[u32]> {
        if self.dense {
            let k = *k;
            if k.saturating_add(1) >= self.offs.len() as u64 {
                return None;
            }
            let (a, b) = (self.offs[k as usize] as usize, self.offs[k as usize + 1] as usize);
            return if a == b { None } else { Some(&self.rows[a..b]) };
        }
        self.at.get(k).map(|(a, l)| &self.rows[*a as usize..(*a + *l) as usize])
    }
}

fn key_of(vals: impl Iterator<Item = u64>) -> u64 {
    let mut h: u64 = 0;
    let mut n = 0;
    let mut first = 0;
    for v in vals {
        if n == 0 {
            first = v;
        }
        h = (h.rotate_left(5) ^ v).wrapping_mul(0x517c_c1b7_2722_0a95);
        n += 1;
    }
    if n == 1 { first } else { h }
}

fn index_of(t: &Table, cols: &[usize]) -> Index {
    if cols.len() == 1 {
        let c = &t.cols[cols[0]];
        return Index::build(t.n, |r| c[r]);
    }
    Index::build(t.n, |r| key_of(cols.iter().map(|c| t.cols[*c][r])))
}

fn table_index(f: &mut File<'_>, tn: usize, t: &Table, cols: &[usize]) -> Rc<Index> {
    // redo-alloc: the index's columns as an inline key, the index in a kept allocation (`index_rc`)
    let same = |c: &inline_key::Key| c.cells().len() == cols.len() && c.cells().iter().zip(cols).all(|(a, b)| *a == *b as u64);
    if let Some(v) = f.index.get(&tn)
        && let Some((_, ix)) = v.iter().find(|(c, _)| same(c))
    {
        return ix.clone();
    }
    let rc = index_rc(index_of(t, cols));
    let mut kb = [0u64; 8];
    let k = if cols.len() <= kb.len() {
        for (i, c) in cols.iter().enumerate() {
            kb[i] = *c as u64;
        }
        inline_key::Key::of(&kb[..cols.len()])
    } else {
        inline_key::Key::of(&cols.iter().map(|c| *c as u64).collect::<Vec<u64>>())
    };
    f.index.entry(tn).or_default().push((k, rc.clone()));
    rc
}

thread_local! {
    /// redo-alloc: emptied indexes' own allocations, for the next index made in a file.
    static IX_RCS: RefCell<Vec<Rc<Index>>> = const { RefCell::new(Vec::new()) };
}

/// redo-alloc: `ix` shared, in a kept allocation when there is one.
fn index_rc(ix: Index) -> Rc<Index> {
    match IX_RCS.with(|r| r.borrow_mut().pop()) {
        Some(mut r) => match Rc::get_mut(&mut r) {
            Some(slot) => {
                *slot = ix;
                r
            }
            None => Rc::new(ix),
        },
        None => Rc::new(ix),
    }
}

/// redo-alloc: an index given back: emptied (its buffers kept, `Index`'s drop) and its allocation kept when no one else
/// holds it.
fn recycle_index(mut r: Rc<Index>) {
    if let Some(slot) = Rc::get_mut(&mut r) {
        *slot = Index { dense: true, offs: Vec::new(), at: FxHashMap::default(), rows: Vec::new() };
        let _ = IX_RCS.try_with(|p| {
            let mut p = p.borrow_mut();
            if p.len() < 4096 {
                p.push(r);
            }
        });
    }
}

fn flatten(e: E, out: &mut Vec<E>) {
    match e {
        E::And(a, b) => {
            flatten(*a, out);
            flatten(*b, out);
        }
        e => out.push(e),
    }
}

fn refs_of(e: &E) -> u64 {
    match e {
        E::Col(s, _, _) => 1 << s,
        E::List(v) | E::Call(_, v) => v.iter().map(refs_of).fold(0, |a, b| a | b),
        E::Cmp(_, a, b) | E::In(a, b, _) | E::And(a, b) | E::Or(a, b) => refs_of(a) | refs_of(b),
        E::Not(a) | E::TextTest(a, _) => refs_of(a),
        _ => 0,
    }
}

fn static_ty(e: &E) -> Ty {
    match e {
        E::Col(_, _, t) => *t,
        E::Num(_) => Ty::Num,
        // redo-honest: `integer_successor(a, b)` is a test (the shipping engine's `Test::IntegerSuccessor`)
        E::Call(n, _) if n == "integer_successor" => Ty::Flag,
        E::Text(_) | E::Call(..) => Ty::Sym,
        _ => Ty::Flag,
    }
}

fn get(run: &mut Run, f: &File<'_>, srcs: &[Src], cur: &[u32], si: usize, col: usize) -> u64 {
    match &srcs[si] {
        Src::Tab(t, _) => t.cols[col][cur[si] as usize],
        Src::View(v, _) => {
            fill_prof::read(6);
            view_cols::cell(f, run, v.base, cur[si], v.map[col])
        }
        Src::Base(n) => {
            fill_prof::read(7);
            view_cols::cell(f, run, *n, cur[si], col)
        }
        Src::Lazy(_) => NULL,
    }
}

/// The rows of a made table whose column holds one of `keys`, through its index on that column (made once a file).
fn subset(f: &mut File<'_>, node: usize, t: &Table, col: usize, keys: &[u64]) -> Table {
    let ix = table_index(f, node, t, &[col]);
    // 10: the rows found first, then each column gathered (the same rows in the same order as row by row)
    thread_local! {
        static ROWS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    }
    ROWS.with(|b| {
        let mut rs = b.borrow_mut();
        rs.clear();
        for k in keys {
            if let Some(rows) = ix.get(k) {
                rs.extend_from_slice(rows);
            }
        }
        // redo-alloc: the columns taken with the rows' room now that their count is known
        let mut out = Table::with_rows(t.names.clone(), t.tys.clone(), rs.len());
        for (c, src) in t.cols.iter().enumerate() {
            let mut v = std::mem::take(&mut out.cols[c]);
            v.extend(rs.iter().map(|r| src[*r as usize]));
            out.cols[c] = v;
        }
        out.n = rs.len();
        out
    })
}

/// redo-alloc: an interned text is read where it lies (no copy); a number's text is made.
fn text_of(run: &Run, v: V) -> std::borrow::Cow<'_, [u8]> {
    use std::borrow::Cow;
    match v.ty() {
        Ty::Sym => Cow::Borrowed(run.text(v.1 as u32)),
        Ty::File => Cow::Borrowed(run.paths[v.1 as usize].as_bytes()),
        Ty::Num => Cow::Owned((v.1 as i64).to_string().into_bytes()),
        Ty::Flag => Cow::Borrowed(if v.1 == 1 { b"true".as_slice() } else { b"false".as_slice() }),
        _ => Cow::Borrowed(&[]),
    }
}

fn flag(b: bool) -> V {
    V::new(Ty::Flag, u64::from(b))
}

/// The ordering of two values, with a text compared to a number or a flag read as one.
fn compare(run: &Run, a: V, b: V) -> Option<std::cmp::Ordering> {
    use Ty::*;
    if a.1 == NULL || b.1 == NULL {
        return None;
    }
    match (a.ty(), b.ty()) {
        (Num, Num) => Some((a.1 as i64).cmp(&(b.1 as i64))),
        (Sym, Sym) => {
            if a.1 == b.1 {
                Some(std::cmp::Ordering::Equal)
            } else {
                Some(run.text(a.1 as u32).cmp(run.text(b.1 as u32)))
            }
        }
        (Num, Sym) | (Sym, Num) | (Flag, Sym) | (Sym, Flag) | (File, Sym) | (Sym, File) | (File, File) => Some(text_of(run, a).cmp(&text_of(run, b))),
        _ => Some(a.1.cmp(&b.1)),
    }
}

fn member_eq(run: &Run, a: V, b: V) -> bool {
    if a.ty() == Ty::Sym && b.ty() == Ty::Sym {
        a.1 == b.1 && a.1 != NULL
    } else {
        compare(run, a, b) == Some(std::cmp::Ordering::Equal)
    }
}

fn eval(run: &mut Run, f: &File<'_>, srcs: &[Src], cur: &[u32], e: &E) -> V {
    match e {
        E::Col(si, c, t) => V::new(*t, get(run, f, srcs, cur, *si, *c)),
        E::Lit(v) => *v,
        E::Text(t) => V::new(Ty::Sym, u64::from(*t)),
        E::Num(n) => V::new(Ty::Num, *n as u64),
        E::List(_) => flag(false),
        E::And(a, b) => flag(truthy(eval(run, f, srcs, cur, a)) && truthy(eval(run, f, srcs, cur, b))),
        E::Or(a, b) => flag(truthy(eval(run, f, srcs, cur, a)) || truthy(eval(run, f, srcs, cur, b))),
        E::Not(a) => flag(!truthy(eval(run, f, srcs, cur, a))),
        E::Cmp(o @ (Op::Eq | Op::Ne), a, b) => {
            let want = *o == Op::Eq;
            let eq = match (&**a, &**b) {
                (E::Col(s1, c1, t1), E::Col(s2, c2, t2)) if t1 == t2 => {
                    let (x, y) = (get(run, f, srcs, cur, *s1, *c1), get(run, f, srcs, cur, *s2, *c2));
                    Some(x == y && x != NULL)
                }
                (E::Col(s1, c1, Ty::Sym), E::Text(t)) | (E::Text(t), E::Col(s1, c1, Ty::Sym)) => Some(get(run, f, srcs, cur, *s1, *c1) == u64::from(*t)),
                _ => None,
            };
            match eq {
                Some(e) => flag(e == want),
                None => {
                    let (x, y) = (eval(run, f, srcs, cur, a), eval(run, f, srcs, cur, b));
                    match compare(run, x, y) {
                        Some(ord) => flag((ord == std::cmp::Ordering::Equal) == want),
                        None => flag(false),
                    }
                }
            }
        }
        E::Cmp(o, a, b) => {
            let (x, y) = (eval(run, f, srcs, cur, a), eval(run, f, srcs, cur, b));
            let Some(ord) = compare(run, x, y) else { return flag(false) };
            use std::cmp::Ordering::*;
            flag(match o {
                Op::Eq => ord == Equal,
                Op::Ne => ord != Equal,
                Op::Lt => ord == Less,
                Op::Le => ord != Greater,
                Op::Gt => ord == Greater,
                Op::Ge => ord != Less,
            })
        }
        E::In(a, b, neg) => {
            let x = eval(run, f, srcs, cur, a);
            let hit = match &**b {
                E::List(items) => items.iter().any(|i| {
                    let y = eval(run, f, srcs, cur, i);
                    member_eq(run, x, y)
                }),
                _ => false,
            };
            flag(hit != *neg)
        }
        E::TextTest(a, re) => {
            let x = eval(run, f, srcs, cur, a);
            flag(re.value(run, x))
        }
        E::Call(name, args) => {
            // redo-alloc: up to four arguments on the stack (the common sizes), more in a list
            if args.len() <= 4 {
                let mut vs = [NULL_V; 4];
                for (i, x) in args.iter().enumerate() {
                    vs[i] = eval(run, f, srcs, cur, x);
                }
                return call(run, name, &vs[..args.len()]);
            }
            let vs: Vec<V> = args.iter().map(|x| eval(run, f, srcs, cur, x)).collect();
            call(run, name, &vs)
        }
    }
}

fn call(run: &mut Run, name: &str, vs: &[V]) -> V {
    // redo-alloc: an argument's text read in place (`text_of`), copied only where a pattern is kept
    match name {
        "matches" => {
            let p = String::from_utf8(text_of(run, vs[1]).into_owned()).unwrap_or_default();
            let re = run.re(&p).expect("a regex");
            flag(TextTest::Regex(re).value(run, vs[0]))
        }
        "starts_with" => flag(text_of(run, vs[0]).starts_with(&text_of(run, vs[1]))),
        "ends_with" => flag(text_of(run, vs[0]).ends_with(&text_of(run, vs[1]))),
        "pascal" | "camel" | "snake" | "kebab" => {
            use ryo_operators::domain::functions::{service::apply, types::MadeText};
            use ryo_values::domain::plan_parts::types::Function;
            let case = match name {
                "pascal" => Function::Pascal,
                "camel" => Function::Camel,
                "snake" => Function::Snake,
                _ => Function::Kebab,
            };
            let input = MadeText(String::from_utf8_lossy(&text_of(run, vs[0])).into_owned());
            let mut output = MadeText::default();
            apply(case, &[input], &mut output);
            V::new(Ty::Sym, u64::from(run.sym(output.0.as_bytes())))
        }
        "lower" if vs[0].ty() == Ty::Sym => V::new(Ty::Sym, u64::from(run.lower_symbol(vs[0].1 as u32))),
        "lower" => {
            let s = String::from_utf8_lossy(&text_of(run, vs[0])).to_lowercase();
            V::new(Ty::Sym, u64::from(run.sym(s.as_bytes())))
        }
        "concat" => {
            // redo-alloc: the worker's one buffer for the text before it is interned
            thread_local! {
                static BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
            }
            let mut s = BUF.with(|b| std::mem::take(&mut *b.borrow_mut()));
            s.clear();
            for i in 0..vs.len() {
                s.extend_from_slice(&text_of(run, vs[i]));
            }
            let v = V::new(Ty::Sym, u64::from(run.sym(&s)));
            BUF.with(|b| *b.borrow_mut() = s);
            v
        }
        "glob_any" => {
            use ryo_matching::inbound::use_cases::matching::{
                interface::MatchingUseCase, types::{GlobPattern, MatchOutcome, SearchText}, use_case::MatchingUseCaseImpl,
            };
            if vs[1].ty() == Ty::Sym
                && let Ok(id) = u32::try_from(vs[1].1)
                && let Some(pattern) = run.prepared_globs.get(&id)
            {
                return flag(pattern.matches(run, vs[0]));
            }
            let text = text_of(run, vs[0]);
            let subject = String::from_utf8_lossy(&text);
            let g = text_of(run, vs[1]);
            let matched = MatchingUseCaseImpl::new().matches_text_glob(GlobPattern(&String::from_utf8_lossy(&g)), SearchText(&subject));
            flag(matched == MatchOutcome::Matches)
        }
        "natural_lt" => flag(natural_lt(&String::from_utf8_lossy(&text_of(run, vs[0])), &String::from_utf8_lossy(&text_of(run, vs[1])))),
        // redo-honest: the shipping engine's meaning (operators numeric_values::integer_successor): whether the right is
        // exactly the left plus one, both integers (an integer cell, or normalized decimal text); a float never is. It was
        // a function answering left + 1, read as a test, so every comparison failed.
        "integer_successor" => {
            let int = |run: &Run, v: V| -> Option<i128> {
                match v.ty() {
                    Ty::Num => Some(i128::from(v.1 as i64)),
                    _ => {
                        let s = String::from_utf8_lossy(&text_of(run, v)).replace('_', "");
                        if s.contains(['.', 'e', 'E']) { None } else { s.parse::<i128>().ok() }
                    }
                }
            };
            flag(matches!((int(run, vs[0]), int(run, vs[1])), (Some(a), Some(b)) if a.checked_add(1) == Some(b)))
        }
        "float_text_gte" => {
            let finite = |v: V| -> Option<f64> {
                if v.ty() != Ty::Sym || v.1 == NULL { return None; }
                let n = std::str::from_utf8(run.text(v.1 as u32)).ok()?.trim().parse::<f64>().ok()?;
                n.is_finite().then_some(n)
            };
            flag(matches!((finite(vs[0]), finite(vs[1])), (Some(a),Some(b)) if a >= b))
        }
        "numeric_gte" => {
            let num = |run: &Run, v: V| -> Option<f64> {
                match v.ty() {
                    Ty::Num => Some(v.1 as i64 as f64),
                    _ => String::from_utf8_lossy(&text_of(run, v)).replace('_', "").parse::<f64>().ok(),
                }
            };
            flag(matches!((num(run, vs[0]), num(run, vs[1])), (Some(a), Some(b)) if a >= b))
        }
        _ => {
            run.missing.insert(name.to_owned());
            flag(false)
        }
    }
}

#[path = "finish/words.rs"]
mod words;
#[path = "finish/populations.rs"]
mod populations;
use super::glob_pattern;

/// Natural order, as the language reference states it.
fn natural_lt(a: &str, b: &str) -> bool {
    // redo-alloc: the units as ranges of the text (a digit run, or one other character; whitespace splits) in the worker's
    // two buffers, not a string a unit
    fn units(s: &str, out: &mut Vec<(bool, usize, usize)>) {
        out.clear();
        let mut run: Option<usize> = None;
        for (at, ch) in s.char_indices() {
            if ch.is_ascii_digit() {
                run.get_or_insert(at);
                continue;
            }
            if let Some(st) = run.take() {
                out.push((true, st, at));
            }
            if !ch.is_whitespace() {
                out.push((false, at, at + ch.len_utf8()));
            }
        }
        if let Some(st) = run {
            out.push((true, st, s.len()));
        }
    }
    thread_local! {
        static UNITS: RefCell<(Vec<(bool, usize, usize)>, Vec<(bool, usize, usize)>)> = const { RefCell::new((Vec::new(), Vec::new())) };
    }
    let (mut xr, mut yr) = UNITS.with(|u| std::mem::take(&mut *u.borrow_mut()));
    units(a, &mut xr);
    units(b, &mut yr);
    let lt = natural_lt_of(xr.iter().map(|(d, s, e)| (*d, &a[*s..*e])), yr.iter().map(|(d, s, e)| (*d, &b[*s..*e])), xr.len() < yr.len());
    UNITS.with(|u| *u.borrow_mut() = (xr, yr));
    lt
}

/// redo-alloc: `natural_lt`'s comparison of two unit lists (`shorter`: the first has fewer units).
fn natural_lt_of<'a>(x: impl Iterator<Item = (bool, &'a str)>, y: impl Iterator<Item = (bool, &'a str)>, shorter: bool) -> bool {
    for (p, q) in x.zip(y) {
        let ord = match (p.0, q.0) {
            (true, true) => {
                if p.1.starts_with('0') || q.1.starts_with('0') {
                    p.1.cmp(&q.1)
                } else {
                    p.1.len().cmp(&q.1.len()).then_with(|| p.1.cmp(&q.1))
                }
            }
            _ => p.1.cmp(&q.1),
        };
        if ord != std::cmp::Ordering::Equal {
            return ord == std::cmp::Ordering::Less;
        }
    }
    shorter
}

/// `token-link` from the token order (step 5's pass): next token, matching and enclosing delimiter.
fn token_links(f: &File<'_>, run: &mut Run) -> Table {
    let (nt, md, ed) = (run.sym(b"next-token"), run.sym(b"matching-delimiter"), run.sym(b"enclosing-delimiter"));
    let mut out = Table::with(
        vec!["source".into(), "target".into(), "link-kind".into(), "file".into(), "span".into()],
        vec![Ty::Node, Ty::Node, Ty::Sym, Ty::File, Ty::Span],
    );
    let mut rows: Vec<[u64; 5]> = Vec::new();
    let sp = |p: u32| (u64::from(f.start[p as usize]) << 32) | u64::from(f.end[p as usize]);
    let fl = u64::from(f.file);
    if let Some(order) = &run.tokens {
        order.links(f.table, &mut |e| match e {
            TokenEvent::Token(_) => {}
            TokenEvent::Next(p, at) => rows.push([u64::from(p), u64::from(at), u64::from(nt), fl, sp(p)]),
            TokenEvent::Matching(o, c) => rows.push([u64::from(o), u64::from(c), u64::from(md), fl, sp(o)]),
            TokenEvent::Enclosing(at, t) => rows.push([u64::from(at), u64::from(t), u64::from(ed), fl, sp(at)]),
        });
    }
    for r in rows {
        out.push_row(&r);
    }
    out
}

/// The provider's rows of a relation as a table: texts interned run-wide, nodes as positions.
pub(super) fn filled_table(run: &mut Run, rel: &str, filled: &languages::FilledColumns, words: &languages::FileWords) -> R<Table> {
    let (_, names, tys) = run.catalog(rel).ok_or_else(|| format!("no relation {rel}"))?;
    let n = filled.0.first().map_or(0, |c| c.0.len());
    let mut out = Table::with_rows(names, tys.clone(), n);
    for (c, ty) in tys.iter().enumerate() {
        let Some(column) = filled.0.get(c) else {
            out.cols[c] = vec![NULL; n];
            continue;
        };
        // redo-alloc: into the pooled column; redo-fills c2: texts through the run's word caches (`word_sym`)
        let v = &mut out.cols[c];
        match ty {
            Ty::Sym => {
                let base = languages::facts::base_count(words).0;
                v.extend(column.0.iter().map(|w| u64::from(word_sym(run, words, *w, base))))
            }
            Ty::Node => v.extend(column.0.iter().map(|w| w.0 & 0xffff_ffff)),
            _ => v.extend(column.0.iter().map(|w| w.0)),
        }
    }
    out.n = n;
    Ok(out)
}

/// redo-fills c2: a provider word's run-wide symbol, through the run's word caches (`Run::base_word_sym`).
fn word_sym(run: &mut Run, words: &languages::FileWords, w: languages::CellWord, base: u64) -> u32 {
    let (own, at) = if w.0 < base {
        (false, w.0 as usize)
    } else if w.0 < 0xFFFF_0000 {
        (true, (w.0 - base) as usize)
    } else if w.0 < 0xFFFF_0700 {
        // redo-fills pass 2: a closed-set word's symbol, once a run (no text copied and hashed a cell)
        let at = (w.0 - 0xFFFF_0000) as usize;
        if let Some(&s) = run.closed_word_sym.get(at)
            && s != u32::MAX
        {
            return s;
        }
        let s = cell_sym(run, words, w);
        if run.closed_word_sym.len() <= at {
            run.closed_word_sym.resize(at + 1, u32::MAX);
        }
        run.closed_word_sym[at] = s;
        return s;
    } else {
        return cell_sym(run, words, w);
    };
    let cache = if own { &run.own_word_sym } else { &run.base_word_sym };
    if let Some(&s) = cache.get(at)
        && s != u32::MAX
    {
        return s;
    }
    let s = run.sym(languages::facts::text(words, w).0.as_bytes());
    let cache = if own { &mut run.own_word_sym } else { &mut run.base_word_sym };
    if at < 1 << 22 {
        if cache.len() <= at {
            cache.resize(at + 1, u32::MAX);
        }
        cache[at] = s;
    }
    s
}

/// redo-fills c2: a new file's words start: the own-word symbols of the last file are forgotten.
pub(super) fn own_words_cleared(run: &mut Run) {
    run.own_word_sym.clear();
}

/// redo-alloc: a cell's text interned (a closed set's word by `closed_text`, else the file's word read in place).
fn cell_sym(run: &mut Run, words: &languages::FileWords, w: languages::CellWord) -> u32 {
    if let Some(text) = closed_text(w) {
        return run.sym(&text);
    }
    run.sym(languages::facts::text(words, w).0.as_bytes())
}

/// redo-binding: a cell's text, a word of a closed set (`defines.kind` and the other closed columns: the catalog's term blocks
/// from 0xFFFF_0000, 0x100 words a block, `ryo_relations::domain::terms`) read by its number, which the file's words do not hold.
/// redo-alloc: its closed-set part only (the file's words are read in place by `cell_sym`).
fn closed_text(w: languages::CellWord) -> Option<Vec<u8>> {
    use ryo_relations::domain::terms::{service::words_of, types::WordBlock};
    if (0xFFFF_0000..0xFFFF_0700).contains(&w.0) {
        let at = w.0 - 0xFFFF_0000;
        let block = [WordBlock::Declaration, WordBlock::Parameter, WordBlock::Value, WordBlock::Mapping, WordBlock::Dependency, WordBlock::Unknown, WordBlock::Origin][(at / 0x100) as usize];
        if let Some(t) = words_of(block).get((at % 0x100) as usize) {
            return Some(t.0.as_bytes().to_vec());
        }
    }
    None
}

/// A tree relation as this file reads it in place, made whole (for `--check-bases`).
pub(super) fn native_table(f: &mut File<'_>, run: &mut Run, rel: &str) -> Table {
    if rel == "token-link" {
        return token_links(f, run);
    }
    let nat = native_of(run, rel).expect("a native relation");
    let cols = native_cols(nat);
    let mut out = Table::with(cols.iter().map(|c| c.0.to_owned()).collect::<Vec<_>>(), cols.iter().map(|c| c.1).collect::<Vec<_>>());
    for r in f.native_rows(run, nat) {
        for c in 0..cols.len() {
            let v = f.native_get(run, nat, r, c);
            out.cols[c].push(v);
        }
        out.n += 1;
    }
    out
}

/// Two tables' rows on their common columns, as multisets: counts and the first differences.
pub(super) fn diff(run: &Run, a: &Table, b: &Table) -> String {
    let common: Vec<(usize, usize)> = a.names.iter().enumerate().filter_map(|(i, n)| b.names.iter().position(|m| m == n).map(|j| (i, j))).collect();
    let only_a: Vec<&String> = a.names.iter().filter(|n| !b.names.contains(n)).collect();
    let only_b: Vec<&String> = b.names.iter().filter(|n| !a.names.contains(n)).collect();
    let rows = |t: &Table, side: usize| {
        let mut m: std::collections::BTreeMap<Vec<u64>, i64> = std::collections::BTreeMap::new();
        for r in 0..t.n {
            let k: Vec<u64> = common.iter().map(|(i, j)| t.cols[if side == 0 { *i } else { *j }][r]).collect();
            *m.entry(k).or_default() += 1;
        }
        m
    };
    let (ra, rb) = (rows(a, 0), rows(b, 1));
    let mut extra = Vec::new();
    let mut missing = Vec::new();
    for (k, c) in &ra {
        let d = c - rb.get(k).copied().unwrap_or(0);
        if d > 0 {
            extra.push(k.clone());
        }
    }
    for (k, c) in &rb {
        let d = c - ra.get(k).copied().unwrap_or(0);
        if d > 0 {
            missing.push(k.clone());
        }
    }
    let show = |k: &Vec<u64>| {
        k.iter()
            .zip(&common)
            .map(|(v, (i, _))| match a.tys[*i] {
                Ty::Sym => format!("{}={:?}", a.names[*i], String::from_utf8_lossy(run.text(*v as u32))),
                _ => format!("{}={v}", a.names[*i]),
            })
            .collect::<Vec<_>>()
            .join(",")
    };
    format!(
        "mine {} theirs {} extra {} missing {} only-mine-cols {:?} only-theirs-cols {:?}{}{}",
        a.n,
        b.n,
        extra.len(),
        missing.len(),
        only_a,
        only_b,
        extra.first().map(|k| format!(" first-extra [{}]", show(k))).unwrap_or_default(),
        missing.first().map(|k| format!(" first-missing [{}]", show(k))).unwrap_or_default()
    )
}

/// Whether every line of `text`, after `prefix` on its first line, stays within `max` display columns (the operator's rule).
fn fits(text: &str, prefix: &str, max: usize, tab: usize) -> bool {
    use unicode_width::UnicodeWidthChar;
    let adv = |col: usize, c: char| if c == '\t' { (col / tab + 1) * tab } else { col + UnicodeWidthChar::width(c).unwrap_or(0) };
    let mut col = prefix.chars().fold(0, adv);
    if col > max {
        return false;
    }
    let mut cr = false;
    for c in text.chars() {
        if c == '\r' {
            col = 0;
            cr = true;
            continue;
        }
        if c == '\n' {
            if !cr {
                col = 0;
            }
            cr = false;
            continue;
        }
        cr = false;
        col = adv(col, c);
        if col > max {
            return false;
        }
    }
    true
}
