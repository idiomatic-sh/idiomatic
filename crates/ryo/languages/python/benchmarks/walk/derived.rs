//! Bake-off round 3 (bead ryo-ger6, throwaway): the Python provider's remaining relations (neither a tree read nor one of the 11
//! semantic relations) computed from the parser's own nodes and the flat table, in the provider's terms. Each relation has a
//! per-candidate form (a function of one node: "the properties of THIS literal", "the arguments of THIS call") and a rows form that
//! runs the per-candidate form over its kind bucket. Each function names the provider reader it follows.

use crate::model::Model;
use crate::table::Table;
use crate::view::node_kind_name;
use ruff_python_ast::{self as ast, AnyNodeRef, Expr, NodeKind};
use ruff_text_size::Ranged as _;

/// One cell of a row as the export writes it (node cells as `kind@start:end`).
#[derive(Clone, Copy)]
pub(crate) enum Cell<'s> {
    Node(u32),
    Key(&'static str, u32, u32),
    Text(&'s str),
    Null,
    Num(i64),
    Flag(bool),
    Span(u32, u32),
    File,
}

pub(crate) trait Sink {
    fn row(&mut self, rel: usize, cells: &[Cell<'_>]);
}

/// Counts rows and touches every cell (what a consumer of materialised rows pays, minus formatting).
pub(crate) struct Count {
    pub counts: Vec<u64>,
    pub touch: u64,
}

impl Sink for Count {
    fn row(&mut self, rel: usize, cells: &[Cell<'_>]) {
        self.counts[rel] += 1;
        for c in cells {
            self.touch = self.touch.wrapping_add(match *c {
                Cell::Node(n) => u64::from(n),
                Cell::Key(_, a, b) => u64::from(a ^ b),
                Cell::Text(t) => t.len() as u64,
                Cell::Num(n) => n as u64,
                Cell::Flag(f) => u64::from(f),
                Cell::Span(a, b) => u64::from(a ^ b),
                Cell::Null | Cell::File => 1,
            });
        }
    }
}

/// Formats rows in the canonical text form compare.py reads.
pub(crate) struct Dump<'t, 'a> {
    pub table: &'t Table<'a>,
    pub file: usize,
    pub lines: Vec<Vec<String>>,
    pub counts: Vec<u64>,
}

pub(crate) fn esc_into(s: &mut String, t: &str) {
    for c in t.chars() {
        match c {
            '\\' => s.push_str("\\\\"),
            '\t' => s.push_str("\\t"),
            '\n' => s.push_str("\\n"),
            '\r' => s.push_str("\\r"),
            c => s.push(c),
        }
    }
}

impl Sink for Dump<'_, '_> {
    fn row(&mut self, rel: usize, cells: &[Cell<'_>]) {
        use std::fmt::Write as _;
        self.counts[rel] += 1;
        let mut s = format!("{}", self.file);
        for c in cells {
            s.push('\t');
            match *c {
                Cell::Node(n) => {
                    let i = n as usize;
                    let _ = write!(s, "{}@{}:{}", node_kind_name(self.table.node[i]), self.table.start[i], self.table.stop[i]);
                }
                Cell::Key(k, a, b) => {
                    let _ = write!(s, "{k}@{a}:{b}");
                }
                Cell::Text(t) => esc_into(&mut s, t),
                Cell::Null => s.push_str("\\N"),
                Cell::Num(n) => {
                    let _ = write!(s, "{n}");
                }
                Cell::Flag(f) => s.push(if f { '1' } else { '0' }),
                Cell::Span(a, b) => {
                    let _ = write!(s, "{a}:{b}");
                }
                Cell::File => {
                    let _ = write!(s, "{}", self.file);
                }
            }
        }
        self.lines[rel].push(s);
    }
}

pub(crate) const RELATIONS: &[&str] = &[
    "identifier",
    "literal-properties",
    "string-literal",
    "static-import",
    "numeric-literal",
    "call-argument",
    "language-target",
    "compiler-feature",
    "filesystem.file",
];

pub(crate) fn index_of(name: &str) -> Option<usize> {
    RELATIONS.iter().position(|r| *r == name)
}

/// The per-file inputs: the flat table of the parser's nodes, the source, the lexical model (for owners), parse completeness.
pub(crate) struct Ctx<'c, 'a> {
    pub table: &'c Table<'a>,
    pub source: &'c str,
    pub model: &'c Model,
    pub incomplete: bool,
}

impl<'a> Ctx<'_, 'a> {
    fn span(&self, id: u32) -> Cell<'static> {
        Cell::Span(self.table.start[id as usize], self.table.stop[id as usize])
    }
    fn written(&self, id: u32) -> &str {
        &self.source[self.table.start[id as usize] as usize..self.table.stop[id as usize] as usize]
    }
    fn kind(&self, k: NodeKind) -> &[u32] {
        self.table.of_kind(k as u8)
    }
    /// `scopes::owner`: the scope the node is evaluated in.
    fn owner(&self, id: u32) -> Cell<'static> {
        Cell::Node(self.model.scope_of[id as usize])
    }
}

fn id_of(node: AnyNodeRef<'_>) -> u32 {
    Table::id_of(node)
}

// ------------------------------------------------------------------------------------------------ identifier
/// `identifier/item.rs` over `syntax::identifiers`: every name expression and every identifier field holding a NAME token, with its
/// parser-normalised spelling; nothing in a file with a syntax error.
pub(crate) fn identifiers_of(node: AnyNodeRef<'_>, out: &mut dyn FnMut(&'static str, &ast::Identifier)) {
    use AnyNodeRef as N;
    let mut id = |i: &ast::Identifier| {
        if !i.id.is_empty() && i.id.as_str() != "*" {
            out("identifier", i)
        }
    };
    match node {
        N::StmtFunctionDef(f) => id(&f.name),
        N::StmtClassDef(c) => id(&c.name),
        N::StmtImportFrom(i) => {
            if let Some(m) = &i.module {
                id(m)
            }
        }
        N::Alias(a) => {
            id(&a.name);
            if let Some(n) = &a.asname {
                id(n)
            }
        }
        N::ExprAttribute(a) => id(&a.attr),
        N::Keyword(k) => {
            if let Some(a) = &k.arg {
                id(a)
            }
        }
        N::Parameter(p) => id(&p.name),
        N::ExceptHandlerExceptHandler(h) => {
            if let Some(n) = &h.name {
                id(n)
            }
        }
        N::StmtGlobal(g) => g.names.iter().for_each(&mut id),
        N::StmtNonlocal(g) => g.names.iter().for_each(&mut id),
        N::PatternMatchAs(p) => {
            if let Some(n) = &p.name {
                id(n)
            }
        }
        N::PatternMatchStar(p) => {
            if let Some(n) = &p.name {
                id(n)
            }
        }
        N::PatternMatchMapping(p) => {
            if let Some(n) = &p.rest {
                id(n)
            }
        }
        N::PatternKeyword(p) => id(&p.attr),
        N::TypeParamTypeVar(p) => id(&p.name),
        N::TypeParamTypeVarTuple(p) => id(&p.name),
        N::TypeParamParamSpec(p) => id(&p.name),
        _ => {}
    }
}

fn rows_identifier(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize) {
    if c.incomplete {
        return;
    }
    for id in 0..c.table.len() as u32 {
        let node = c.table.node[id as usize];
        if let AnyNodeRef::ExprName(n) = node {
            sink.row(rel, &[Cell::Node(id), Cell::Text(n.id.as_str()), Cell::File, c.span(id)]);
            continue;
        }
        identifiers_of(node, &mut |kind, i| {
            let (s, e) = (i.range.start().to_u32(), i.range.end().to_u32());
            sink.row(rel, &[Cell::Key(kind, s, e), Cell::Text(i.id.as_str()), Cell::File, Cell::Span(s, e)]);
        });
    }
}

// ------------------------------------------------------------------------------------------------ literal-properties
/// `literal_properties::classify`: kind, zero, empty, boolean of one primitive literal, from the parser's own payload.
#[derive(Clone, Copy)]
pub(crate) struct LiteralProps {
    pub kind: &'static str,
    pub zero: bool,
    pub empty: bool,
    pub boolean: bool,
}

pub(crate) fn literal_properties(node: AnyNodeRef<'_>) -> Option<LiteralProps> {
    let p = |kind, zero, empty, boolean| Some(LiteralProps { kind, zero, empty, boolean });
    match node {
        AnyNodeRef::ExprStringLiteral(s) => p("string", false, s.value.iter().all(|part| part.value.is_empty()), false),
        AnyNodeRef::ExprBytesLiteral(b) => p("bytes", false, b.value.iter().all(|part| part.value.is_empty()), false),
        AnyNodeRef::ExprNumberLiteral(n) => match &n.value {
            ast::Number::Int(i) => p("integer", i.as_u64() == Some(0), false, false),
            ast::Number::Float(f) => p("float", *f == 0.0, false, false),
            ast::Number::Complex { real, imag } => p("complex", *real == 0.0 && *imag == 0.0, false, false),
        },
        AnyNodeRef::ExprBooleanLiteral(b) => p("boolean", false, false, b.value),
        AnyNodeRef::ExprNoneLiteral(_) => p("null", false, false, false),
        AnyNodeRef::ExprEllipsisLiteral(_) => p("ellipsis", false, false, false),
        _ => None,
    }
}

const LITERAL_KINDS: [NodeKind; 6] = [
    NodeKind::ExprStringLiteral,
    NodeKind::ExprBytesLiteral,
    NodeKind::ExprNumberLiteral,
    NodeKind::ExprBooleanLiteral,
    NodeKind::ExprNoneLiteral,
    NodeKind::ExprEllipsisLiteral,
];

fn rows_literal_properties(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize) {
    for k in LITERAL_KINDS {
        for &id in c.kind(k) {
            if let Some(p) = literal_properties(c.table.node[id as usize]) {
                sink.row(rel, &[Cell::Node(id), Cell::Text(p.kind), Cell::Flag(p.zero), Cell::Flag(p.empty), Cell::Flag(p.boolean), Cell::File, c.span(id)]);
            }
        }
    }
}

// ------------------------------------------------------------------------------------------------ string-literal
/// `names::string_value`: the decoded parts joined; None (the unknown value) when the f-string or t-string interpolates.
pub(crate) fn string_value<'n>(node: AnyNodeRef<'n>, buf: &'n mut String) -> Option<&'n str> {
    match node {
        AnyNodeRef::ExprStringLiteral(s) => {
            if s.value.is_implicit_concatenated() {
                buf.clear();
                for part in s.value.iter() {
                    buf.push_str(&part.value);
                }
                Some(buf.as_str())
            } else {
                Some(s.value.to_str())
            }
        }
        AnyNodeRef::ExprFString(f) => {
            buf.clear();
            for part in f.value.iter() {
                match part {
                    ast::FStringPartRef::Literal(l) => buf.push_str(&l.value),
                    ast::FStringPartRef::FString(fs) => {
                        for e in fs.elements.iter() {
                            match e {
                                ast::InterpolatedStringElement::Literal(l) => buf.push_str(&l.value),
                                ast::InterpolatedStringElement::Interpolation(_) => return None,
                            }
                        }
                    }
                }
            }
            Some(buf.as_str())
        }
        AnyNodeRef::ExprTString(t) => {
            buf.clear();
            for ts in t.value.iter() {
                for e in ts.elements.iter() {
                    match e {
                        ast::InterpolatedStringElement::Literal(l) => buf.push_str(&l.value),
                        ast::InterpolatedStringElement::Interpolation(_) => return None,
                    }
                }
            }
            Some(buf.as_str())
        }
        _ => None,
    }
}

/// `fact_notes::specifier`: the first argument of `__import__(..)` or `importlib.import_module(..)` as written.
fn specifier(c: &Ctx<'_, '_>, id: u32) -> bool {
    let args = c.table.parent[id as usize];
    if args == u32::MAX {
        return false;
    }
    let AnyNodeRef::Arguments(a) = c.table.node[args as usize] else { return false };
    let call = c.table.parent[args as usize];
    let AnyNodeRef::ExprCall(call) = c.table.node[call as usize] else { return false };
    let first = a.iter_source_order().next().map(|x| match x {
        ast::ArgOrKeyword::Arg(e) => id_of(e.into()),
        ast::ArgOrKeyword::Keyword(k) => id_of(k.into()),
    });
    first == Some(id) && matches!(c.written(id_of((&*call.func).into())), "__import__" | "importlib.import_module")
}

const STRING_KINDS: [NodeKind; 3] = [NodeKind::ExprStringLiteral, NodeKind::ExprFString, NodeKind::ExprTString];

fn rows_string_literal(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, buf: &mut String) {
    for k in STRING_KINDS {
        for &id in c.kind(k) {
            let node = c.table.node[id as usize];
            let value = match string_value(node, buf) {
                Some(v) => Cell::Text(v),
                None => Cell::Null, // the unknown value of an interpolated string: a symbol with no text
            };
            sink.row(rel, &[Cell::Node(id), c.owner(id), value, Cell::File, c.span(id), Cell::Flag(specifier(c, id))]);
        }
    }
}

// ------------------------------------------------------------------------------------------------ static-import
/// `imports::import_binding` / `import_target` of one alias: the name it binds and the dotted path it imports.
/// Returns the buffer and the split between binding and target; a wildcard's binding is the unknown value (no text, `split == 0`
/// with `wildcard`).
pub(crate) fn static_import<'n>(c: &Ctx<'_, '_>, id: u32, buf: &'n mut String) -> Option<(&'n str, usize, bool)> {
    let AnyNodeRef::Alias(alias) = c.table.node[id as usize] else { return None };
    let parent = c.table.parent[id as usize] as usize;
    buf.clear();
    let wildcard = alias.name.as_str() == "*";
    let from = if let AnyNodeRef::StmtImportFrom(f) = c.table.node[parent] { Some(f) } else { None };
    // binding first, then target, in one buffer: returns (buffer, split)
    if wildcard {
    } else if let Some(asname) = &alias.asname {
        buf.push_str(asname.as_str());
    } else if from.is_some() {
        buf.push_str(alias.name.as_str());
    } else {
        buf.push_str(alias.name.as_str().split('.').next().unwrap_or(""));
    }
    let split = buf.len();
    if let Some(f) = from {
        for _ in 0..f.level {
            buf.push('.');
        }
        if let Some(m) = &f.module {
            buf.push_str(m.as_str());
            if !wildcard {
                buf.push('.');
            }
        }
    }
    if !wildcard {
        buf.push_str(alias.name.as_str());
    }
    Some((buf.as_str(), split, wildcard))
}

fn rows_static_import(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, buf: &mut String) {
    for &id in c.kind(NodeKind::Alias) {
        if let Some((text, split, wildcard)) = static_import(c, id, buf) {
            let binding = if wildcard { Cell::Null } else { Cell::Text(&text[..split]) };
            sink.row(rel, &[Cell::Node(id), c.owner(id), binding, Cell::Text(&text[split..]), Cell::File, c.span(id)]);
        }
    }
}

// ------------------------------------------------------------------------------------------------ numeric-literal
/// `numeric_literals::classify` + `intern_value` + `exactness`: an integer's decimal value, a float's binary64 text and whether the
/// written digits survive binary64; complex numbers are not numeric literals here.
pub(crate) fn numeric_literal<'n>(c: &Ctx<'_, '_>, id: u32, buf: &'n mut String) -> Option<(&'static str, &'n str, bool)> {
    use std::fmt::Write as _;
    let AnyNodeRef::ExprNumberLiteral(n) = c.table.node[id as usize] else { return None };
    buf.clear();
    match &n.value {
        ast::Number::Int(i) => {
            let _ = write!(buf, "{i}");
            let digits = buf.as_str();
            let canonical = !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) && (digits.len() == 1 || !digits.starts_with('0'));
            if !canonical {
                return None;
            }
            Some(("integer", buf.as_str(), true))
        }
        ast::Number::Float(f) => {
            let bits = ryo::languages::FloatBits(f.to_bits());
            let mut fb = ryo::languages::FloatBuffer([0; ryo::languages::FLOAT_TEXT_BYTES]);
            buf.push_str(ryo::languages::binary64_text(bits, &mut fb).0);
            let mut digits = String::new();
            for b in c.written(id).bytes().take_while(|b| !matches!(b, b'e' | b'E')).filter(|b| *b != b'_').skip_while(|b| *b == b'0') {
                digits.push(b as char);
            }
            let exact = digits.len() <= ryo::languages::WRITTEN_DIGITS_BYTES
                && ryo::languages::exact_digits(ryo::languages::NameText(&digits), bits) == ryo::languages::Flag::Raised;
            Some(("float", buf.as_str(), exact))
        }
        ast::Number::Complex { .. } => None,
    }
}

fn rows_numeric_literal(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, buf: &mut String) {
    for &id in c.kind(NodeKind::ExprNumberLiteral) {
        if let Some((kind, value, exact)) = numeric_literal(c, id, buf) {
            sink.row(rel, &[Cell::Node(id), Cell::Text(kind), Cell::Text(value), Cell::Flag(exact), Cell::File, c.span(id)]);
        }
    }
}

// ------------------------------------------------------------------------------------------------ call-argument
/// `names::arguments_of` + `passed`: the arguments of one call in written order, a keyword as its value; the one name an argument
/// hands on (behind any stars).
pub(crate) fn call_arguments<'n>(c: &Ctx<'_, 'n>, call: u32, out: &mut dyn FnMut(u32, u32, Option<&'n str>)) {
    let AnyNodeRef::ExprCall(call_node) = c.table.node[call as usize] else { return };
    for (position, a) in call_node.arguments.iter_source_order().enumerate() {
        let e: &'n Expr = match a {
            ast::ArgOrKeyword::Arg(e) => e,
            ast::ArgOrKeyword::Keyword(k) => &k.value,
        };
        let mut cur = e;
        while let Expr::Starred(s) = cur {
            cur = &s.value;
        }
        let passes = if let Expr::Name(n) = cur { Some(n.id.as_str()) } else { None };
        out(position as u32, id_of(e.into()), passes);
    }
}

fn rows_call_argument(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize) {
    for &id in c.kind(NodeKind::ExprCall) {
        call_arguments(c, id, &mut |position, e, passes| {
            sink.row(rel, &[Cell::Node(id), Cell::Num(i64::from(position)), Cell::Node(e), passes.map_or(Cell::Null, Cell::Text), Cell::File, c.span(e)]);
        });
    }
}

// ------------------------------------------------------------------------------------------------ language-target, compiler-feature, filesystem.file
/// `language_target`: the configured target version (the request's; one row per file).
pub(crate) const TARGET: (i64, i64) = (3, 10);

/// `future::module_future_annotations`: `from __future__ import annotations` among the module's leading future imports, with every
/// future import naming a known feature and standing where one may.
pub(crate) fn future_annotations(c: &Ctx<'_, '_>) -> Option<bool> {
    const FEATURES: [&str; 10] = [
        "nested_scopes", "generators", "division", "absolute_import", "with_statement", "print_function", "unicode_literals",
        "barry_as_FLUFL", "generator_stop", "annotations",
    ];
    let AnyNodeRef::ModModule(m) = c.table.node[0] else { return None };
    let mut permitted = Vec::new();
    for (i, s) in m.body.iter().enumerate() {
        if i == 0 && matches!(s, ast::Stmt::Expr(e) if e.value.is_string_literal_expr()) {
            continue;
        }
        match s {
            ast::Stmt::ImportFrom(f) if f.level == 0 && f.module.as_deref() == Some("__future__") => permitted.push(f.range),
            _ => break,
        }
    }
    let (mut enabled, mut valid) = (false, true);
    for &id in c.kind(NodeKind::StmtImportFrom) {
        let AnyNodeRef::StmtImportFrom(f) = c.table.node[id as usize] else { continue };
        if f.level != 0 || f.module.as_deref() != Some("__future__") {
            continue;
        }
        let ok = permitted.contains(&f.range);
        if !ok || f.is_lazy {
            valid = false;
        }
        let known = f.names.iter().all(|a| FEATURES.contains(&a.name.as_str()));
        valid &= known;
        enabled |= ok && !f.is_lazy && known && f.names.iter().any(|a| a.name.as_str() == "annotations");
    }
    Some(enabled && valid)
}

// ------------------------------------------------------------------------------------------------ driver
pub(crate) fn produce(c: &Ctx<'_, '_>, wanted: u64, sink: &mut dyn Sink, buf: &mut String) {
    let want = |name: &str| index_of(name).filter(|i| wanted & (1 << i) != 0);
    if let Some(r) = want("identifier") {
        rows_identifier(c, sink, r);
    }
    if let Some(r) = want("literal-properties") {
        rows_literal_properties(c, sink, r);
    }
    if let Some(r) = want("string-literal") {
        rows_string_literal(c, sink, r, buf);
    }
    if let Some(r) = want("static-import") {
        rows_static_import(c, sink, r, buf);
    }
    if let Some(r) = want("numeric-literal") {
        rows_numeric_literal(c, sink, r, buf);
    }
    if let Some(r) = want("call-argument") {
        rows_call_argument(c, sink, r);
    }
    if let Some(r) = want("language-target") {
        sink.row(r, &[Cell::File, Cell::Num(TARGET.0), Cell::Num(TARGET.1)]);
    }
    if let Some(r) = want("compiler-feature") {
        if let Some(enabled) = future_annotations(c) {
            sink.row(r, &[Cell::File, Cell::Text("python.future-annotations"), Cell::Flag(enabled)]);
        }
    }
    if let Some(r) = want("filesystem.file") {
        sink.row(r, &[Cell::File]);
    }
}
