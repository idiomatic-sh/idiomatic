//! redo-fills pass 2 (THROWAWAY, bead ryo-fmx3.1): a provider relation the prototype writes directly as columns
//! (`ColumnFile::column`: texts as the run's own symbols through `RunIds`, nodes as ids) made the engine's table with no
//! reader rows and no word conversion: the rows in the order the provider wrote them (the reader's order: proved by the
//! fill dump's ordered digest), the key column from the rows' keys, a span from the node table, the file column the
//! caller's. A relation the prototype does not write, or a file it does not answer whole, goes to the reader.
//! `VEC_NO_DIRECT` turns the path off (every relation through the reader).

use super::finish::{self, NULL, Table, Ty};
use ryo::languages::{NodeTable, Position, node_table};
use ryo_python::fixtures::columns::{Ask, ColumnFile, ColumnMemo, FactTable, Layout, TextIds};
use std::cell::RefCell;

thread_local! {
    /// The worker's answer buffer, kept from fill to fill.
    static FT: RefCell<FactTable> = RefCell::new(FactTable::default());
    /// The direct path's own text memo (run symbols) and the file it last wrote.
    static MEMO: RefCell<(ColumnMemo, u32)> = RefCell::new((ColumnMemo::default(), u32::MAX));
}

/// The run's symbols as the provider's text ids.
struct RunIds<'r>(&'r mut finish::Run);

impl TextIds for RunIds<'_> {
    fn id(&mut self, text: &[u8]) -> u32 {
        self.0.sym(text)
    }
}

/// Whether `rel` is answered by the direct path.
pub(super) fn direct(rel: &str) -> bool {
    static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    !*OFF.get_or_init(|| std::env::var_os("VEC_NO_DIRECT").is_some())
        && matches!(rel, "qualified-name" | "identifier" | "literal-properties" | "numeric-literal" | "string-literal" | "literal-expression-parse" | "expression-shape" | "expression-type" | "binding-event" | "binding-read")
}

/// The direct fill of `rel` (see the module); `None` where the reader must answer. The seconds of the provider's write and
/// of the table's making come back for the fills' clock.
pub(super) fn fill(file: &mut ColumnFile<'_, '_>, run: &mut finish::Run, rel: &str, file_no: u32, timed: bool, columns: u64) -> Option<(Result<Table, String>, f64, f64)> {
    let (relation, names, tys) = run.catalog(rel)?;
    let relation = relation?;
    let key = tys.iter().position(|t| *t == Ty::Node)?;
    let table: &NodeTable = file.table;
    FT.with(|ft| {
        let mut ft = ft.borrow_mut();
        let t0 = if timed { super::thread_cpu() } else { 0.0 };
        MEMO.with(|m| {
            let (memo, at) = &mut *m.borrow_mut();
            if *at != file_no {
                memo.new_file();
                *at = file_no;
            }
            file.column_with(&Ask { relation, key, exists: false, columns }, &mut RunIds(run), &mut ft, memo);
        });
        let t1 = if timed { super::thread_cpu() } else { 0.0 };
        if !ft.answered {
            return None;
        }
        let keys = ft.keys();
        let n = keys.len();
        // redo-alloc (merged by redo-int): the table through the pool's constructor, each column written in place
        let tys2 = tys.clone();
        let mut out = Table::projected(names, tys, n, columns);
        for (c, ty) in tys2.iter().enumerate() {
            if !finish::request::wants(columns, c) { continue; }
            let written = ft.columns.get(c).filter(|col| !col.is_empty());
            let col = &mut out.cols[c];
            match (written, ty) {
                (Some(w), _) if ft.layout == Layout::PerNode => col.extend(keys.iter().map(|k| widen(*ty, w[*k as usize]))),
                (Some(w), _) => col.extend(w.iter().map(|v| widen(*ty, *v))),
                (None, _) if c == key => col.extend(keys.iter().map(|k| u64::from(*k))),
                (None, Ty::File) => col.resize(n, u64::from(file_no)),
                (None, Ty::Span) => col.extend(keys.iter().map(|k| {
                    let s = node_table::span_of(table, Position(*k));
                    (u64::from(s.start.0) << 32) | u64::from(s.end.0)
                })),
                (None, _) if n == 0 => {}
                (None, _) => return None,
            }
        }
        out.n = n;
        let t2 = if timed { super::thread_cpu() } else { 0.0 };
        Some((Ok(out), t1 - t0, t2 - t1))
    })
}

/// A provider cell as the engine's: the same number (texts are symbols already, nodes ids); a flag 0 or 1.
#[inline]
fn widen(_ty: Ty, v: u32) -> u64 {
    if v == u32::MAX { NULL } else { u64::from(v) }
}

/// `VEC_DIRECT_CHECK`: every direct fill is checked against the reader's, cell by cell in row order.
pub(super) fn checking() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VEC_DIRECT_CHECK").is_some())
}

thread_local! {
    static SEEN: RefCell<std::collections::HashMap<String, u32>> = RefCell::new(std::collections::HashMap::new());
}

/// Prints the first differences of a direct fill from the reader's (at most 5 a relation and column).
pub(super) fn check(run: &finish::Run, rel: &str, file: u32, reader: &Table, direct: &Table, columns: u64) {
    let mut say = |what: String| {
        SEEN.with(|s| {
            let mut s = s.borrow_mut();
            let e = s.entry(format!("{rel}:{}", what.split(' ').next().unwrap_or(""))).or_default();
            *e += 1;
            if *e <= 5 {
                eprintln!("DIRECT-DIFF {rel} {} {what}", run.paths[file as usize]);
            }
        });
    };
    if reader.n != direct.n {
        say(format!("rows reader {} direct {}", reader.n, direct.n));
    }
    for c in 0..reader.cols.len() {
        if !finish::request::wants(columns, c) {
            continue;
        }
        for r in 0..reader.n.min(direct.n) {
            let (a, b) = (reader.cols[c][r], direct.cols[c][r]);
            if a != b {
                let txt = |v: u64| if reader.tys[c] == Ty::Sym && v != NULL { String::from_utf8_lossy(&run.texts[v as usize]).into_owned() } else { String::new() };
                say(format!("{} row {r} reader {a:#x} {} direct {b:#x} {}", reader.names[c], txt(a), txt(b)));
                break;
            }
        }
    }
}
