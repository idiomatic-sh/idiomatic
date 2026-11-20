//! The model's 11 relations as typed cells for the nav executor (bead ryo-lxty, throwaway): the same rows as `rows::produce`
//! (equal to the provider's on all of Django, per ryo-emlr), written in the catalog's column order (node = the view's id, which
//! is the table's id for AST nodes; symbol = the run's text code; file = 0; span = start << 32 | end), one relation at a time.

use crate::model::{DefKind, Model, NONE};
use crate::symbols::{NULL, Symbols};
use crate::table::Table;
use ruff_python_ast::AnyNodeRef;

pub(crate) const RELATIONS: [&str; 11] = crate::rows::RELATIONS;

fn sp(table: &Table<'_>, id: u32) -> u64 {
    let i = id as usize;
    (u64::from(table.start[i]) << 32) | u64::from(table.stop[i])
}

fn t(sym: &mut Symbols, text: Option<&str>) -> u64 {
    match text {
        Some(x) => u64::from(sym.text(x)),
        None => u64::from(NULL),
    }
}

/// The columns of relation `rel` with a per-key function (facts): a scope by its node (column 1); a reference, a resolution and a
/// qualified name by the name or attribute expression (column 0). The flow relations are arrays of the model, filled whole.
pub(crate) fn key_columns(rel: usize) -> u16 {
    match rel {
        0 => 1 << 1,
        2..=4 => 1,
        _ => 0,
    }
}

/// Fills `cols` (one vector per column) with the rows of relation `rel` (index into RELATIONS); with `only = (column, node)`,
/// just the rows whose key column holds that node (the per-key function: nothing else of the relation is computed).
pub(crate) fn produce(model: &mut Model, table: &Table<'_>, rel: usize, sym: &mut Symbols, cols: &mut [Vec<u64>], scratch: &mut String, only: Option<(u8, u32)>) {
    let mut push = |cols: &mut [Vec<u64>], cells: &[u64]| {
        for (c, &v) in cols.iter_mut().zip(cells) {
            c.push(v);
        }
    };
    let n = table.len();
    let one = only.map(|(_, id)| id).filter(|id| (*id as usize) < n);
    if only.is_some() && one.is_none() {
        return;
    }
    let all = |k: usize| -> &[u32] {
        match &one {
            Some(id) => {
                if table.kind[*id as usize] == k as u8 {
                    std::slice::from_ref(id)
                } else {
                    &[]
                }
            }
            None => table.of_kind(k as u8),
        }
    };
    match rel {
        0 => {
            for id in one.map_or(0..n as u32, |id| id..id + 1) {
                if matches!(
                    table.node[id as usize],
                    AnyNodeRef::ModModule(_)
                        | AnyNodeRef::StmtFunctionDef(_)
                        | AnyNodeRef::StmtClassDef(_)
                        | AnyNodeRef::ExprLambda(_)
                        | AnyNodeRef::ExprListComp(_)
                        | AnyNodeRef::ExprSetComp(_)
                        | AnyNodeRef::ExprDictComp(_)
                        | AnyNodeRef::ExprGenerator(_)
                ) {
                    let parent = if id == 0 { 0 } else { model.scope_of[id as usize] };
                    push(cols, &[u64::from(parent), u64::from(id), 0, sp(table, id)]);
                }
            }
        }
        1 => {
            for k in 0..model.defs.len() {
                let d = model.defs[k];
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
                let cells = [
                    u64::from(d.at),
                    t(sym, Some(kind)),
                    t(sym, Some(d.word)),
                    t(sym, model.text(d.name)),
                    u64::from(d.owner),
                    0,
                    (u64::from(d.named_start) << 32) | u64::from(d.named_stop),
                ];
                push(cols, &cells);
            }
        }
        2..=4 => {
            for &kind in &[ruff_python_ast::NodeKind::ExprName as u8, ruff_python_ast::NodeKind::ExprAttribute as u8] {
                for &id in all(kind as usize) {
                    let i = id as usize;
                    let is_name = matches!(table.node[i], AnyNodeRef::ExprName(_));
                    let ctx_store = if is_name { model.store[i] == 1 } else { !load(table, i) };
                    if rel == 2 {
                        let used = if is_name { !ctx_store } else { load(table, i) } && !object_of_read(table, i);
                        if used {
                            scratch.clear();
                            if dotted(table, i, scratch) {
                                let cells = [u64::from(id), u64::from(model.scope_of[i]), t(sym, Some(scratch)), 0, sp(table, id)];
                                push(cols, &cells);
                            }
                        }
                        continue;
                    }
                    if !load(table, i) {
                        continue;
                    }
                    if rel == 3 && is_name {
                        let r = model.resolve_memo(table, id);
                        if r != NONE {
                            let name = model.sym[i];
                            let cells = [u64::from(id), u64::from(model.scope_of[i]), t(sym, model.text(name)), u64::from(r), 0, sp(table, id)];
                            push(cols, &cells);
                        }
                    }
                    if rel == 4 && !model.incomplete {
                        let (label, since, has_path) = crate::rows::qualify_pub(model, table, id, scratch);
                        let path = if has_path { t(sym, Some(scratch)) } else { u64::from(NULL) };
                        let cells = [u64::from(id), path, t(sym, Some(label)), since.0 as u64, since.1 as u64, 0, sp(table, id)];
                        push(cols, &cells);
                    }
                }
            }
        }
        5 => {
            for (name, kind, major, minor) in crate::builtins::IMPLICIT {
                let cells = [t(sym, Some(*name)), t(sym, Some(*kind)), 0, *major as u64, *minor as u64, 0];
                push(cols, &cells);
            }
        }
        _ if model.flow_incomplete => {}
        6 => {
            for k in 0..model.events.len() {
                let e = model.events[k];
                let cells = [
                    u64::from(e.at),
                    t(sym, model.text(e.name)),
                    u64::from(e.scope),
                    u64::from(e.phase),
                    t(sym, Some(e.kind.word())),
                    e.ordinal as u64,
                    0,
                    sp(table, e.at),
                    u64::from(e.lexical),
                ];
                push(cols, &cells);
            }
        }
        7 => {
            for k in 0..model.reads.len() {
                let r = model.reads[k].clone();
                let (a, b) = r.state.since();
                let cells = [
                    u64::from(r.at),
                    t(sym, model.text(r.name)),
                    u64::from(r.scope),
                    u64::from(r.phase),
                    t(sym, Some(r.context.word())),
                    t(sym, Some(r.state.word())),
                    a as u64,
                    b as u64,
                    0,
                    sp(table, r.at),
                ];
                push(cols, &cells);
            }
        }
        8 => {
            for k in 0..model.uses.len() {
                let u = model.uses[k];
                let cells = [
                    u64::from(u.at),
                    u64::from(u.binding),
                    u64::from(u.scope),
                    u64::from(u.phase),
                    t(sym, Some(u.context.word())),
                    t(sym, Some(if u.unique { "unique" } else { "alternative" })),
                    0,
                    sp(table, u.at),
                ];
                push(cols, &cells);
            }
        }
        9 => {
            for k in 0..model.shadows.len() {
                let s = model.shadows[k];
                let cells = [
                    u64::from(s.binding),
                    u64::from(s.previous),
                    u64::from(s.scope),
                    u64::from(s.previous_scope),
                    u64::from(s.phase),
                    t(sym, Some(if s.lexical { "lexical" } else { "replacement" })),
                    0,
                ];
                push(cols, &cells);
            }
        }
        _ => {
            for k in 0..model.resolutions.len() {
                let r = model.resolutions[k];
                let cells = [
                    u64::from(r.at),
                    u64::from(r.event),
                    t(sym, Some(r.kind.word())),
                    u64::from(r.scope),
                    u64::from(r.lexical),
                    r.depth as u64,
                    r.ordinal as u64,
                    t(sym, Some(r.context.word())),
                    0,
                    sp(table, r.at),
                ];
                push(cols, &cells);
            }
        }
    }
}

fn load(table: &Table<'_>, at: usize) -> bool {
    match table.node[at] {
        AnyNodeRef::ExprName(n) => n.ctx.is_load(),
        AnyNodeRef::ExprAttribute(a) => a.ctx.is_load(),
        _ => false,
    }
}

fn object_of_read(table: &Table<'_>, at: usize) -> bool {
    let p = table.parent[at] as usize;
    matches!(table.node[p], AnyNodeRef::ExprAttribute(a) if Table::id_of(AnyNodeRef::from(&*a.value)) as usize == at && a.ctx.is_load())
}

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
