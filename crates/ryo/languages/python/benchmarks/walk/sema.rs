//! A Ruff-shaped semantic model built once per file from the flat table and the parser's nodes in place: scopes (module, class,
//! function, lambda, comprehension), bindings (assignment and deletion targets, parameters, defs, imports, handler and pattern
//! names, global/nonlocal declarations), interned names, and every load resolved to its binding through the scope chain (class
//! scopes skipped from nested functions; function bodies see the whole enclosing scope, as Ruff's deferred bodies do).

use crate::table::Table;
use ruff_python_ast::{AnyNodeRef, ExprContext, NodeKind};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

#[derive(Default)]
pub(crate) struct Fx(u64);
impl Hasher for Fx {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut v = [0u8; 8];
            v[..chunk.len()].copy_from_slice(chunk);
            self.0 = (self.0.rotate_left(5) ^ u64::from_le_bytes(v)).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
        }
    }
    fn write_u64(&mut self, i: u64) {
        self.0 = (self.0.rotate_left(5) ^ i).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
    fn write_u32(&mut self, i: u32) {
        self.write_u64(u64::from(i));
    }
}
pub(crate) type FxMap<K, V> = HashMap<K, V, BuildHasherDefault<Fx>>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScopeKind {
    Module,
    Class,
    Function,
    Lambda,
    Comprehension,
}

#[derive(Clone, Copy)]
pub(crate) struct Scope {
    pub kind: ScopeKind,
    pub parent: u32,
    pub node: u32,
}

#[derive(Clone, Copy)]
pub(crate) struct Binding {
    pub name: u32,
    pub scope: u32,
    pub node: u32,
    pub kind: u8,
}

#[derive(Default)]
pub(crate) struct Sema {
    pub scopes: Vec<Scope>,
    pub bindings: Vec<Binding>,
    /// scope of every node id
    pub scope_of: Vec<u32>,
    /// interned name per file (text -> symbol)
    pub names: FxMap<u64, u32>,
    /// (scope, symbol) -> last binding
    pub lookup: FxMap<(u32, u32), u32>,
    /// for each node id that is a load: resolved binding or u32::MAX
    pub resolved: Vec<u32>,
    pub resolved_count: usize,
    stack: Vec<(u32, u32)>,
}

pub(crate) const B_ASSIGN: u8 = 0;
pub(crate) const B_PARAM: u8 = 1;
pub(crate) const B_DEF: u8 = 2;
pub(crate) const B_IMPORT: u8 = 3;
pub(crate) const B_GLOBAL: u8 = 4;
pub(crate) const B_OTHER: u8 = 5;

impl Sema {
    fn intern(&mut self, text: &str) -> u32 {
        let next = self.names.len() as u32;
        *self.names.entry(hash_text(text)).or_insert(next)
    }

    fn bind(&mut self, text: &str, scope: u32, node: u32, kind: u8) {
        let name = self.intern(text);
        let id = self.bindings.len() as u32;
        self.bindings.push(Binding { name, scope, node, kind });
        self.lookup.insert((scope, name), id);
    }

    pub(crate) fn build(&mut self, table: &Table<'_>, _module: &ruff_python_ast::Mod, _source: &str) {
        self.scopes.clear();
        self.bindings.clear();
        self.scope_of.clear();
        self.names.clear();
        self.lookup.clear();
        self.resolved.clear();
        self.resolved_count = 0;
        self.stack.clear();
        let n = table.len();
        self.scope_of.resize(n, 0);
        self.resolved.resize(n, u32::MAX);
        self.scopes.push(Scope { kind: ScopeKind::Module, parent: u32::MAX, node: 0 });
        self.stack.push((0, n as u32));
        for id in 0..n {
            while self.stack.last().is_some_and(|&(_, end)| id as u32 >= end) {
                self.stack.pop();
            }
            let current = self.stack.last().map_or(0, |&(s, _)| s);
            self.scope_of[id] = current;
            let node = table.node[id];
            let new_scope = match node {
                AnyNodeRef::StmtFunctionDef(f) => {
                    self.bind(f.name.as_str(), current, id as u32, B_DEF);
                    Some(ScopeKind::Function)
                }
                AnyNodeRef::StmtClassDef(c) => {
                    self.bind(c.name.as_str(), current, id as u32, B_DEF);
                    Some(ScopeKind::Class)
                }
                AnyNodeRef::ExprLambda(_) => Some(ScopeKind::Lambda),
                AnyNodeRef::ExprListComp(_)
                | AnyNodeRef::ExprSetComp(_)
                | AnyNodeRef::ExprDictComp(_)
                | AnyNodeRef::ExprGenerator(_) => Some(ScopeKind::Comprehension),
                AnyNodeRef::ExprName(name) => {
                    if !matches!(name.ctx, ExprContext::Load) {
                        self.bind(name.id.as_str(), current, id as u32, B_ASSIGN);
                    }
                    None
                }
                AnyNodeRef::Parameter(p) => {
                    self.bind(p.name.as_str(), current, id as u32, B_PARAM);
                    None
                }
                AnyNodeRef::Alias(a) => {
                    let text = a.asname.as_ref().map_or_else(
                        || a.name.as_str().split('.').next().unwrap_or(""),
                        |asname| asname.as_str(),
                    );
                    self.bind(text, current, id as u32, B_IMPORT);
                    None
                }
                AnyNodeRef::ExceptHandlerExceptHandler(h) => {
                    if let Some(name) = &h.name {
                        self.bind(name.as_str(), current, id as u32, B_OTHER);
                    }
                    None
                }
                AnyNodeRef::StmtGlobal(g) => {
                    for name in &g.names {
                        self.bind(name.as_str(), current, id as u32, B_GLOBAL);
                    }
                    None
                }
                AnyNodeRef::StmtNonlocal(g) => {
                    for name in &g.names {
                        self.bind(name.as_str(), current, id as u32, B_GLOBAL);
                    }
                    None
                }
                AnyNodeRef::PatternMatchAs(p) => {
                    if let Some(name) = &p.name {
                        self.bind(name.as_str(), current, id as u32, B_OTHER);
                    }
                    None
                }
                AnyNodeRef::PatternMatchStar(p) => {
                    if let Some(name) = &p.name {
                        self.bind(name.as_str(), current, id as u32, B_OTHER);
                    }
                    None
                }
                _ => None,
            };
            if let Some(kind) = new_scope {
                let scope = self.scopes.len() as u32;
                self.scopes.push(Scope { kind, parent: current, node: id as u32 });
                self.stack.push((scope, table.end[id]));
            }
        }
        // resolve every load through the scope chain, from the Name bucket only
        let name_kind = NodeKind::ExprName as u8;
        for &id in table.of_kind(name_kind) {
            let AnyNodeRef::ExprName(name) = table.node[id as usize] else { continue };
            if !matches!(name.ctx, ExprContext::Load) {
                continue;
            }
            let Some(&symbol) = self.names.get(&hash_text(name.id.as_str())) else { continue };
            let mut scope = self.scope_of[id as usize];
            let mut first = true;
            loop {
                let s = self.scopes[scope as usize];
                if first || s.kind != ScopeKind::Class {
                    if let Some(&binding) = self.lookup.get(&(scope, symbol)) {
                        self.resolved[id as usize] = binding;
                        self.resolved_count += 1;
                        break;
                    }
                }
                first = false;
                if s.parent == u32::MAX {
                    break;
                }
                scope = s.parent;
            }
        }
    }
}

pub(crate) fn hash_text(text: &str) -> u64 {
    let mut h = Fx::default();
    h.write(text.as_bytes());
    h.write_u64(text.len() as u64);
    h.finish()
}
