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
    "embedded-expression",
    "function",
    "parameter",
    "decorates",
    "inherits",
    "bytes-literal",
    "interpolated-string-literal",
    "filesystem.executable",
    "expression-shape",
    "expression-type",
];

pub(crate) fn index_of(name: &str) -> Option<usize> {
    RELATIONS.iter().position(|r| *r == name)
}

/// The per-file inputs: the flat table of the parser's nodes, the source, the lexical model (for owners), parse completeness.
pub(crate) struct Ctx<'c, 'a> {
    pub path: &'c std::path::Path,
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

// ------------------------------------------------------------------------------------------------ embedded-expression
/// `adapter::append_literal` over the writer's annotation candidates: a string literal in annotation position parsed as a type
/// expression; a simple one maps exactly into the host's content, a complex one (escapes, implicit concatenation) maps to the whole
/// host. Not done: the provider's `visibility` (a candidate outside a type context, e.g. inside `Literal[..]`, is hidden), and the
/// relations of the nodes inside an embedded expression (Django has no string annotation; 0 rows in the export).
pub(crate) fn embedded_expression(c: &Ctx<'_, '_>, id: u32) -> Option<(&'static str, u32, u32, bool)> {
    let AnyNodeRef::ExprStringLiteral(literal) = c.table.node[id as usize] else { return None };
    let mut up = id;
    let mut annotated = false;
    while up != u32::MAX {
        if c.model.annotation[up as usize] {
            annotated = true;
            break;
        }
        up = c.table.parent[up as usize];
    }
    if !annotated {
        return None;
    }
    let parsed = ruff_python_parser::typing::parse_type_annotation(literal, c.source).ok()?;
    let root = parsed.expression();
    let kind = node_kind_name(AnyNodeRef::from(root));
    if parsed.kind().is_simple() {
        Some((kind, root.start().to_u32(), root.end().to_u32(), true))
    } else {
        let len = literal.value.to_str().len() as u32;
        Some((kind, 0, len, false))
    }
}

fn rows_embedded_expression(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize) {
    if !c.model.annotation.iter().any(|a| *a) {
        return;
    }
    for &id in c.kind(NodeKind::ExprStringLiteral) {
        if let Some((kind, s, e, exact)) = embedded_expression(c, id) {
            sink.row(rel, &[Cell::Key(kind, s, e), Cell::Node(id), Cell::Text("deferred-type-expression"), Cell::Text(if exact { "exact-content" } else { "whole-host" }), Cell::File, Cell::Span(s, e)]);
        }
    }
}

// ------------------------------------------------------------------------------------------------ function, parameter
fn rows_function(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize) {
    for &id in c.kind(NodeKind::StmtFunctionDef) {
        let AnyNodeRef::StmtFunctionDef(f) = c.table.node[id as usize] else { continue };
        sink.row(rel, &[Cell::Node(id), Cell::Text(f.name.as_str()), Cell::File, c.span(id)]);
    }
}

/// `definitions::placed`: how one parameter takes its argument, its place among the callable's parameters, and the callable.
pub(crate) fn parameter_place(c: &Ctx<'_, '_>, id: u32) -> Option<(&'static str, &'static str, i64, u32)> {
    let mut holder = c.table.parent[id as usize];
    if matches!(c.table.node[holder as usize], AnyNodeRef::ParameterWithDefault(_)) {
        holder = c.table.parent[holder as usize];
    }
    let AnyNodeRef::Parameters(ps) = c.table.node[holder as usize] else { return None };
    let target = c.table.node[id as usize];
    let mut ordinal = 0i64;
    let callable = c.table.parent[holder as usize];
    let same = |p: &ast::Parameter| id_of(p.into()) == id;
    for p in &ps.posonlyargs {
        if same(&p.parameter) {
            return Some(("positional", "positional-only", ordinal, callable));
        }
        ordinal += 1;
    }
    for p in &ps.args {
        if same(&p.parameter) {
            return Some(("positional-or-keyword", "positional-or-keyword", ordinal, callable));
        }
        ordinal += 1;
    }
    if let Some(v) = &ps.vararg {
        if same(v) {
            return Some(("variadic-positional", "var-positional", ordinal, callable));
        }
        ordinal += 1;
    }
    for p in &ps.kwonlyargs {
        if same(&p.parameter) {
            return Some(("keyword-only", "keyword-only", ordinal, callable));
        }
        ordinal += 1;
    }
    if let Some(v) = &ps.kwarg {
        if same(v) {
            return Some(("variadic-keyword", "var-keyword", ordinal, callable));
        }
    }
    let _ = target;
    None
}

fn rows_parameter(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize) {
    for &id in c.kind(NodeKind::Parameter) {
        let AnyNodeRef::Parameter(p) = c.table.node[id as usize] else { continue };
        if let Some((kind, word, ordinal, callable)) = parameter_place(c, id) {
            sink.row(rel, &[Cell::Node(id), Cell::Text(kind), Cell::Text(word), Cell::Text(p.name.as_str()), Cell::Num(ordinal), Cell::Node(callable), Cell::File, c.span(id)]);
        }
    }
}

// ------------------------------------------------------------------------------------------------ decorates, inherits
/// `names::dotted`: the dotted name a name or attribute chain writes.
pub(crate) fn dotted_into(e: &Expr, out: &mut String) -> bool {
    match e {
        Expr::Name(n) => {
            out.push_str(n.id.as_str());
            true
        }
        Expr::Attribute(a) => {
            if !dotted_into(&a.value, out) {
                return false;
            }
            out.push('.');
            out.push_str(a.attr.as_str());
            true
        }
        _ => false,
    }
}

/// `names::decorator_target` + `decorator_arguments`: the dotted name a decorator writes or calls, and the text between its call's
/// parentheses (empty when it calls nothing). Returns (target known, split) over `buf` = target then arguments.
pub(crate) fn decorator<'n>(c: &Ctx<'_, '_>, id: u32, buf: &'n mut String) -> Option<(bool, usize)> {
    let AnyNodeRef::Decorator(d) = c.table.node[id as usize] else { return None };
    buf.clear();
    let named = if let Expr::Call(call) = &d.expression { &*call.func } else { &d.expression };
    let known = dotted_into(named, buf);
    if !known {
        buf.clear();
    }
    let split = buf.len();
    if let Expr::Call(call) = &d.expression {
        // the call's argument list spans `(` .. `)`
        let (s, e) = (call.arguments.range.start().to_usize(), call.arguments.range.end().to_usize());
        if e >= s + 2 {
            buf.push_str(&c.source[s + 1..e - 1]);
        }
    }
    Some((known, split))
}

fn rows_decorates(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, buf: &mut String) {
    for &id in c.kind(NodeKind::Decorator) {
        let parent = c.table.parent[id as usize];
        let ordinal = match c.table.node[parent as usize] {
            AnyNodeRef::StmtFunctionDef(f) => f.decorator_list.iter().position(|d| id_of(d.into()) == id),
            AnyNodeRef::StmtClassDef(k) => k.decorator_list.iter().position(|d| id_of(d.into()) == id),
            _ => None,
        }
        .unwrap_or(0);
        if let Some((known, split)) = decorator(c, id, buf) {
            let target = if known { Cell::Text(&buf[..split]) } else { Cell::Null };
            sink.row(rel, &[Cell::Node(id), Cell::Node(parent), Cell::Num(ordinal as i64), target, Cell::Text(&buf[split..]), Cell::File, c.span(id)]);
        }
    }
}

/// `names::base_target`: what a class base names (a subscript names its value, unless indexed by a number; a star is dynamic).
pub(crate) fn base_target(e: &Expr, buf: &mut String) -> bool {
    buf.clear();
    let ok = match e {
        Expr::Starred(_) => false,
        Expr::Subscript(s) if matches!(&*s.slice, Expr::NumberLiteral(_)) => false,
        Expr::Subscript(s) => dotted_into(&s.value, buf),
        _ => dotted_into(e, buf),
    };
    if !ok {
        buf.clear();
    }
    ok
}

fn rows_inherits(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, buf: &mut String) {
    for &id in c.kind(NodeKind::StmtClassDef) {
        let AnyNodeRef::StmtClassDef(k) = c.table.node[id as usize] else { continue };
        let Some(args) = &k.arguments else { continue };
        for base in args.args.iter() {
            let b = id_of(base.into());
            let target = if base_target(base, buf) { Cell::Text(buf.as_str()) } else { Cell::Null };
            sink.row(rel, &[Cell::Node(b), Cell::Node(id), target, Cell::File, c.span(b)]);
        }
    }
}

// ------------------------------------------------------------------------------------------------ bytes-literal, interpolated-string-literal
fn rows_bytes_literal(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, buf: &mut String) {
    use std::fmt::Write as _;
    for &id in c.kind(NodeKind::ExprBytesLiteral) {
        let AnyNodeRef::ExprBytesLiteral(b) = c.table.node[id as usize] else { continue };
        buf.clear();
        for byte in b.value.bytes() {
            let _ = write!(buf, "{byte:02x}");
        }
        sink.row(rel, &[Cell::Node(id), c.owner(id), Cell::Text(buf.as_str()), Cell::File, c.span(id)]);
    }
}

/// The literal fragments of one f-string (not of a format spec): its plain string parts and the literal elements of its f-string
/// parts, each with its decoded text.
pub(crate) fn fstring_fragments<'n>(node: AnyNodeRef<'n>, out: &mut dyn FnMut(&'static str, u32, u32, &'n str)) {
    let AnyNodeRef::ExprFString(f) = node else { return };
    for part in f.value.iter() {
        match part {
            ast::FStringPartRef::Literal(l) => out("string-part", l.range.start().to_u32(), l.range.end().to_u32(), &l.value),
            ast::FStringPartRef::FString(fs) => {
                for e in fs.elements.iter() {
                    if let ast::InterpolatedStringElement::Literal(l) = e {
                        out("constant", l.range.start().to_u32(), l.range.end().to_u32(), &l.value);
                    }
                }
            }
        }
    }
}

fn rows_interpolated_string_literal(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize) {
    for &id in c.kind(NodeKind::ExprFString) {
        fstring_fragments(c.table.node[id as usize], &mut |k, s, e, v| {
            sink.row(rel, &[Cell::Key(k, s, e), Cell::Node(id), Cell::Text(v), Cell::File, Cell::Span(s, e)]);
        });
    }
}

// ------------------------------------------------------------------------------------------------ expression-shape
/// `expression_shapes::prepare` (`languages::prepare_shapes`): bottom-up hash-consing of every expression by its structure: kind,
/// the parser's scalar payload, a name's spelling, a string's decoded value (an f-string's adjacent literal pieces merged), operator
/// and keyword tokens, and its children's classes by field; parentheses, delimiters and trivia are not structure. Built over the
/// table in descending pre-order; a class is named by its highest-positioned member. Nothing in a file with a syntax error.
#[derive(Default)]
pub(crate) struct Shapes {
    /// class (representative id) of each node; u32::MAX when it has none
    pub class: Vec<u32>,
    map: crate::sema::FxMap<Box<[u64]>, u32>,
    words: Vec<u64>,
}

fn h(text: &str) -> u64 {
    crate::sema::hash_text(text)
}

const W_KIND: u64 = 1 << 56;
const W_LIT: u64 = 2 << 56;
const W_PROP: u64 = 3 << 56;
const W_TOK: u64 = 4 << 56;
const W_IDENT: u64 = 5 << 56;

pub(crate) fn is_expression(node: AnyNodeRef<'_>) -> bool {
    node.as_expr_ref().is_some()
        || matches!(
            node,
            AnyNodeRef::InterpolatedElement(_) | AnyNodeRef::InterpolatedStringLiteralElement(_) | AnyNodeRef::InterpolatedStringFormatSpec(_)
        )
}

impl Shapes {
    fn word(&mut self, tag: u64, v: u64) {
        self.words.push(tag ^ (v & ((1 << 56) - 1)));
        self.words.push(v);
    }
    fn child(&mut self, field: &str, node: AnyNodeRef<'_>) {
        let i = id_of(node);
        if i == u32::MAX {
            panic!("child not in table: {:?}", node.kind());
        }
        let c = self.class[i as usize];
        self.words.push(h(field).rotate_left(7) ^ 0x6368_696c_64);
        self.words.push(u64::from(c));
    }
    fn tok(&mut self, t: &str) {
        self.word(W_TOK, h(t));
    }
    fn lit(&mut self, v: &str) {
        self.word(W_LIT, h(v));
    }
    fn prop(&mut self, name: &str, v: &str) {
        self.word(W_PROP, h(name) ^ h(v).rotate_left(29));
    }
    fn ident(&mut self, field: &str, i: &ast::Identifier) {
        self.words.push(h(field).rotate_left(7) ^ 0x6964);
        self.word(W_IDENT, h(i.id.as_str()));
    }

    /// The literal pieces and interpolations of an f-string, t-string or format spec, adjacent literal pieces merged.
    fn segments<'n>(&mut self, elements: &'n ast::InterpolatedStringElements, pending: &mut Option<String>) {
        for e in elements.iter() {
            match e {
                ast::InterpolatedStringElement::Literal(l) => pending.get_or_insert_with(String::new).push_str(&l.value),
                ast::InterpolatedStringElement::Interpolation(i) => {
                    if let Some(t) = pending.take() {
                        self.lit(&t);
                    }
                    self.child("values", AnyNodeRef::from(i));
                }
            }
        }
    }

    fn key(&mut self, node: AnyNodeRef<'_>) {
        use AnyNodeRef as N;
        self.words.clear();
        self.word(W_KIND, h(node_kind_name(node)));
        match node {
            N::ExprName(n) => self.word(W_IDENT, h(n.id.as_str())),
            N::ExprStringLiteral(s) => {
                let mut v = String::new();
                for p in s.value.iter() {
                    v.push_str(&p.value);
                }
                self.lit(&v);
            }
            N::InterpolatedStringLiteralElement(l) => self.lit(&l.value),
            N::ExprBytesLiteral(b) => {
                use std::fmt::Write as _;
                let mut v = String::new();
                for byte in b.value.bytes() {
                    let _ = write!(v, "{byte:02x}");
                }
                self.prop("kind", "bytes");
                self.prop("value", &v);
            }
            N::ExprNumberLiteral(n) => match &n.value {
                ast::Number::Int(i) => {
                    self.prop("kind", "integer");
                    self.prop("value", &i.to_string());
                }
                ast::Number::Float(f) => {
                    self.prop("kind", "float");
                    self.prop("value", &f.to_bits().to_string());
                }
                ast::Number::Complex { real, imag } => {
                    self.prop("kind", "complex");
                    self.prop("value", &real.to_bits().to_string());
                    self.prop("imaginary", &imag.to_bits().to_string());
                }
            },
            N::ExprBooleanLiteral(b) => {
                self.prop("kind", "boolean");
                self.prop("value", if b.value { "true" } else { "false" });
            }
            N::ExprNoneLiteral(_) => self.prop("kind", "none"),
            N::ExprEllipsisLiteral(_) => self.prop("kind", "ellipsis"),
            N::ExprFString(f) => {
                let mut pending = None;
                for part in f.value.iter() {
                    match part {
                        ast::FStringPartRef::Literal(l) => pending.get_or_insert_with(String::new).push_str(&l.value),
                        ast::FStringPartRef::FString(fs) => self.segments(&fs.elements, &mut pending),
                    }
                }
                if let Some(t) = pending.take() {
                    self.lit(&t);
                }
            }
            N::ExprTString(t) => {
                let mut pending = None;
                for ts in t.value.iter() {
                    self.segments(&ts.elements, &mut pending);
                }
                if let Some(t) = pending.take() {
                    self.lit(&t);
                }
            }
            N::InterpolatedStringFormatSpec(spec) => {
                let mut pending = None;
                self.segments(&spec.elements, &mut pending);
                if let Some(t) = pending.take() {
                    self.lit(&t);
                }
            }
            N::InterpolatedElement(i) => {
                if let Some(d) = &i.debug_text {
                    self.prop("debug", &d.as_str().replace("\r\n", "\n").replace('\r', "\n"));
                }
                if let Some(c) = i.conversion.to_char() {
                    self.prop("conversion", &c.to_string());
                }
                self.child("value", (&*i.expression).into());
                if let Some(spec) = &i.format_spec {
                    self.child("format-spec", (&**spec).into());
                }
            }
            N::ExprAttribute(a) => {
                self.child("value", (&*a.value).into());
                self.ident("attr", &a.attr);
            }
            N::ExprCall(c) => {
                self.child("func", (&*c.func).into());
                self.child("arguments", (&c.arguments).into());
            }
            N::Arguments(a) => {
                for x in a.args.iter() {
                    self.child("args", x.into());
                }
                for k in a.keywords.iter() {
                    self.child("keywords", k.into());
                }
            }
            N::Keyword(k) => {
                match &k.arg {
                    Some(a) => self.ident("arg", a),
                    None => self.tok("**"),
                }
                self.child("value", (&k.value).into());
            }
            N::ExprBoolOp(b) => {
                let op = if matches!(b.op, ast::BoolOp::And) { "and" } else { "or" };
                for (i, v) in b.values.iter().enumerate() {
                    if i > 0 {
                        self.tok(op);
                    }
                    self.child("values", v.into());
                }
            }
            N::ExprBinOp(b) => {
                self.child("left", (&*b.left).into());
                self.tok(b.op.as_str());
                self.child("right", (&*b.right).into());
            }
            N::ExprUnaryOp(u) => {
                self.tok(u.op.as_str());
                self.child("operand", (&*u.operand).into());
            }
            N::ExprCompare(c) => {
                for (i, o) in c.operands.iter().enumerate() {
                    if i > 0 {
                        self.tok(c.ops[i - 1].as_str());
                    }
                    self.child(if i == 0 { "left" } else { "comparators" }, o.into());
                }
            }
            N::ExprDict(d) => {
                for item in d.iter() {
                    match &item.key {
                        Some(k) => self.child("keys", k.into()),
                        None => self.tok("**"),
                    }
                    self.child("values", (&item.value).into());
                }
            }
            N::ExprSet(x) => x.elts.iter().for_each(|e| self.child("elts", e.into())),
            N::ExprList(x) => x.elts.iter().for_each(|e| self.child("elts", e.into())),
            N::ExprTuple(x) => x.elts.iter().for_each(|e| self.child("elts", e.into())),
            N::ExprStarred(x) => {
                self.tok("*");
                self.child("value", (&*x.value).into());
            }
            N::ExprSubscript(x) => {
                self.child("value", (&*x.value).into());
                self.child("slice", (&*x.slice).into());
            }
            N::ExprSlice(x) => {
                for (f, e) in [("lower", &x.lower), ("upper", &x.upper), ("step", &x.step)] {
                    if let Some(e) = e {
                        self.child(f, (&**e).into());
                    }
                }
            }
            N::ExprIf(x) => {
                self.child("body", (&*x.body).into());
                self.tok("if");
                self.child("test", (&*x.test).into());
                self.tok("else");
                self.child("orelse", (&*x.orelse).into());
            }
            N::ExprNamed(x) => {
                self.child("target", (&*x.target).into());
                self.child("value", (&*x.value).into());
            }
            N::ExprAwait(x) => {
                self.tok("await");
                self.child("value", (&*x.value).into());
            }
            N::ExprYield(x) => {
                self.tok("yield");
                if let Some(v) = &x.value {
                    self.child("value", (&**v).into());
                }
            }
            N::ExprYieldFrom(x) => {
                self.tok("yield");
                self.tok("from");
                self.child("value", (&*x.value).into());
            }
            N::ExprLambda(x) => {
                self.tok("lambda");
                if let Some(p) = &x.parameters {
                    self.child("args", (&**p).into());
                }
                self.child("body", (&*x.body).into());
            }
            N::Parameters(p) => {
                for x in &p.posonlyargs {
                    self.child("posonlyargs", x.into());
                }
                if !p.posonlyargs.is_empty() {
                    self.tok("/");
                }
                for x in &p.args {
                    self.child("args", x.into());
                }
                if let Some(v) = &p.vararg {
                    self.child("vararg", (&**v).into());
                } else if !p.kwonlyargs.is_empty() {
                    self.tok("*");
                }
                for x in &p.kwonlyargs {
                    self.child("kwonlyargs", x.into());
                }
                if let Some(v) = &p.kwarg {
                    self.child("kwarg", (&**v).into());
                }
            }
            N::ParameterWithDefault(p) => {
                self.child("parameter", (&p.parameter).into());
                if let Some(d) = &p.default {
                    self.child("default", (&**d).into());
                }
            }
            N::Parameter(p) => {
                self.ident("arg", &p.name);
                if let Some(a) = &p.annotation {
                    self.child("annotation", (&**a).into());
                }
            }
            N::ExprListComp(x) => {
                self.child("elt", (&*x.elt).into());
                x.generators.iter().for_each(|g| self.child("generators", g.into()));
            }
            N::ExprSetComp(x) => {
                self.child("elt", (&*x.elt).into());
                x.generators.iter().for_each(|g| self.child("generators", g.into()));
            }
            N::ExprGenerator(x) => {
                self.child("elt", (&*x.elt).into());
                x.generators.iter().for_each(|g| self.child("generators", g.into()));
            }
            N::ExprDictComp(x) => {
                match &x.key {
                    Some(k) => self.child("key", (&**k).into()),
                    None => self.tok("**"),
                }
                self.child("value", (&*x.value).into());
                x.generators.iter().for_each(|g| self.child("generators", g.into()));
            }
            N::Comprehension(g) => {
                if g.is_async {
                    self.tok("async");
                }
                self.child("target", (&g.target).into());
                self.child("iter", (&g.iter).into());
                for i in &g.ifs {
                    self.tok("if");
                    self.child("ifs", i.into());
                }
            }
            _ => {
                self.words.push(0x756e_6275_696c_74);
            }
        }
    }

    pub(crate) fn build(&mut self, c: &Ctx<'_, '_>) -> bool {
        let n = c.table.len();
        self.class.clear();
        self.class.resize(n, u32::MAX);
        self.map.clear();
        if c.incomplete {
            return false;
        }
        for id in (0..n).rev() {
            let node = c.table.node[id];
            let wanted = is_expression(node)
                || matches!(
                    node,
                    AnyNodeRef::Arguments(_) | AnyNodeRef::Keyword(_) | AnyNodeRef::Comprehension(_) | AnyNodeRef::Parameters(_)
                        | AnyNodeRef::ParameterWithDefault(_) | AnyNodeRef::Parameter(_)
                );
            if !wanted {
                continue;
            }
            self.key(node);
            let class = match self.map.get(self.words.as_slice()) {
                Some(&c) => c,
                None => {
                    self.map.insert(self.words.clone().into_boxed_slice(), id as u32);
                    id as u32
                }
            };
            self.class[id] = class;
        }
        true
    }
}

fn rows_expression_shape(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, shapes: &mut Shapes) {
    if !shapes.build(c) {
        return;
    }
    for id in 0..c.table.len() as u32 {
        if is_expression(c.table.node[id as usize]) {
            let class = shapes.class[id as usize];
            sink.row(rel, &[Cell::Node(id), Cell::Node(class), Cell::File, c.span(id)]);
        }
    }
}

// ------------------------------------------------------------------------------------------------ expression-type
/// `expression_types::build`: one bottom-up pass of basic types: literals by their kind, displays and comprehensions by what they
/// build, f-strings are strings, `not` is a boolean, unary and binary arithmetic by Python's numeric tower, conditional and boolean
/// expressions the union of their branches (a type included in another folds into the wider); everything else unknown.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Res {
    Atom(u8),
    Union(u16),
    Unknown,
    TypeError,
}

pub(crate) const TYPES: [&str; 14] =
    ["boolean", "integer", "float", "complex", "string", "bytes", "list", "tuple", "set", "dict", "generator", "null", "ellipsis", "unknown"];
const T_BOOL: u8 = 0;
const T_INT: u8 = 1;
const T_FLOAT: u8 = 2;
const T_COMPLEX: u8 = 3;
const T_STR: u8 = 4;
const T_BYTES: u8 = 5;
const T_LIST: u8 = 6;
const T_TUPLE: u8 = 7;
const T_SET: u8 = 8;
const T_DICT: u8 = 9;
const T_GEN: u8 = 10;
const T_NULL: u8 = 11;
const T_ELLIPSIS: u8 = 12;

fn included(a: u8, b: u8) -> bool {
    a == b || matches!((a, b), (T_BOOL, T_INT | T_FLOAT | T_COMPLEX) | (T_INT, T_FLOAT | T_COMPLEX) | (T_FLOAT, T_COMPLEX))
}

fn union(l: Res, r: Res) -> Res {
    let add = |mut mask: u16, t: u8| {
        if !(0..14u8).any(|k| mask & (1 << k) != 0 && included(t, k)) {
            mask |= 1 << t;
        }
        mask
    };
    match (l, r) {
        (Res::TypeError, _) | (_, Res::TypeError) => Res::TypeError,
        (Res::Unknown, _) | (_, Res::Unknown) => Res::Unknown,
        (Res::Atom(a), Res::Atom(b)) => {
            if included(a, b) {
                Res::Atom(b)
            } else if included(b, a) {
                Res::Atom(a)
            } else {
                Res::Union((1 << a) | (1 << b))
            }
        }
        (Res::Atom(a), Res::Union(m)) | (Res::Union(m), Res::Atom(a)) => Res::Union(add(m, a)),
        (Res::Union(m), Res::Union(n)) => Res::Union((0..14u8).filter(|k| n & (1 << k) != 0).fold(m, add)),
    }
}

fn binary(op: ast::Operator, l: Res, r: Res) -> Res {
    use ast::Operator as O;
    if op == O::Mod {
        match l {
            Res::Atom(T_STR) => return Res::Atom(T_STR),
            Res::Atom(T_BYTES) => return Res::Atom(T_BYTES),
            _ => {}
        }
    }
    let (Res::Atom(a), Res::Atom(b)) = (l, r) else { return Res::Unknown };
    let num = |t: u8| matches!(t, T_BOOL | T_INT | T_FLOAT | T_COMPLEX);
    let (na, nb) = (num(a), num(b));
    let coerced = if a == T_COMPLEX || b == T_COMPLEX {
        T_COMPLEX
    } else if a == T_FLOAT || b == T_FLOAT {
        T_FLOAT
    } else {
        T_INT
    };
    match op {
        O::Add => match (a, b) {
            (T_STR, T_STR) => Res::Atom(T_STR),
            (T_BYTES, T_BYTES) => Res::Atom(T_BYTES),
            (T_LIST, T_LIST) => Res::Atom(T_LIST),
            (T_TUPLE, T_TUPLE) => Res::Atom(T_TUPLE),
            _ if na && nb => Res::Atom(coerced),
            _ => Res::TypeError,
        },
        O::Sub => match (a, b) {
            (T_SET, T_SET) => Res::Atom(T_SET),
            _ if na && nb => Res::Atom(coerced),
            _ => Res::TypeError,
        },
        O::Mod => {
            if na && nb {
                Res::Atom(coerced)
            } else {
                Res::Unknown
            }
        }
        O::Mult => match (a, b) {
            (T_STR, T_INT) | (T_INT, T_STR) => Res::Atom(T_STR),
            _ if na && nb => Res::Atom(coerced),
            _ => Res::TypeError,
        },
        O::FloorDiv | O::Pow => {
            if na && nb {
                Res::Atom(coerced)
            } else {
                Res::TypeError
            }
        }
        O::Div => {
            if na && nb {
                Res::Atom(if coerced == T_INT { T_FLOAT } else { coerced })
            } else {
                Res::TypeError
            }
        }
        O::BitAnd | O::BitOr | O::BitXor | O::LShift | O::RShift => {
            if na && nb && coerced == T_INT {
                Res::Atom(T_INT)
            } else {
                Res::TypeError
            }
        }
        O::MatMult => Res::Unknown,
    }
}

#[derive(Default)]
pub(crate) struct Types {
    pub res: Vec<Res>,
}

impl Types {
    pub(crate) fn build(&mut self, c: &Ctx<'_, '_>) -> bool {
        use AnyNodeRef as N;
        let n = c.table.len();
        self.res.clear();
        self.res.resize(n, Res::Unknown);
        if c.incomplete {
            return false;
        }
        let at = |res: &Vec<Res>, e: &Expr| res[id_of(e.into()) as usize];
        for id in (0..n).rev() {
            let node = c.table.node[id];
            let r = match node {
                N::ExprDict(_) | N::ExprDictComp(_) => Res::Atom(T_DICT),
                N::ExprSet(_) | N::ExprSetComp(_) => Res::Atom(T_SET),
                N::ExprList(_) | N::ExprListComp(_) => Res::Atom(T_LIST),
                N::ExprTuple(_) => Res::Atom(T_TUPLE),
                N::ExprGenerator(_) => Res::Atom(T_GEN),
                N::ExprFString(_) | N::InterpolatedStringFormatSpec(_) => Res::Atom(T_STR),
                N::ExprNamed(x) => at(&self.res, &x.value),
                N::ExprIf(x) => union(at(&self.res, &x.body), at(&self.res, &x.orelse)),
                N::ExprBoolOp(b) => {
                    let mut it = b.values.iter().map(|v| at(&self.res, v));
                    it.next().map_or(Res::Unknown, |first| it.fold(first, union))
                }
                N::ExprUnaryOp(u) => {
                    let operand = at(&self.res, &u.operand);
                    match u.op {
                        ast::UnaryOp::Not => Res::Atom(T_BOOL),
                        ast::UnaryOp::Invert => match operand {
                            Res::Atom(T_BOOL | T_INT) => Res::Atom(T_INT),
                            Res::Atom(_) => Res::TypeError,
                            _ => Res::Unknown,
                        },
                        ast::UnaryOp::UAdd | ast::UnaryOp::USub => match operand {
                            Res::Atom(T_BOOL) => Res::Atom(T_INT),
                            Res::Atom(t @ (T_INT | T_FLOAT | T_COMPLEX)) => Res::Atom(t),
                            Res::Atom(_) => Res::TypeError,
                            _ => Res::Unknown,
                        },
                    }
                }
                N::ExprBinOp(b) => binary(b.op, at(&self.res, &b.left), at(&self.res, &b.right)),
                _ => match literal_properties(node) {
                    Some(p) => Res::Atom(match p.kind {
                        "boolean" => T_BOOL,
                        "integer" => T_INT,
                        "float" => T_FLOAT,
                        "complex" => T_COMPLEX,
                        "string" => T_STR,
                        "bytes" => T_BYTES,
                        "null" => T_NULL,
                        _ => T_ELLIPSIS,
                    }),
                    None => Res::Unknown,
                },
            };
            self.res[id] = r;
        }
        true
    }
}

fn rows_expression_type(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, types: &mut Types) {
    if !types.build(c) {
        return;
    }
    for id in 0..c.table.len() as u32 {
        if !is_expression(c.table.node[id as usize]) {
            continue;
        }
        let row = |sink: &mut dyn Sink, res: &'static str, t: &'static str| sink.row(rel, &[Cell::Node(id), Cell::Text(res), Cell::Text(t), Cell::File, c.span(id)]);
        match types.res[id as usize] {
            Res::Atom(t) => row(sink, "atom", TYPES[t as usize]),
            Res::Union(m) => {
                for k in 0..14 {
                    if m & (1 << k) != 0 {
                        row(sink, "union", TYPES[k]);
                    }
                }
            }
            Res::Unknown => row(sink, "unknown", ""),
            Res::TypeError => row(sink, "type-error", ""),
        }
    }
}

// ------------------------------------------------------------------------------------------------ driver
/// Reusable per-worker state of the derived analyses.
#[derive(Default)]
pub(crate) struct State {
    pub shapes: Shapes,
    pub types: Types,
}

pub(crate) fn produce(c: &Ctx<'_, '_>, wanted: u64, sink: &mut dyn Sink, buf: &mut String, st: &mut State) {
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
    if let Some(r) = want("embedded-expression") {
        rows_embedded_expression(c, sink, r);
    }
    if let Some(r) = want("function") {
        rows_function(c, sink, r);
    }
    if let Some(r) = want("parameter") {
        rows_parameter(c, sink, r);
    }
    if let Some(r) = want("decorates") {
        rows_decorates(c, sink, r, buf);
    }
    if let Some(r) = want("inherits") {
        rows_inherits(c, sink, r, buf);
    }
    if let Some(r) = want("bytes-literal") {
        rows_bytes_literal(c, sink, r, buf);
    }
    if let Some(r) = want("interpolated-string-literal") {
        rows_interpolated_string_literal(c, sink, r);
    }
    if let Some(r) = want("expression-shape") {
        rows_expression_shape(c, sink, r, &mut st.shapes);
    }
    if let Some(r) = want("expression-type") {
        rows_expression_type(c, sink, r, &mut st.types);
    }
    if let Some(r) = want("filesystem.executable") {
        // the executable bit of the file as the run reads it (host)
        use std::os::unix::fs::PermissionsExt as _;
        let exec = std::fs::metadata(c.path).map(|m| m.permissions().mode() & 0o111 != 0).unwrap_or(false);
        sink.row(r, &[Cell::File, Cell::Flag(exec)]);
    }
}
