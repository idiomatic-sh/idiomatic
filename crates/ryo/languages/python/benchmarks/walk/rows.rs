//! Bake-off round 2 (bead ryo-emlr, throwaway): the provider's semantic relations as rows computed from the model, and a
//! canonical text dump (node cells `kind@start:end`, spans `start:end`) compared with the export by `sema/compare.py`.
//! `count` produces every row's cells into reusable buffers without formatting (what a consumer pays); `dump` formats them.

use crate::model::{DefKind, Model, NONE, UNKNOWN};
use crate::table::Table;
use crate::view::node_kind_name;
use ruff_python_ast::AnyNodeRef;

pub(crate) const RELATIONS: [&str; 11] = [
    "scope",
    "defines",
    "reference",
    "resolves",
    "qualified-name",
    "implicit-symbol",
    "binding-event",
    "binding-read",
    "binding-use",
    "binding-shadow",
    "binding-resolution",
];

/// The output of one file: per relation, either formatted lines (dump) or only counts.
pub(crate) struct Out<'o> {
    pub lines: Option<&'o mut [Vec<String>; 11]>,
    pub counts: [u64; 11],
    pub path: String,
}

fn esc(text: Option<&str>) -> String {
    match text {
        None => "\\N".to_owned(),
        Some(t) => {
            let mut s = String::with_capacity(t.len());
            for c in t.chars() {
                match c {
                    '\\' => s.push_str("\\\\"),
                    '\t' => s.push_str("\\t"),
                    '\n' => s.push_str("\\n"),
                    '\r' => s.push_str("\\r"),
                    c => s.push(c),
                }
            }
            s
        }
    }
}

struct Ctx<'a, 't> {
    table: &'a Table<'t>,
    file: usize,
}

impl Ctx<'_, '_> {
    fn node(&self, id: u32) -> String {
        let i = id as usize;
        format!("{}@{}:{}", node_kind_name(self.table.node[i]), self.table.start[i], self.table.stop[i])
    }
    fn span(&self, id: u32) -> String {
        let i = id as usize;
        format!("{}:{}", self.table.start[i], self.table.stop[i])
    }
}

/// `qualification::qualify`'s path: the import's or builtin's dotted name, then the attributes read from it.
fn qualify(model: &mut Model, table: &Table<'_>, at: u32, path: &mut String) -> (&'static str, (i64, i64), bool) {
    let mut base = at as usize;
    loop {
        match table.node[base] {
            AnyNodeRef::ExprName(_) => break,
            AnyNodeRef::ExprAttribute(a) => base = Table::id_of(AnyNodeRef::from(&*a.value)) as usize,
            _ => return ("unresolved", (0, 0), false),
        }
    }
    let name = model.sym[base];
    let r = model.resolve_memo(table, base as u32);
    let binding = (r != NONE).then_some(r);
    if model.uncertain(base as u32, name, binding) {
        return ("ambiguous", (0, 0), false);
    }
    path.clear();
    let (label, since) = match binding {
        Some(b) => {
            let AnyNodeRef::Alias(alias) = table.node[b as usize] else { return ("local", (0, 0), false) };
            if let AnyNodeRef::StmtImportFrom(from) = table.node[table.parent[b as usize] as usize] {
                for _ in 0..from.level {
                    path.push('.');
                }
                if let Some(m) = &from.module {
                    path.push_str(m.as_str());
                    path.push('.');
                }
                path.push_str(alias.name.as_str());
            } else if alias.asname.is_some() {
                path.push_str(alias.name.as_str());
            } else {
                path.push_str(alias.name.as_str().split('.').next().unwrap_or(""));
            }
            ("import", (0, 0))
        }
        None => match model.builtin(name) {
            Some(v) => {
                path.push_str("builtins.");
                path.push_str(model.text(name).unwrap_or(""));
                ("builtin", v)
            }
            None => return ("unresolved", (0, 0), false),
        },
    };
    append_attributes(table, at as usize, path);
    (label, since, true)
}

fn append_attributes(table: &Table<'_>, at: usize, path: &mut String) {
    if let AnyNodeRef::ExprAttribute(a) = table.node[at] {
        append_attributes(table, Table::id_of(AnyNodeRef::from(&*a.value)) as usize, path);
        path.push('.');
        path.push_str(a.attr.as_str());
    }
}

/// `names::dottedness` + `dotted`
fn dotted(table: &Table<'_>, at: usize, out: &mut String) -> bool {
    match table.node[at] {
        AnyNodeRef::ExprName(n) => {
            out.push_str(n.id.as_str());
            true
        }
        AnyNodeRef::ExprAttribute(a) => {
            if !dotted(table, Table::id_of(AnyNodeRef::from(&*a.value)) as usize, out) {
                return false;
            }
            out.push('.');
            out.push_str(a.attr.as_str());
            true
        }
        _ => false,
    }
}

fn load(table: &Table<'_>, at: usize) -> bool {
    match table.node[at] {
        AnyNodeRef::ExprName(n) => n.ctx.is_load(),
        AnyNodeRef::ExprAttribute(a) => a.ctx.is_load(),
        _ => false,
    }
}

/// `names::reading`: the object of an attribute that is read is part of the longer name, not a reference of its own.
fn object_of_read(table: &Table<'_>, at: usize) -> bool {
    let p = table.parent[at] as usize;
    matches!(table.node[p], AnyNodeRef::ExprAttribute(a) if Table::id_of(AnyNodeRef::from(&*a.value)) as usize == at && a.ctx.is_load())
}

pub(crate) fn produce(model: &mut Model, table: &Table<'_>, file: usize, wanted: u16, out: &mut Out<'_>, scratch: &mut String) {
    let c = Ctx { table, file };
    let f = file;
    let want = |i: usize| wanted & (1 << i) != 0;
    let mut lines = out.lines.take();
    macro_rules! emit {
        ($i:expr, $($fmt:tt)*) => {{
            out.counts[$i] += 1;
            if let Some(l) = lines.as_deref_mut() { l[$i].push(format!($($fmt)*)); }
        }};
    }
    let n = table.len();
    // scope
    if want(0) {
        for id in 0..n as u32 {
            if matches!(table.node[id as usize], AnyNodeRef::ModModule(_) | AnyNodeRef::StmtFunctionDef(_) | AnyNodeRef::StmtClassDef(_) | AnyNodeRef::ExprLambda(_) | AnyNodeRef::ExprListComp(_) | AnyNodeRef::ExprSetComp(_) | AnyNodeRef::ExprDictComp(_) | AnyNodeRef::ExprGenerator(_)) {
                let parent = if id == 0 { 0 } else { model.scope_of[id as usize] };
                emit!(0, "{f}\t{}\t{}\t{f}\t{}", c.node(parent), c.node(id), c.span(id));
            }
        }
    }
    // defines
    if want(1) {
        for d in &model.defs {
            let kind = match d.kind {
                DefKind::Function => "function",
                DefKind::Method => "method",
                DefKind::Class => "class",
                DefKind::Import => "import",
                DefKind::Variable => "variable",
                DefKind::TypeAlias => "type-alias",
                DefKind::Parameter => "parameter",
                DefKind::TypeParameter => "type-parameter",
            };
            emit!(1, "{f}\t{}\t{kind}\t{}\t{}\t{}\t{f}\t{}:{}", c.node(d.at), d.word, esc(model.text(d.name)), c.node(d.owner), d.named_start, d.named_stop);
        }
    }
    // reference, resolves, qualified-name: one pass over names and attributes
    if want(2) || want(3) || want(4) {
        for &kind in &[ruff_python_ast::NodeKind::ExprName as u8, ruff_python_ast::NodeKind::ExprAttribute as u8] {
            for &id in table.of_kind(kind) {
                let i = id as usize;
                let is_name = matches!(table.node[i], AnyNodeRef::ExprName(_));
                let ctx_store = if is_name { model.store[i] == 1 } else { !load(table, i) };
                if want(2) {
                    let used = if is_name { !ctx_store } else { load(table, i) } && !object_of_read(table, i);
                    if used {
                        scratch.clear();
                        if dotted(table, i, scratch) {
                            emit!(2, "{f}\t{}\t{}\t{}\t{f}\t{}", c.node(id), c.node(model.scope_of[i]), esc(Some(scratch)), c.span(id));
                        }
                    }
                }
                if !load(table, i) {
                    continue;
                }
                if want(3) && is_name {
                    let r = model.resolve_memo(table, id);
                    if r != NONE {
                        emit!(3, "{f}\t{}\t{}\t{}\t{}\t{f}\t{}", c.node(id), c.node(model.scope_of[i]), esc(model.text(model.sym[i])), c.node(r), c.span(id));
                    }
                }
                if want(4) && !model.incomplete {
                    let (label, since, has_path) = qualify(model, table, id, scratch);
                    let path = if has_path { esc(Some(scratch)) } else { "\\N".to_owned() };
                    emit!(4, "{f}\t{}\t{path}\t{label}\t{}\t{}\t{f}\t{}", c.node(id), since.0, since.1, c.span(id));
                }
            }
        }
    }
    // implicit-symbol: the module's catalog
    if want(5) {
        for (name, kind, major, minor) in crate::builtins::IMPLICIT {
            emit!(5, "{f}\t{name}\t{kind}\t{}\t{major}\t{minor}\t{f}", c.node(0));
        }
    }
    if !model.flow_incomplete {
        if want(6) {
            for e in &model.events {
                emit!(6, "{f}\t{}\t{}\t{}\t{}\t{}\t{}\t{f}\t{}\t{}", c.node(e.at), esc(model.text(e.name)), c.node(e.scope), c.node(e.phase), e.kind.word(), e.ordinal, c.span(e.at), c.node(e.lexical));
            }
        }
        if want(7) {
            for r in &model.reads {
                let (a, b) = r.state.since();
                emit!(7, "{f}\t{}\t{}\t{}\t{}\t{}\t{}\t{a}\t{b}\t{f}\t{}", c.node(r.at), esc(model.text(r.name)), c.node(r.scope), c.node(r.phase), r.context.word(), r.state.word(), c.span(r.at));
            }
        }
        if want(8) {
            for u in &model.uses {
                emit!(8, "{f}\t{}\t{}\t{}\t{}\t{}\t{}\t{f}\t{}", c.node(u.at), c.node(u.binding), c.node(u.scope), c.node(u.phase), u.context.word(), if u.unique { "unique" } else { "alternative" }, c.span(u.at));
            }
        }
        if want(9) {
            for s in &model.shadows {
                emit!(9, "{f}\t{}\t{}\t{}\t{}\t{}\t{}\t{f}", c.node(s.binding), c.node(s.previous), c.node(s.scope), c.node(s.previous_scope), c.node(s.phase), if s.lexical { "lexical" } else { "replacement" });
            }
        }
        if want(10) {
            for r in &model.resolutions {
                emit!(10, "{f}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{f}\t{}", c.node(r.at), c.node(r.event), r.kind.word(), c.node(r.scope), c.node(r.lexical), r.depth, r.ordinal, r.context.word(), c.span(r.at));
            }
        }
    }
    let _ = UNKNOWN;
    out.lines = lines;
}
