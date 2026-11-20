//! Bake-off round 2 (bead ryo-emlr, throwaway): the Python semantic model in the provider's terms, built from the parser's own tree
//! (walk's flat table, ids = pre-order over ruff's nodes, identifiers included) in passes the language needs:
//!
//! 1. one pre-order pass: the scope every node is evaluated in (Python's rules: a definition's decorators, defaults, annotations
//!    and bases are evaluated around it, a comprehension's first iterable around the comprehension), names interned, `global` /
//!    `nonlocal` declarations, definitions and namespace hazards (wildcard imports, deletions);
//! 2. definitions' owners (after every declaration of the file is known: a `global` later in a function still routes its
//!    bindings), indexed by (namespace, name);
//! 3. the evaluation schedule: one walk in evaluation order (values before targets, a callable's body after the phase that defines
//!    it, a generator's body after its first iterable, structured alternatives for branches and loops), then its interpretation
//!    over sparse environments: binding events, reads with their state, uses, shadows; then schedule-order resolution.
//!
//! Every behaviour mirrors `languages/python/src/domain/{scopes,bindings,definitions,qualification,binding_*}` (the oracle is the
//! provider's own rows); the provider function each part follows is named at it.

use crate::builtins::{ADDED_BUILTINS, BUILTIN_NAMES};
use crate::sema::{FxMap, hash_text};
use crate::table::Table;
use ruff_python_ast::{self as ast, AnyNodeRef, ExprContext};

pub(crate) const NONE: u32 = u32::MAX;
/// The unknown name of a wildcard import (the provider's `unknown(WildcardImport)`).
pub(crate) const UNKNOWN: u32 = u32::MAX - 1;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Redirect {
    Global,
    Nonlocal,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DefKind {
    Function,
    Method,
    Class,
    Import,
    Variable,
    TypeAlias,
    Parameter,
    TypeParameter,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Def {
    pub at: u32,
    pub name: u32,
    pub owner: u32,
    pub kind: DefKind,
    pub word: &'static str,
    /// the node whose span names it (an identifier, a name, or for `import a.b` the first name: start/stop in bytes)
    pub named_start: u32,
    pub named_stop: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Action {
    Read,
    Bind,
    Annotation,
    Delete,
    Cleanup,
    Global,
    Nonlocal,
    BeginPhase,
    EndPhase,
    BeginAlternatives,
    NextAlternative,
    EndAlternatives,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Step {
    pub at: u32,
    pub phase: u32,
    pub action: Action,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PhaseKind {
    Callable,
    Generator,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Phase {
    pub root: u32,
    pub parent: u32,
    pub kind: PhaseKind,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum EventKind {
    Wildcard,
    Read,
    Bind,
    Annotation,
    Delete,
    Cleanup,
    Global,
    Nonlocal,
}

impl EventKind {
    pub(crate) const fn word(self) -> &'static str {
        match self {
            Self::Wildcard => "wildcard",
            Self::Read => "read",
            Self::Bind => "bind",
            Self::Annotation => "annotation",
            Self::Delete => "delete",
            Self::Cleanup => "cleanup",
            Self::Global => "global",
            Self::Nonlocal => "nonlocal",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Event {
    pub at: u32,
    pub name: u32,
    pub scope: u32,
    pub phase: u32,
    pub ordinal: u32,
    pub kind: EventKind,
    pub lexical: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Context {
    Value,
    Type,
    Deletion,
}

impl Context {
    pub(crate) const fn word(self) -> &'static str {
        match self {
            Self::Value => "value",
            Self::Type => "type",
            Self::Deletion => "deletion",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ReadState {
    Unique,
    Alternatives,
    PossiblyUnbound,
    Unbound,
    Builtin(i64, i64),
    DeclarationOrBuiltin(i64, i64),
    Uncertain,
}

impl ReadState {
    pub(crate) const fn word(self) -> &'static str {
        match self {
            Self::Unique => "unique-declaration",
            Self::Alternatives => "alternative-declarations",
            Self::PossiblyUnbound => "possibly-unbound",
            Self::Unbound => "unbound",
            Self::Builtin(..) => "builtin",
            Self::DeclarationOrBuiltin(..) => "declaration-or-builtin",
            Self::Uncertain => "uncertain",
        }
    }
    pub(crate) const fn since(self) -> (i64, i64) {
        match self {
            Self::Builtin(a, b) | Self::DeclarationOrBuiltin(a, b) => (a, b),
            _ => (0, 0),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Read {
    pub at: u32,
    pub name: u32,
    pub scope: u32,
    pub phase: u32,
    pub context: Context,
    pub state: ReadState,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Use {
    pub at: u32,
    pub binding: u32,
    pub scope: u32,
    pub phase: u32,
    pub context: Context,
    pub unique: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Shadow {
    pub binding: u32,
    pub previous: u32,
    pub scope: u32,
    pub previous_scope: u32,
    pub phase: u32,
    pub lexical: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Resolution {
    pub at: u32,
    pub event: u32,
    pub kind: EventKind,
    pub scope: u32,
    pub lexical: u32,
    pub depth: u32,
    pub ordinal: u32,
    pub context: Context,
}

#[derive(Clone, Copy)]
struct Alternatives {
    start: u32,
    end: u32,
    absent: bool,
    uncertain: bool,
}

#[derive(Clone)]
struct Frame {
    parent: u32,
    touched: Vec<(u32, u32)>,
    hazards: Vec<u32>,
}

#[derive(Clone, Copy)]
struct Mutation {
    at: u32,
    phase: u32,
    action: Action,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Candidate {
    pub binding: u32,
    pub scope: u32,
}

/// What the parser's node is, in the terms the semantic passes test (one small integer per node, read from the typed node).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub(crate) enum K {
    Module,
    FunctionDef,
    ClassDef,
    Lambda,
    ListComp,
    SetComp,
    DictComp,
    Generator,
    Comprehension,
    Name,
    Attribute,
    Identifier,
    Alias,
    Parameter,
    Parameters,
    ParameterWithDefault,
    ExceptHandler,
    MatchAs,
    MatchStar,
    MatchMapping,
    TypeVar,
    TypeVarTuple,
    ParamSpec,
    TypeParams,
    Global,
    Nonlocal,
    NamedExpr,
    TypeAlias,
    AnnAssign,
    Assign,
    ImportFrom,
    Import,
    Statement,
    Expression,
    Other,
}

fn k_of(node: AnyNodeRef<'_>) -> K {
    use AnyNodeRef as N;
    match node {
        N::ModModule(_) | N::ModExpression(_) => K::Module,
        N::StmtFunctionDef(_) => K::FunctionDef,
        N::StmtClassDef(_) => K::ClassDef,
        N::ExprLambda(_) => K::Lambda,
        N::ExprListComp(_) => K::ListComp,
        N::ExprSetComp(_) => K::SetComp,
        N::ExprDictComp(_) => K::DictComp,
        N::ExprGenerator(_) => K::Generator,
        N::Comprehension(_) => K::Comprehension,
        N::ExprName(_) => K::Name,
        N::ExprAttribute(_) => K::Attribute,
        N::Identifier(_) => K::Identifier,
        N::Alias(_) => K::Alias,
        N::Parameter(_) => K::Parameter,
        N::Parameters(_) => K::Parameters,
        N::ParameterWithDefault(_) => K::ParameterWithDefault,
        N::ExceptHandlerExceptHandler(_) => K::ExceptHandler,
        N::PatternMatchAs(_) => K::MatchAs,
        N::PatternMatchStar(_) => K::MatchStar,
        N::PatternMatchMapping(_) => K::MatchMapping,
        N::TypeParamTypeVar(_) => K::TypeVar,
        N::TypeParamTypeVarTuple(_) => K::TypeVarTuple,
        N::TypeParamParamSpec(_) => K::ParamSpec,
        N::TypeParams(_) => K::TypeParams,
        N::StmtGlobal(_) => K::Global,
        N::StmtNonlocal(_) => K::Nonlocal,
        N::ExprNamed(_) => K::NamedExpr,
        N::StmtTypeAlias(_) => K::TypeAlias,
        N::StmtAnnAssign(_) => K::AnnAssign,
        N::StmtAssign(_) => K::Assign,
        N::StmtImportFrom(_) => K::ImportFrom,
        N::StmtImport(_) => K::Import,
        n if n.is_statement() => K::Statement,
        n if n.is_expression() => K::Expression,
        _ => K::Other,
    }
}

const fn is_scope(k: K) -> bool {
    matches!(k, K::Module | K::FunctionDef | K::ClassDef | K::Lambda | K::ListComp | K::SetComp | K::DictComp | K::Generator)
}
const fn is_comprehension_scope(k: K) -> bool {
    matches!(k, K::ListComp | K::SetComp | K::DictComp | K::Generator)
}
/// `bindings::execution`: a scope whose code runs after the statements around it.
const fn deferred_scope(k: K) -> bool {
    matches!(k, K::FunctionDef | K::Lambda | K::Generator)
}
const fn is_expression(k: K) -> bool {
    matches!(
        k,
        K::Expression | K::Name | K::Attribute | K::Lambda | K::ListComp | K::SetComp | K::DictComp | K::Generator | K::NamedExpr
    )
}
const fn is_statement(k: K) -> bool {
    matches!(
        k,
        K::Statement | K::FunctionDef | K::ClassDef | K::Global | K::Nonlocal | K::TypeAlias | K::AnnAssign | K::Assign | K::ImportFrom | K::Import
    )
}

#[derive(Default)]
pub(crate) struct Model {
    // ---- pass 1
    pub k: Vec<K>,
    /// the scope node each node is evaluated in (`scopes::enclosing`)
    pub scope_of: Vec<u32>,
    /// interned name of a Name or Identifier node
    pub sym: Vec<u32>,
    /// Name context (ruff's, equal to `scopes::context` on Django's names)
    pub store: Vec<u8>,
    /// whether a node is an annotation or a return annotation (the field the provider calls `annotation`/`returns`)
    pub annotation: Vec<bool>,
    pub sym_map: FxMap<u64, u32>,
    pub sym_text: String,
    pub sym_span: Vec<(u32, u32)>,
    pub sym_builtin: Vec<Option<(i64, i64)>>,
    /// facts: each interned name's code in the run's symbols, filled on first use in the file (u32::MAX - 2: not yet)
    pub run_sym: Vec<u32>,
    pub declared: FxMap<(u32, u32), Redirect>,
    pub def_nodes: Vec<u32>,
    pub hazards: Vec<(u32, u32, u32)>, // (scope, at, name or NONE for wildcard)
    pub del_names: Vec<u32>,
    pub incomplete: bool,
    // ---- pass 2
    pub defs: Vec<Def>,
    /// definition index of a node, NONE when it defines nothing
    pub def_of: Vec<u32>,
    /// (owner, name) -> (start, end) into `sorted`
    pub def_index: FxMap<(u32, u32), (u32, u32)>,
    pub sorted: Vec<u32>,
    /// lexical resolution of a name load (`bindings::resolve`), memoised: NONE unresolved, NONE-1 not asked
    pub resolved: Vec<u32>,
    // ---- pass 3
    pub steps: Vec<Step>,
    pub phases: Vec<Phase>,
    pub events: Vec<Event>,
    pub reads: Vec<Read>,
    pub uses: Vec<Use>,
    pub shadows: Vec<Shadow>,
    pub resolutions: Vec<Resolution>,
    pub flow_incomplete: bool,
    frames: Vec<Frame>,
    active: usize,
    slots: FxMap<(u32, u32, u32), Alternatives>,
    arena: Vec<u32>,
    union: Vec<u32>,
    keys: Vec<(u32, u32)>,
    phase_frames: FxMap<u32, u32>,
    branches: Vec<(u32, usize)>,
    arms: Vec<u32>,
    candidates: Vec<Candidate>,
    mutations: FxMap<(u32, u32), Vec<Mutation>>,
    mutation_pool: Vec<Vec<Mutation>>,
    latest: FxMap<(u32, u32), u32>,
}

const NOT_ASKED: u32 = u32::MAX - 1;

impl Model {
    pub(crate) fn text(&self, sym: u32) -> Option<&str> {
        if sym >= UNKNOWN {
            return None;
        }
        let (a, b) = self.sym_span[sym as usize];
        Some(&self.sym_text[a as usize..b as usize])
    }

    fn intern(&mut self, text: &str) -> u32 {
        let h = hash_text(text);
        if let Some(&s) = self.sym_map.get(&h) {
            return s;
        }
        let s = self.sym_span.len() as u32;
        let a = self.sym_text.len() as u32;
        self.sym_text.push_str(text);
        self.sym_span.push((a, self.sym_text.len() as u32));
        self.sym_builtin.push(builtin_named(text));
        self.sym_map.insert(h, s);
        s
    }

    pub(crate) fn builtin(&self, sym: u32) -> Option<(i64, i64)> {
        if sym >= UNKNOWN { None } else { self.sym_builtin[sym as usize] }
    }

    fn clear(&mut self) {
        self.k.clear();
        self.scope_of.clear();
        self.sym.clear();
        self.store.clear();
        self.annotation.clear();
        self.sym_map.clear();
        self.sym_text.clear();
        self.sym_span.clear();
        self.sym_builtin.clear();
        self.run_sym.clear();
        self.declared.clear();
        self.def_nodes.clear();
        self.hazards.clear();
        self.del_names.clear();
        self.defs.clear();
        self.def_of.clear();
        self.def_index.clear();
        self.sorted.clear();
        self.resolved.clear();
        self.incomplete = false;
    }

    // ------------------------------------------------------------------------------------------------------------------------
    // pass 1 + 2: scopes, names, declarations, definitions

    pub(crate) fn build_lexical(&mut self, table: &Table<'_>, incomplete: bool) {
        self.clear();
        self.incomplete = incomplete;
        let n = table.len();
        self.k.reserve(n);
        for id in 0..n {
            self.k.push(k_of(table.node[id]));
        }
        self.scope_of.resize(n, 0);
        self.sym.resize(n, NONE);
        self.store.resize(n, 0);
        self.annotation.resize(n, false);
        for id in 1..n {
            let p = table.parent[id] as usize;
            let pk = self.k[p];
            // scopes::enclosing, computed from the parent's answer
            let scope = if p == 0 {
                0
            } else if pk == K::Comprehension && self.first_iterable(table, p, id) {
                self.scope_of[table.parent[p] as usize]
            } else if is_scope(pk) && self.inside(pk, id) {
                p as u32
            } else {
                self.scope_of[p]
            };
            self.scope_of[id] = scope;
            match table.node[id] {
                AnyNodeRef::ExprName(name) => {
                    self.sym[id] = self.intern(name.id.as_str());
                    self.store[id] = match name.ctx {
                        ExprContext::Load | ExprContext::Invalid => 0,
                        ExprContext::Store => 1,
                        ExprContext::Del => 2,
                    };
                    match self.store[id] {
                        1 => self.def_nodes.push(id as u32),
                        2 => self.del_names.push(id as u32),
                        _ => {}
                    }
                }
                AnyNodeRef::Identifier(ident) => {
                    self.sym[id] = self.intern(ident.id.as_str());
                }
                AnyNodeRef::StmtGlobal(_) | AnyNodeRef::StmtNonlocal(_) => {}
                AnyNodeRef::StmtFunctionDef(f) => {
                    if let Some(r) = &f.returns {
                        self.annotation[Table::id_of(AnyNodeRef::from(&**r)) as usize] = true;
                    }
                    self.def_nodes.push(id as u32);
                }
                AnyNodeRef::Parameter(p) => {
                    if let Some(a) = &p.annotation {
                        self.annotation[Table::id_of(AnyNodeRef::from(&**a)) as usize] = true;
                    }
                    self.def_nodes.push(id as u32);
                }
                AnyNodeRef::StmtAnnAssign(a) => {
                    self.annotation[Table::id_of(AnyNodeRef::from(&*a.annotation)) as usize] = true;
                }
                AnyNodeRef::StmtClassDef(_) => self.def_nodes.push(id as u32),
                AnyNodeRef::ExceptHandlerExceptHandler(h) if h.name.is_some() => self.def_nodes.push(id as u32),
                AnyNodeRef::PatternMatchAs(m) if m.name.is_some() => self.def_nodes.push(id as u32),
                AnyNodeRef::PatternMatchStar(m) if m.name.is_some() => self.def_nodes.push(id as u32),
                AnyNodeRef::PatternMatchMapping(m) if m.rest.is_some() => self.def_nodes.push(id as u32),
                AnyNodeRef::Alias(a) => {
                    if a.name.as_str() == "*" {
                        self.hazards.push((scope, id as u32, NONE));
                    } else if a.asname.is_none() {
                        // `import a.b` binds `a`, the first name of the dotted name (`imports::import_binding`)
                        let head = a.name.as_str().split('.').next().unwrap_or("");
                        self.intern(head);
                    }
                    self.def_nodes.push(id as u32);
                }
                AnyNodeRef::TypeParamTypeVar(_) | AnyNodeRef::TypeParamTypeVarTuple(_) | AnyNodeRef::TypeParamParamSpec(_) => {
                    self.def_nodes.push(id as u32);
                }
                _ => {}
            }
        }
        // identifiers of `global`/`nonlocal` are children of the statement, interned above; declarations need them
        for id in 1..n {
            let redirect = match self.k[id] {
                K::Global => Redirect::Global,
                K::Nonlocal => Redirect::Nonlocal,
                _ => continue,
            };
            let scope = self.scope_of[id];
            let mut c = id + 1;
            while c < table.end[id] as usize {
                if self.k[c] == K::Identifier {
                    self.declared.entry((scope, self.sym[c])).or_insert(redirect);
                }
                c = table.end[c] as usize;
            }
        }
        // deletions as hazards (`qualification::prepare`): their namespace is where the name is bound
        for i in 0..self.del_names.len() {
            let at = self.del_names[i];
            let name = self.sym[at as usize];
            let scope = self.binding_scope(table, at, name);
            self.hazards.push((scope, at, name));
        }
        self.hazards.sort_unstable_by_key(|h| h.1);
        // definitions (`definitions::definition`) with their owners
        self.def_of.resize(n, NONE);
        for i in 0..self.def_nodes.len() {
            let at = self.def_nodes[i];
            if let Some(def) = self.definition(table, at) {
                self.def_of[at as usize] = self.defs.len() as u32;
                self.defs.push(def);
            }
        }
        self.sorted.extend(0..self.defs.len() as u32);
        let defs = &self.defs;
        self.sorted.sort_unstable_by_key(|&d| (defs[d as usize].owner, defs[d as usize].name, defs[d as usize].at));
        let mut i = 0;
        while i < self.sorted.len() {
            let d = self.defs[self.sorted[i] as usize];
            let mut j = i + 1;
            while j < self.sorted.len() {
                let e = self.defs[self.sorted[j] as usize];
                if e.owner != d.owner || e.name != d.name {
                    break;
                }
                j += 1;
            }
            self.def_index.insert((d.owner, d.name), (i as u32, j as u32));
            i = j;
        }
        self.resolved.resize(n, NOT_ASKED);
    }

    /// `scopes::evaluation`: the child is the iterable of the first clause of its comprehension.
    fn first_iterable(&self, table: &Table<'_>, clause: usize, child: usize) -> bool {
        let AnyNodeRef::Comprehension(c) = table.node[clause] else { return false };
        if Table::id_of(AnyNodeRef::from(&c.iter)) as usize != child {
            return false;
        }
        let owner = table.parent[clause] as usize;
        let mut first = owner + 1;
        while first < table.end[owner] as usize {
            if self.k[first] == K::Comprehension {
                return first == clause;
            }
            first = table.end[first] as usize;
        }
        false
    }

    /// `scopes::inside`
    fn inside(&self, scope: K, child: usize) -> bool {
        match scope {
            K::FunctionDef | K::ClassDef => is_statement(self.k[child]) || self.k[child] == K::TypeParams,
            K::Lambda => is_expression(self.k[child]),
            _ => true,
        }
    }

    /// `scopes::lexical_scope`
    pub(crate) fn lexical_scope(&self, table: &Table<'_>, at: u32) -> u32 {
        let mut scope = self.scope_of[at as usize];
        let p = table.parent[at as usize] as usize;
        if self.k[p] == K::NamedExpr && p + 1 == at as usize {
            // the target is the named expression's first expression child
            while is_comprehension_scope(self.k[scope as usize]) {
                scope = self.scope_of[scope as usize];
            }
        }
        scope
    }

    /// `scopes::function_around`
    fn function_around(&self, scope: u32) -> u32 {
        let mut around = self.scope_of[scope as usize];
        while self.k[around as usize] != K::FunctionDef && around != 0 {
            around = self.scope_of[around as usize];
        }
        around
    }

    /// `scopes::redirected`
    pub(crate) fn redirected(&self, scope: u32, name: u32) -> u32 {
        if self.declared.is_empty() {
            return scope;
        }
        match self.declared.get(&(scope, name)) {
            Some(Redirect::Global) => 0,
            Some(Redirect::Nonlocal) => self.function_around(scope),
            None => scope,
        }
    }

    pub(crate) fn declares_global(&self, scope: u32, name: u32) -> bool {
        self.declared.get(&(scope, name)) == Some(&Redirect::Global)
    }

    /// `scopes::binding_scope`
    pub(crate) fn binding_scope(&self, table: &Table<'_>, at: u32, name: u32) -> u32 {
        self.redirected(self.lexical_scope(table, at), name)
    }

    /// `definitions::definition`
    fn definition(&self, table: &Table<'_>, at: u32) -> Option<Def> {
        let id = at as usize;
        let ident = |i: &ast::Identifier| -> (u32, u32, u32) {
            let x = Table::id_of(AnyNodeRef::from(i)) as usize;
            (self.sym[x], table.start[x], table.stop[x])
        };
        let named = |owner_at: u32, i: &ast::Identifier, kind: DefKind, word: &'static str| -> Option<Def> {
            let (name, s, e) = ident(i);
            Some(Def { at: owner_at, name, owner: self.binding_scope(table, owner_at, name), kind, word, named_start: s, named_stop: e })
        };
        match table.node[id] {
            AnyNodeRef::StmtFunctionDef(f) => {
                let method = self.k[self.scope_of[id] as usize] == K::ClassDef;
                let (kind, word) = match (method, f.is_async) {
                    (true, true) => (DefKind::Method, "async-method"),
                    (true, false) => (DefKind::Method, "method"),
                    (false, true) => (DefKind::Function, "async-function"),
                    (false, false) => (DefKind::Function, "function"),
                };
                named(at, &f.name, kind, word)
            }
            AnyNodeRef::StmtClassDef(c) => named(at, &c.name, DefKind::Class, "class"),
            AnyNodeRef::ExceptHandlerExceptHandler(h) => named(at, h.name.as_ref()?, DefKind::Variable, "exception"),
            AnyNodeRef::PatternMatchAs(m) => named(at, m.name.as_ref()?, DefKind::Variable, "pattern"),
            AnyNodeRef::PatternMatchStar(m) => named(at, m.name.as_ref()?, DefKind::Variable, "pattern"),
            AnyNodeRef::PatternMatchMapping(m) => named(at, m.rest.as_ref()?, DefKind::Variable, "pattern"),
            AnyNodeRef::Alias(a) => {
                let from = self.k[table.parent[id] as usize] == K::ImportFrom;
                let word = if from { "from-import" } else { "import" };
                let first = a.name.as_str();
                let (name, s, e) = if first == "*" {
                    (UNKNOWN, a.name.range.start().to_u32(), a.name.range.end().to_u32())
                } else if let Some(asname) = &a.asname {
                    ident(asname)
                } else {
                    // the first name token of the imported name: `a` of `import a.b`
                    let head = first.split('.').next().unwrap_or("");
                    let s = a.name.range.start().to_u32();
                    let x = Table::id_of(AnyNodeRef::from(&a.name)) as usize;
                    let sym = if head.len() == first.len() { self.sym[x] } else { self.lookup_sym(head)? };
                    (sym, s, s + head.len() as u32)
                };
                Some(Def { at, name, owner: self.binding_scope(table, at, name), kind: DefKind::Import, word, named_start: s, named_stop: e })
            }
            AnyNodeRef::ExprName(_) => {
                if self.store[id] != 1 {
                    return None;
                }
                let name = self.sym[id];
                let p = table.parent[id] as usize;
                let alias = self.k[p] == K::TypeAlias && p + 1 == id;
                let (kind, word) = if alias { (DefKind::TypeAlias, "type-alias") } else { (DefKind::Variable, "variable") };
                Some(Def { at, name, owner: self.binding_scope(table, at, name), kind, word, named_start: table.start[id], named_stop: table.stop[id] })
            }
            AnyNodeRef::Parameter(p) => {
                let (word, callable) = self.placed(table, id)?;
                let (name, s, e) = ident(&p.name);
                Some(Def { at, name, owner: callable, kind: DefKind::Parameter, word, named_start: s, named_stop: e })
            }
            AnyNodeRef::TypeParamTypeVar(t) => self.type_param(table, at, &t.name, "type-var"),
            AnyNodeRef::TypeParamTypeVarTuple(t) => self.type_param(table, at, &t.name, "type-var-tuple"),
            AnyNodeRef::TypeParamParamSpec(t) => self.type_param(table, at, &t.name, "param-spec"),
            _ => None,
        }
    }

    fn lookup_sym(&self, text: &str) -> Option<u32> {
        self.sym_map.get(&hash_text(text)).copied()
    }

    fn type_param(&self, table: &Table<'_>, at: u32, name: &ast::Identifier, word: &'static str) -> Option<Def> {
        let x = Table::id_of(AnyNodeRef::from(name)) as usize;
        let owner = table.parent[table.parent[at as usize] as usize];
        Some(Def { at, name: self.sym[x], owner, kind: DefKind::TypeParameter, word, named_start: table.start[x], named_stop: table.stop[x] })
    }

    /// `definitions::placed`: how a parameter takes its argument, and its callable.
    fn placed(&self, table: &Table<'_>, at: usize) -> Option<(&'static str, u32)> {
        let holder = table.parent[at] as usize;
        let element = if self.k[holder] == K::ParameterWithDefault { holder } else { at };
        let parameters = table.parent[element] as usize;
        let AnyNodeRef::Parameters(ps) = table.node[parameters] else { return None };
        let callable = table.parent[parameters];
        let is = |p: &ast::ParameterWithDefault| Table::id_of(AnyNodeRef::from(p)) as usize == element;
        let word = if ps.posonlyargs.iter().any(is) {
            "positional-only"
        } else if ps.args.iter().any(is) {
            "positional-or-keyword"
        } else if ps.kwonlyargs.iter().any(is) {
            "keyword-only"
        } else if ps.vararg.as_deref().is_some_and(|v| Table::id_of(AnyNodeRef::from(v)) as usize == element) {
            "var-positional"
        } else if ps.kwarg.as_deref().is_some_and(|v| Table::id_of(AnyNodeRef::from(v)) as usize == element) {
            "var-keyword"
        } else {
            return None;
        };
        Some((word, callable))
    }

    // ------------------------------------------------------------------------------------------------------------------------
    // lexical queries

    /// `bindings::availability`: an assignment's right side is evaluated before its targets are bound.
    fn available(&self, table: &Table<'_>, binding: u32, occurrence: u32) -> bool {
        let mut above = binding as usize;
        loop {
            let k = self.k[above];
            if matches!(k, K::Assign | K::AnnAssign | K::NamedExpr) || (above == binding as usize && matches!(k, K::FunctionDef | K::ClassDef)) {
                return occurrence >= table.end[above] || (occurrence as usize) < above;
            }
            if matches!(k, K::Module | K::FunctionDef | K::ClassDef | K::Lambda) || above == 0 {
                return true;
            }
            above = table.parent[above] as usize;
        }
    }

    /// `bindings::resolve`: the lexical declaration of a name expression.
    pub(crate) fn resolve(&self, table: &Table<'_>, at: u32, name: u32) -> Option<u32> {
        let initial = self.scope_of[at as usize];
        let mut scope = initial;
        let mut delayed = false;
        loop {
            let lookup = self.redirected(scope, name);
            delayed |= lookup != scope && deferred_scope(self.k[scope as usize]);
            let lk = self.k[lookup as usize];
            let class = lk == K::ClassDef;
            if !class || scope == initial {
                if let Some(&(s, e)) = self.def_index.get(&(lookup, name)) {
                    let range = &self.sorted[s as usize..e as usize];
                    let lexical = matches!(lk, K::FunctionDef | K::Lambda | K::ListComp | K::SetComp | K::DictComp | K::Generator);
                    if let Some(&d) = range.iter().rev().find(|&&d| {
                        let pos = self.defs[d as usize].at;
                        (delayed || pos < at) && (delayed || lexical || self.available(table, pos, at))
                    }) {
                        return Some(self.defs[d as usize].at);
                    }
                    if lexical {
                        return Some(self.defs[range[0] as usize].at);
                    }
                }
            }
            delayed |= deferred_scope(self.k[scope as usize]);
            if lookup == 0 {
                return None;
            }
            scope = self.scope_of[scope as usize];
        }
    }

    /// The lexical declaration of every name load, memoised (what `resolves` and `qualified-name` read).
    pub(crate) fn resolve_loads(&mut self, table: &Table<'_>) {
        for &id in table.of_kind(ruff_python_ast::NodeKind::ExprName as u8) {
            if self.store[id as usize] == 0 {
                self.resolve_memo(table, id);
            }
        }
    }

    pub(crate) fn resolve_memo(&mut self, table: &Table<'_>, at: u32) -> u32 {
        let r = self.resolved[at as usize];
        if r != NOT_ASKED {
            return r;
        }
        let r = self.resolve(table, at, self.sym[at as usize]).unwrap_or(NONE);
        self.resolved[at as usize] = r;
        r
    }

    /// `qualification::uncertain`: a wildcard import or a deletion between the binding and the read leaves the origin unproven.
    pub(crate) fn uncertain(&self, at: u32, name: u32, binding: Option<u32>) -> bool {
        if self.hazards.is_empty() {
            return false;
        }
        let binding_owner = binding.map(|b| self.defs[self.def_of[b as usize] as usize].owner);
        let builtin = self.builtin(name).is_some();
        let initial = self.scope_of[at as usize];
        let mut scope = initial;
        let mut deferred = false;
        loop {
            let visible = scope == initial || self.k[scope as usize] != K::ClassDef;
            if visible {
                for &(hs, hat, hname) in &self.hazards {
                    if hs != scope || !(deferred || hat < at) {
                        continue;
                    }
                    if hname == NONE {
                        if binding.is_none() && !builtin {
                            return true;
                        }
                    } else if hname == name && binding.is_none_or(|b| hat > b) {
                        return true;
                    }
                }
            }
            if binding_owner == Some(scope) || scope == 0 {
                return false;
            }
            if deferred_scope(self.k[scope as usize]) {
                deferred = true;
            }
            scope = self.scope_of[scope as usize];
        }
    }

    // ------------------------------------------------------------------------------------------------------------------------
    // pass 3: the evaluation schedule (`binding_walk`, `binding_schedule`) and its interpretation (`binding_analysis`)

    fn push(&mut self, at: u32, phase: u32, action: Action) {
        self.steps.push(Step { at, phase, action });
    }

    fn children<'b>(table: &'b Table<'_>, at: usize) -> impl Iterator<Item = usize> + 'b {
        let end = table.end[at] as usize;
        let mut c = at + 1;
        std::iter::from_fn(move || {
            if c >= end {
                return None;
            }
            let x = c;
            c = table.end[c] as usize;
            Some(x)
        })
    }

    fn walk_expr(&mut self, table: &Table<'_>, e: &ast::Expr, phase: u32) {
        self.walk(table, Table::id_of(AnyNodeRef::from(e)) as usize, phase);
    }

    fn walk_body(&mut self, table: &Table<'_>, body: &[ast::Stmt], phase: u32) {
        for s in body {
            self.walk(table, Table::id_of(AnyNodeRef::from(s)) as usize, phase);
        }
    }

    /// `binding_walk::parameter_values`: defaults and annotations run before the callable is entered.
    fn parameter_values(&mut self, table: &Table<'_>, at: usize, phase: u32) {
        if is_expression(self.k[at]) {
            self.walk(table, at, phase);
            return;
        }
        let end = table.end[at] as usize;
        let mut c = at + 1;
        while c < end {
            self.parameter_values(table, c, phase);
            c = table.end[c] as usize;
        }
    }

    /// `binding_walk::parameters`: parameter identities are bound at body entry.
    fn parameters(&mut self, table: &Table<'_>, at: usize, phase: u32) {
        if self.k[at] == K::Parameter {
            self.push(at as u32, phase, Action::Bind);
            return;
        }
        if is_expression(self.k[at]) {
            return;
        }
        let end = table.end[at] as usize;
        let mut c = at + 1;
        while c < end {
            self.parameters(table, c, phase);
            c = table.end[c] as usize;
        }
    }

    fn comprehension(&mut self, table: &Table<'_>, at: usize, generators: &[ast::Comprehension], phase: u32, deferred: bool) {
        if let Some(first) = generators.first() {
            self.walk_expr(table, &first.iter, phase);
        }
        if deferred {
            self.phases.push(Phase { root: at as u32, parent: phase, kind: PhaseKind::Generator });
            return;
        }
        self.comprehension_body(table, at, phase);
    }

    /// `binding_walk::comprehension_body`
    fn comprehension_body(&mut self, table: &Table<'_>, at: usize, phase: u32) {
        self.push(at as u32, phase, Action::BeginAlternatives);
        let (generators, results): (&[ast::Comprehension], [Option<&ast::Expr>; 2]) = match table.node[at] {
            AnyNodeRef::ExprListComp(c) => (&c.generators, [Some(&*c.elt), None]),
            AnyNodeRef::ExprSetComp(c) => (&c.generators, [Some(&*c.elt), None]),
            AnyNodeRef::ExprGenerator(c) => (&c.generators, [Some(&*c.elt), None]),
            AnyNodeRef::ExprDictComp(c) => (&c.generators, [c.key.as_deref(), Some(&*c.value)]),
            _ => (&[], [None, None]),
        };
        for (i, clause) in generators.iter().enumerate() {
            if i != 0 {
                self.walk_expr(table, &clause.iter, phase);
            }
            self.walk_expr(table, &clause.target, phase);
            for cond in &clause.ifs {
                self.walk_expr(table, cond, phase);
            }
        }
        for r in results.into_iter().flatten() {
            self.walk_expr(table, r, phase);
        }
        self.push(at as u32, phase, Action::NextAlternative);
        self.push(at as u32, phase, Action::EndAlternatives);
    }

    /// `binding_walk::walk`: syntax in the language's evaluation order.
    fn walk(&mut self, table: &Table<'_>, at: usize, phase: u32) {
        let node = table.node[at];
        let a = at as u32;
        match node {
            AnyNodeRef::StmtFunctionDef(_) | AnyNodeRef::ExprLambda(_) => {
                // every child but the body: decorators, identifier, type parameters, parameters, returns (source order)
                let body_start = match node {
                    AnyNodeRef::StmtFunctionDef(f) => f.body.first().map(|s| Table::id_of(AnyNodeRef::from(s)) as usize),
                    AnyNodeRef::ExprLambda(l) => Some(Table::id_of(AnyNodeRef::from(&*l.body)) as usize),
                    _ => None,
                };
                for c in Self::children(table, at).collect::<smallvec_like::Ids>() {
                    if Some(c) == body_start {
                        break;
                    }
                    self.parameter_values(table, c, phase);
                }
                if self.k[at] == K::FunctionDef {
                    self.push(a, phase, Action::Bind);
                }
                self.phases.push(Phase { root: a, parent: phase, kind: PhaseKind::Callable });
            }
            AnyNodeRef::StmtClassDef(_) => {
                for c in Self::children(table, at).collect::<smallvec_like::Ids>() {
                    self.walk(table, c, phase);
                }
                self.push(a, phase, Action::Bind);
            }
            AnyNodeRef::StmtAssign(s) => {
                self.walk_expr(table, &s.value, phase);
                for t in &s.targets {
                    self.walk_expr(table, t, phase);
                }
            }
            AnyNodeRef::ExprNamed(s) => {
                self.walk_expr(table, &s.value, phase);
                self.walk_expr(table, &s.target, phase);
            }
            AnyNodeRef::StmtAnnAssign(s) => {
                if let Some(v) = &s.value {
                    self.walk_expr(table, v, phase);
                }
                self.walk_expr(table, &s.annotation, phase);
                if s.value.is_some() {
                    self.walk_expr(table, &s.target, phase);
                } else if let ast::Expr::Name(_) = &*s.target {
                    self.push(Table::id_of(AnyNodeRef::from(&*s.target)), phase, Action::Annotation);
                } else {
                    self.walk_expr(table, &s.target, phase);
                }
            }
            AnyNodeRef::StmtAugAssign(s) => {
                if let ast::Expr::Name(_) = &*s.target {
                    self.push(Table::id_of(AnyNodeRef::from(&*s.target)), phase, Action::Read);
                } else {
                    self.walk_expr(table, &s.target, phase);
                }
                self.walk_expr(table, &s.value, phase);
                self.walk_expr(table, &s.target, phase);
            }
            AnyNodeRef::StmtIf(s) => {
                self.walk_expr(table, &s.test, phase);
                self.push(a, phase, Action::BeginAlternatives);
                self.walk_body(table, &s.body, phase);
                for clause in &s.elif_else_clauses {
                    self.push(a, phase, Action::NextAlternative);
                    self.walk(table, Table::id_of(AnyNodeRef::from(clause)) as usize, phase);
                }
                if s.elif_else_clauses.last().is_none_or(|c| c.test.is_some()) {
                    self.push(a, phase, Action::NextAlternative);
                }
                self.push(a, phase, Action::EndAlternatives);
            }
            AnyNodeRef::ExprIf(s) => {
                self.walk_expr(table, &s.test, phase);
                self.push(a, phase, Action::BeginAlternatives);
                self.walk_expr(table, &s.body, phase);
                self.push(a, phase, Action::NextAlternative);
                self.walk_expr(table, &s.orelse, phase);
                self.push(a, phase, Action::EndAlternatives);
            }
            AnyNodeRef::ExprBoolOp(s) => {
                let mut values = s.values.iter();
                if let Some(first) = values.next() {
                    self.walk_expr(table, first, phase);
                }
                for v in values {
                    self.push(a, phase, Action::BeginAlternatives);
                    self.walk_expr(table, v, phase);
                    self.push(a, phase, Action::NextAlternative);
                    self.push(a, phase, Action::EndAlternatives);
                }
            }
            AnyNodeRef::StmtMatch(s) => {
                self.walk_expr(table, &s.subject, phase);
                self.push(a, phase, Action::BeginAlternatives);
                for case in &s.cases {
                    self.walk(table, Table::id_of(AnyNodeRef::from(case)) as usize, phase);
                    self.push(a, phase, Action::NextAlternative);
                }
                self.push(a, phase, Action::EndAlternatives);
            }
            AnyNodeRef::StmtFor(s) => {
                self.walk_expr(table, &s.iter, phase);
                self.push(a, phase, Action::BeginAlternatives);
                self.walk_expr(table, &s.target, phase);
                self.walk_body(table, &s.body, phase);
                self.push(a, phase, Action::NextAlternative);
                self.push(a, phase, Action::EndAlternatives);
                self.walk_body(table, &s.orelse, phase);
            }
            AnyNodeRef::StmtWhile(s) => {
                self.walk_expr(table, &s.test, phase);
                self.push(a, phase, Action::BeginAlternatives);
                self.walk_body(table, &s.body, phase);
                self.push(a, phase, Action::NextAlternative);
                self.push(a, phase, Action::EndAlternatives);
                self.walk_body(table, &s.orelse, phase);
            }
            AnyNodeRef::StmtTry(s) => {
                self.push(a, phase, Action::BeginAlternatives);
                self.walk_body(table, &s.body, phase);
                self.walk_body(table, &s.orelse, phase);
                for h in &s.handlers {
                    self.push(a, phase, Action::NextAlternative);
                    self.walk(table, Table::id_of(AnyNodeRef::from(h)) as usize, phase);
                }
                self.push(a, phase, Action::EndAlternatives);
                self.walk_body(table, &s.finalbody, phase);
            }
            AnyNodeRef::ExceptHandlerExceptHandler(h) => {
                if let Some(t) = &h.type_ {
                    self.walk_expr(table, t, phase);
                }
                let named = self.def_of[at] != NONE;
                if named {
                    self.push(a, phase, Action::Bind);
                }
                self.walk_body(table, &h.body, phase);
                if named {
                    self.push(a, phase, Action::Cleanup);
                }
            }
            AnyNodeRef::ExprGenerator(c) => self.comprehension(table, at, &c.generators, phase, true),
            AnyNodeRef::ExprListComp(c) => self.comprehension(table, at, &c.generators, phase, false),
            AnyNodeRef::ExprSetComp(c) => self.comprehension(table, at, &c.generators, phase, false),
            AnyNodeRef::ExprDictComp(c) => self.comprehension(table, at, &c.generators, phase, false),
            AnyNodeRef::ExprName(_) => {
                let action = match self.store[at] {
                    0 => Action::Read,
                    1 => Action::Bind,
                    _ => Action::Delete,
                };
                self.push(a, phase, action);
            }
            AnyNodeRef::StmtGlobal(_) => self.push(a, phase, Action::Global),
            AnyNodeRef::StmtNonlocal(_) => self.push(a, phase, Action::Nonlocal),
            AnyNodeRef::Parameter(_) => self.parameter_values(table, at, phase),
            _ => {
                let end = table.end[at] as usize;
                let mut c = at + 1;
                while c < end {
                    self.walk(table, c, phase);
                    c = table.end[c] as usize;
                }
                if self.def_of[at] != NONE {
                    self.push(a, phase, Action::Bind);
                }
            }
        }
    }

    /// `binding_schedule::schedule` and `deferred`
    fn schedule(&mut self, table: &Table<'_>) {
        self.steps.clear();
        self.phases.clear();
        self.push(0, 0, Action::BeginPhase);
        self.walk(table, 0, 0);
        self.push(0, 0, Action::EndPhase);
        let mut next = 0;
        while next < self.phases.len() {
            let task = self.phases[next];
            next += 1;
            let root = task.root as usize;
            self.push(task.root, task.root, Action::BeginPhase);
            match task.kind {
                PhaseKind::Callable => {
                    match table.node[root] {
                        AnyNodeRef::StmtFunctionDef(f) => {
                            self.parameters(table, Table::id_of(AnyNodeRef::from(&*f.parameters)) as usize, task.root);
                            self.walk_body(table, &f.body, task.root);
                        }
                        AnyNodeRef::ExprLambda(l) => {
                            if let Some(p) = &l.parameters {
                                self.parameters(table, Table::id_of(AnyNodeRef::from(&**p)) as usize, task.root);
                            }
                            self.walk_expr(table, &l.body, task.root);
                        }
                        _ => {}
                    }
                }
                PhaseKind::Generator => self.comprehension_body(table, root, task.root),
            }
            self.push(task.root, task.root, Action::EndPhase);
        }
    }

    fn open(&mut self, parent: u32) -> u32 {
        let id = self.active;
        self.active += 1;
        if let Some(f) = self.frames.get_mut(id) {
            f.parent = parent;
            f.touched.clear();
            f.hazards.clear();
        } else {
            self.frames.push(Frame { parent, touched: Vec::new(), hazards: Vec::new() });
        }
        id as u32
    }

    fn lookup(&self, mut frame: u32, key: (u32, u32)) -> Option<Alternatives> {
        loop {
            if let Some(v) = self.slots.get(&(frame, key.0, key.1)) {
                return Some(*v);
            }
            let p = self.frames[frame as usize].parent;
            if p == NONE {
                return None;
            }
            frame = p;
        }
    }

    fn store_slot(&mut self, frame: u32, key: (u32, u32), value: Alternatives) {
        if self.slots.insert((frame, key.0, key.1), value).is_none() {
            self.frames[frame as usize].touched.push(key);
        }
    }

    fn assign(&mut self, frame: u32, key: (u32, u32), definition: u32) {
        let start = self.arena.len() as u32;
        self.arena.push(definition);
        self.store_slot(frame, key, Alternatives { start, end: start + 1, absent: false, uncertain: false });
    }

    fn delete(&mut self, frame: u32, key: (u32, u32)) {
        let end = self.arena.len() as u32;
        self.store_slot(frame, key, Alternatives { start: end, end, absent: true, uncertain: false });
    }

    fn annotate(&mut self, frame: u32, key: (u32, u32), declaration: u32) {
        if self.lookup(frame, key).is_some() {
            return;
        }
        self.assign(frame, key, declaration);
        if let Some(v) = self.slots.get_mut(&(frame, key.0, key.1)) {
            v.absent = true;
        }
    }

    fn wildcard(&mut self, frame: u32, scope: u32) {
        let f = &mut self.frames[frame as usize];
        if !f.hazards.contains(&scope) {
            f.hazards.push(scope);
        }
    }

    fn has_wildcard(&self, mut frame: u32, scope: u32) -> bool {
        loop {
            let f = &self.frames[frame as usize];
            if f.hazards.contains(&scope) {
                return true;
            }
            if f.parent == NONE {
                return false;
            }
            frame = f.parent;
        }
    }

    /// `binding_states::join`
    fn join(&mut self, target: u32, arms_from: usize) {
        let arms: Vec<u32> = self.arms[arms_from..].to_vec();
        for &arm in &arms {
            for i in 0..self.frames[arm as usize].hazards.len() {
                let scope = self.frames[arm as usize].hazards[i];
                self.wildcard(target, scope);
            }
        }
        self.keys.clear();
        for &arm in &arms {
            if arm != target {
                let touched = std::mem::take(&mut self.frames[arm as usize].touched);
                self.keys.extend_from_slice(&touched);
                self.frames[arm as usize].touched = touched;
            }
        }
        self.keys.sort_unstable();
        self.keys.dedup();
        for i in 0..self.keys.len() {
            let key = self.keys[i];
            self.union.clear();
            let mut absent = false;
            let mut uncertain = false;
            for &arm in &arms {
                if let Some(v) = self.lookup(arm, key) {
                    let (s, e) = (v.start as usize, v.end as usize);
                    for j in s..e {
                        let x = self.arena[j];
                        self.union.push(x);
                    }
                    absent |= v.absent;
                    uncertain |= v.uncertain;
                } else {
                    absent = true;
                }
            }
            self.union.sort_unstable();
            self.union.dedup();
            let start = self.arena.len() as u32;
            let u = std::mem::take(&mut self.union);
            self.arena.extend_from_slice(&u);
            self.union = u;
            let end = self.arena.len() as u32;
            self.store_slot(target, key, Alternatives { start, end, absent, uncertain });
        }
    }

    /// `binding_lookup::annotation_only`
    fn annotation_only(&self, table: &Table<'_>, at: u32) -> bool {
        let p = table.parent[at as usize] as usize;
        matches!(table.node[p], AnyNodeRef::StmtAnnAssign(s) if s.value.is_none())
    }

    /// `binding_lookup::resolve`: the flow-sensitive candidates of a name at a point of the schedule.
    fn flow_resolve(&mut self, table: &Table<'_>, frame: u32, reference: u32, name: u32) -> ReadState {
        self.candidates.clear();
        let initial = self.scope_of[reference as usize];
        let mut scope = self.redirected(initial, name);
        let mut uncertain = false;
        let mut wildcard = false;
        let classified = |n: usize, absent: bool, uncertain: bool| -> ReadState {
            if uncertain {
                ReadState::Uncertain
            } else if n == 0 {
                ReadState::Unbound
            } else if absent {
                ReadState::PossiblyUnbound
            } else if n == 1 {
                ReadState::Unique
            } else {
                ReadState::Alternatives
            }
        };
        loop {
            let visible = scope == initial || self.k[scope as usize] != K::ClassDef;
            if visible && self.has_wildcard(frame, scope) {
                wildcard = true;
            }
            if visible && let Some(v) = self.lookup(frame, (scope, name)) {
                for j in v.start..v.end {
                    let b = self.arena[j as usize];
                    if !self.annotation_only(table, b) {
                        self.candidates.push(Candidate { binding: b, scope });
                    }
                }
                uncertain |= v.uncertain;
                if !v.absent {
                    return classified(self.candidates.len(), false, uncertain);
                }
                if matches!(self.k[scope as usize], K::FunctionDef | K::Lambda | K::Generator | K::ListComp | K::SetComp | K::DictComp) {
                    return classified(self.candidates.len(), true, uncertain);
                }
            }
            if scope == 0 {
                break;
            }
            scope = self.redirected(self.scope_of[scope as usize], name);
        }
        if uncertain {
            return ReadState::Uncertain;
        }
        if let Some((major, minor)) = self.builtin(name) {
            if self.candidates.is_empty() { ReadState::Builtin(major, minor) } else { ReadState::DeclarationOrBuiltin(major, minor) }
        } else {
            classified(self.candidates.len(), true, wildcard)
        }
    }

    fn within(table: &Table<'_>, at: u32, region: u32) -> bool {
        // `binding_uncertainty::within`: at is region or below it (pre-order subtree)
        at >= region && at < table.end[region as usize]
    }

    /// `binding_uncertainty::repeats_reference`
    fn repeats_reference(&self, table: &Table<'_>, reference: u32, region: u32) -> bool {
        match table.node[region as usize] {
            AnyNodeRef::StmtWhile(_) => true,
            AnyNodeRef::StmtFor(f) => {
                if reference == region {
                    return false;
                }
                let mut at = reference;
                while table.parent[at as usize] != region {
                    if at == 0 {
                        return false;
                    }
                    at = table.parent[at as usize];
                }
                Table::id_of(AnyNodeRef::from(&*f.iter)) != at
            }
            AnyNodeRef::ExprListComp(_) | AnyNodeRef::ExprSetComp(_) | AnyNodeRef::ExprDictComp(_) | AnyNodeRef::ExprGenerator(_) => {
                Self::within(table, self.scope_of[reference as usize], region)
            }
            _ => false,
        }
    }

    /// `binding_uncertainty::protected_body`: the mutation is in the `try` body the handler protects.
    fn protected_body(table: &Table<'_>, mutation: u32, handler: u32) -> bool {
        let statement = table.parent[handler as usize];
        let AnyNodeRef::StmtTry(t) = table.node[statement as usize] else { return false };
        t.body.iter().any(|s| Self::within(table, mutation, Table::id_of(AnyNodeRef::from(s))))
    }

    /// `binding_uncertainty::refine`
    fn refine(&mut self, table: &Table<'_>, reference: u32, phase: u32, name: u32, state: ReadState) -> ReadState {
        let key = (self.binding_scope(table, reference, name), name);
        let Some(relevant) = self.mutations.get(&key) else { return state };
        let relevant = relevant.clone();
        let mut at = reference;
        let mut uncertain = false;
        loop {
            let repeated = self.repeats_reference(table, reference, at);
            let handler = self.k[at as usize] == K::ExceptHandler;
            if (repeated || handler)
                && !(state == ReadState::Unique && self.candidates.iter().all(|c| Self::within(table, c.binding, at)))
            {
                for m in &relevant {
                    if m.phase != phase {
                        continue;
                    }
                    let inside = if handler { Self::protected_body(table, m.at, at) } else { Self::within(table, m.at, at) };
                    if !inside {
                        continue;
                    }
                    uncertain = true;
                    if m.action == Action::Bind && !self.candidates.iter().any(|c| c.binding == m.at) {
                        self.candidates.push(Candidate { binding: m.at, scope: key.0 });
                    }
                }
            }
            if at == phase || at == 0 {
                break;
            }
            at = table.parent[at as usize];
        }
        if uncertain { ReadState::Uncertain } else { state }
    }

    /// `binding_observations::read_context`
    fn read_context(&self, table: &Table<'_>, mut at: u32, phase: u32, fallback: Context) -> Context {
        if fallback != Context::Value {
            return fallback;
        }
        loop {
            if self.annotation[at as usize] {
                return Context::Type;
            }
            if at == phase || at == 0 {
                return fallback;
            }
            at = table.parent[at as usize];
        }
    }

    fn read(&mut self, table: &Table<'_>, frame: u32, step: Step, context: Context) {
        let name = self.sym[step.at as usize];
        if name == NONE {
            return;
        }
        let context = self.read_context(table, step.at, step.phase, context);
        let state = self.flow_resolve(table, frame, step.at, name);
        let state = self.refine(table, step.at, step.phase, name, state);
        let unique = state == ReadState::Unique;
        self.reads.push(Read { at: step.at, name, scope: self.scope_of[step.at as usize], phase: step.phase, context, state });
        for c in &self.candidates {
            self.uses.push(Use { at: step.at, binding: c.binding, scope: c.scope, phase: step.phase, context, unique });
        }
    }

    fn bind(&mut self, table: &Table<'_>, frame: u32, step: Step) {
        let d = self.def_of[step.at as usize];
        if d == NONE {
            return;
        }
        let def = self.defs[d as usize];
        let key = (def.owner, def.name);
        if let Some(prior) = self.lookup(frame, key) {
            for j in prior.start..prior.end {
                let previous = self.arena[j as usize];
                if previous != step.at {
                    self.shadows.push(Shadow { binding: step.at, previous, scope: def.owner, previous_scope: def.owner, phase: step.phase, lexical: false });
                }
            }
        }
        let _ = self.flow_resolve(table, frame, step.at, def.name);
        for c in &self.candidates {
            if c.scope != def.owner {
                self.shadows.push(Shadow { binding: step.at, previous: c.binding, scope: def.owner, previous_scope: c.scope, phase: step.phase, lexical: true });
            }
        }
        match step.action {
            Action::Bind => self.assign(frame, key, step.at),
            Action::Annotation => self.annotate(frame, key, step.at),
            _ => {}
        }
    }

    /// the name and namespace a binding step mutates (`binding_uncertainty::prepare`, `binding_observations::observe`)
    fn mutated(&self, table: &Table<'_>, at: u32) -> Option<(u32, u32, Option<Def>)> {
        let d = self.def_of[at as usize];
        if d != NONE {
            let def = self.defs[d as usize];
            return Some((def.owner, def.name, Some(def)));
        }
        let name = self.sym[at as usize];
        if name == NONE {
            return None;
        }
        Some((self.binding_scope(table, at, name), name, None))
    }

    fn observe(&mut self, table: &Table<'_>, frame: u32, step: Step, context: Context, ordinal: u32) {
        let wildcard = step.action == Action::Bind && self.k[step.at as usize] == K::Alias && self.def_of[step.at as usize] != NONE
            && self.defs[self.def_of[step.at as usize] as usize].name == UNKNOWN;
        let kind = if wildcard {
            EventKind::Wildcard
        } else {
            match step.action {
                Action::Read => EventKind::Read,
                Action::Bind => EventKind::Bind,
                Action::Annotation => EventKind::Annotation,
                Action::Delete => EventKind::Delete,
                Action::Cleanup => EventKind::Cleanup,
                Action::Global => EventKind::Global,
                Action::Nonlocal => EventKind::Nonlocal,
                _ => return,
            }
        };
        if matches!(kind, EventKind::Global | EventKind::Nonlocal) {
            let end = table.end[step.at as usize];
            let mut c = step.at + 1;
            while c < end {
                if self.k[c as usize] == K::Identifier {
                    let name = self.sym[c as usize];
                    let scope = self.binding_scope(table, c, name);
                    let lexical = self.lexical_scope(table, c);
                    self.events.push(Event { at: c, name, scope, phase: step.phase, ordinal, kind, lexical });
                }
                c = table.end[c as usize];
            }
            return;
        }
        let d = self.def_of[step.at as usize];
        let def = (d != NONE).then(|| self.defs[d as usize]);
        let name = match def {
            Some(def) => def.name,
            None => self.sym[step.at as usize],
        };
        if name == NONE {
            return;
        }
        let scope = if kind == EventKind::Read {
            self.scope_of[step.at as usize]
        } else {
            def.map_or_else(|| self.binding_scope(table, step.at, name), |d| d.owner)
        };
        let lexical = if kind == EventKind::Read || matches!(self.k[step.at as usize], K::Parameter | K::TypeVar | K::TypeVarTuple | K::ParamSpec) {
            scope
        } else {
            self.lexical_scope(table, step.at)
        };
        self.events.push(Event { at: step.at, name, scope, phase: step.phase, ordinal, kind, lexical });
        match kind {
            EventKind::Wildcard => self.wildcard(frame, scope),
            EventKind::Read => self.read(table, frame, step, context),
            EventKind::Bind | EventKind::Annotation => self.bind(table, frame, step),
            EventKind::Delete => {
                self.read(table, frame, step, Context::Deletion);
                self.delete(frame, (scope, name));
            }
            EventKind::Cleanup => self.delete(frame, (scope, name)),
            EventKind::Global | EventKind::Nonlocal => {}
        }
    }

    /// `binding_analysis::build_within`
    pub(crate) fn build_flow(&mut self, table: &Table<'_>) {
        self.events.clear();
        self.reads.clear();
        self.uses.clear();
        self.shadows.clear();
        self.resolutions.clear();
        self.flow_incomplete = false;
        self.active = 0;
        self.slots.clear();
        self.arena.clear();
        self.phase_frames.clear();
        self.branches.clear();
        self.arms.clear();
        for (_, mut v) in self.mutations.drain() {
            v.clear();
            self.mutation_pool.push(v);
        }
        if self.incomplete {
            self.flow_incomplete = true;
            return;
        }
        self.schedule(table);
        // binding_uncertainty::prepare
        for i in 0..self.steps.len() {
            let s = self.steps[i];
            if !matches!(s.action, Action::Bind | Action::Delete | Action::Cleanup) {
                continue;
            }
            let Some((scope, name, _)) = self.mutated(table, s.at) else { continue };
            let pool = &mut self.mutation_pool;
            self.mutations.entry((scope, name)).or_insert_with(|| pool.pop().unwrap_or_default()).push(Mutation { at: s.at, phase: s.phase, action: s.action });
        }
        let mut frame = NONE;
        let mut context = Context::Value;
        for cursor in 0..self.steps.len() {
            let step = self.steps[cursor];
            match step.action {
                Action::BeginPhase => {
                    let parent = if step.at == 0 {
                        NONE
                    } else {
                        let task = self.phases.iter().find(|t| t.root == step.at).copied();
                        match task.and_then(|t| self.phase_frames.get(&t.parent).copied()) {
                            Some(f) => f,
                            None => {
                                self.flow_incomplete = true;
                                return;
                            }
                        }
                    };
                    context = Context::Value;
                    frame = self.open(parent);
                }
                Action::EndPhase => {
                    if !self.branches.is_empty() {
                        self.flow_incomplete = true;
                        return;
                    }
                    if frame != NONE {
                        self.phase_frames.insert(step.at, frame);
                        frame = NONE;
                    }
                }
                Action::BeginAlternatives => {
                    self.branches.push((frame, self.arms.len()));
                    frame = self.open(frame);
                }
                Action::NextAlternative => {
                    let (target, _) = *self.branches.last().expect("branch");
                    self.arms.push(frame);
                    frame = self.open(target);
                }
                Action::EndAlternatives => {
                    let (target, start) = self.branches.pop().expect("branch");
                    self.arms.push(frame);
                    self.join(target, start);
                    self.arms.truncate(start);
                    frame = target;
                }
                _ => self.observe(table, frame, step, context, cursor as u32),
            }
        }
    }

    /// `binding_resolution::resolve` (type parameters: none on Django; `lazy` bound/default reads likewise)
    pub(crate) fn build_resolution(&mut self, table: &Table<'_>) {
        self.resolutions.clear();
        self.latest.clear();
        if self.flow_incomplete {
            return;
        }
        for (i, e) in self.events.iter().enumerate() {
            if matches!(e.kind, EventKind::Global | EventKind::Nonlocal) && e.lexical != e.scope {
                self.latest.entry((e.lexical, e.name)).or_insert(i as u32);
            }
        }
        let mut cursor = 0;
        for i in 0..self.events.len() {
            let e = self.events[i];
            match e.kind {
                EventKind::Read | EventKind::Delete => {
                    let mut context = if e.kind == EventKind::Delete { Context::Deletion } else { Context::Value };
                    while let Some(r) = self.reads.get(cursor) {
                        cursor += 1;
                        if r.at == e.at {
                            context = r.context;
                            break;
                        }
                    }
                    self.reference(table, e, context);
                    if e.kind == EventKind::Delete {
                        self.latest.insert((e.scope, e.name), i as u32);
                    }
                }
                EventKind::Bind | EventKind::Annotation | EventKind::Cleanup => {
                    self.latest.insert((e.scope, e.name), i as u32);
                }
                EventKind::Global | EventKind::Nonlocal => {
                    self.latest.insert((e.scope, e.name), i as u32);
                    if e.lexical != e.scope {
                        self.latest.insert((e.lexical, e.name), i as u32);
                    }
                }
                EventKind::Wildcard => {}
            }
        }
    }

    fn reference(&mut self, _table: &Table<'_>, e: Event, context: Context) {
        let start = self.scope_of[e.at as usize];
        let mut scope = start;
        let mut first = true;
        let mut depth = 0;
        loop {
            let skipped = !first && self.k[scope as usize] == K::ClassDef;
            if !skipped {
                self.namespace(scope, e, depth, context);
                depth += 1;
                if scope != 0 && self.declares_global(scope, e.name) {
                    self.namespace(0, e, depth, context);
                    break;
                }
            }
            if scope == 0 {
                break;
            }
            scope = self.scope_of[scope as usize];
            first = false;
        }
    }

    fn namespace(&mut self, scope: u32, reference: Event, depth: u32, context: Context) {
        let Some(&at) = self.latest.get(&(scope, reference.name)) else { return };
        let found = self.events[at as usize];
        self.resolutions.push(Resolution {
            at: reference.at,
            event: found.at,
            kind: found.kind,
            scope,
            lexical: found.lexical,
            depth,
            ordinal: found.ordinal,
            context,
        });
    }
}

fn builtin_named(text: &str) -> Option<(i64, i64)> {
    BUILTIN_NAMES.contains(&text).then(|| ADDED_BUILTINS.iter().find(|a| a.0 == text).map_or((3, 0), |a| (a.1, a.2)))
}

mod smallvec_like {
    pub(crate) type Ids = Vec<usize>;
}
