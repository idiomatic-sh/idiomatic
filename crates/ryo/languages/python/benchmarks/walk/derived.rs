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
    /// a token of the parser's token array, by its index (facts: no lookup by span)
    Token(u32),
    Key(&'static str, u32, u32),
    Text(&'s str),
    /// a text fixed in the program (join: decoded through a per-run cache by address)
    Word(&'static str),
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
                Cell::Node(n) | Cell::Token(n) => u64::from(n),
                Cell::Key(_, a, b) => u64::from(a ^ b),
                Cell::Text(t) => t.len() as u64,
                Cell::Word(t) => t.len() as u64,
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
                Cell::Token(i) => {
                    let _ = write!(s, "token#{i}");
                }
                Cell::Text(t) | Cell::Word(t) => esc_into(&mut s, t),
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
    "syntax-child",
    "syntax-tree",
    "source-location",
    "comment",
    "indent-region",
    "function-terminal",
    "constant-key",
    "token-link",
    "format-parse",
    "format-field",
    "brace-format-parse",
    "brace-format-field",
    "literal-expression-parse",
    "raw-format-segment",
    "raw-format-field",
    "format-specification",
];

/// The other relation computed from the same parse (its rows come from one pass).
pub(crate) fn partner(i: usize) -> Option<usize> {
    let pair = |a: &str, b: &str| match RELATIONS[i] {
        x if x == a => index_of(b),
        x if x == b => index_of(a),
        _ => None,
    };
    pair("format-parse", "format-field").or_else(|| pair("brace-format-parse", "brace-format-field")).or_else(|| pair("raw-format-segment", "raw-format-field"))
}

pub(crate) fn index_of(name: &str) -> Option<usize> {
    RELATIONS.iter().position(|r| *r == name)
}

/// The per-file inputs: the flat table of the parser's nodes, the source, the lexical model (for owners), parse completeness.
pub(crate) struct Ctx<'c, 'a> {
    pub parsed: &'c ruff_python_parser::Parsed<ast::Mod>,
    pub path: &'c std::path::Path,
    pub table: &'c Table<'a>,
    pub source: &'c str,
    pub model: &'c Model,
    pub incomplete: bool,
    /// the run's view, for token ids (facts; None in the rows-only modes)
    pub view: Option<&'c crate::view::View>,
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

/// The nodes a rows form runs over: the kind's bucket, or (facts: the per-key form) the one node asked for when it is of the kind.
fn ids<'x>(c: &'x Ctx<'_, '_>, k: NodeKind, only: &'x Option<u32>) -> &'x [u32] {
    match only {
        Some(id) => {
            if (*id as usize) < c.table.len() && c.table.kind[*id as usize] == k as u8 {
                std::slice::from_ref(id)
            } else {
                &[]
            }
        }
        None => c.kind(k),
    }
}

/// Every node, or the one asked for.
fn all_ids(c: &Ctx<'_, '_>, only: Option<u32>) -> std::ops::Range<u32> {
    match only {
        Some(id) if (id as usize) < c.table.len() => id..id + 1,
        Some(_) => 0..0,
        None => 0..c.table.len() as u32,
    }
}

/// The relations with a per-key function and the column it is keyed on (facts): "the rows of R whose column c is this node",
/// computed at that node from the tree, with no pass over the file (the whole-file analyses of shapes, types, terminals and
/// constant keys run once per file, then answer per node).
pub(crate) const KEYED: &[(&str, u8)] = &[
    ("literal-properties", 0),
    ("string-literal", 0),
    ("static-import", 0),
    ("numeric-literal", 0),
    ("call-argument", 0),
    ("call-argument", 2),
    ("function", 0),
    ("parameter", 0),
    ("decorates", 0),
    ("decorates", 1),
    ("bytes-literal", 0),
    ("expression-shape", 0),
    ("expression-shape", 1),
    ("expression-type", 0),
    ("function-terminal", 0),
    ("constant-key", 0),
    ("token-link", 0),
    ("token-link", 1),
    ("format-parse", 0),
    ("format-field", 0),
    ("brace-format-parse", 0),
    ("brace-format-field", 0),
    ("literal-expression-parse", 0),
    ("raw-format-segment", 0),
    ("raw-format-field", 0),
];

/// Bitmask of the columns of `name` that have a per-key function.
pub(crate) fn key_columns(name: &str) -> u16 {
    KEYED.iter().filter(|(n, _)| *n == name).fold(0, |m, (_, c)| m | (1 << c))
}

/// Whether the relation's rows read the lexical model (the scope a node is evaluated in).
pub(crate) fn needs_model(name: &str) -> bool {
    matches!(name, "string-literal" | "static-import" | "bytes-literal" | "embedded-expression")
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

fn rows_identifier(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, only: Option<u32>) {
    if c.incomplete {
        return;
    }
    for id in all_ids(c, only) {
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

fn rows_literal_properties(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, only: Option<u32>) {
    for k in LITERAL_KINDS {
        for &id in ids(c, k, &only) {
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

fn rows_string_literal(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, buf: &mut String, only: Option<u32>) {
    for k in STRING_KINDS {
        for &id in ids(c, k, &only) {
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

fn rows_static_import(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, buf: &mut String, only: Option<u32>) {
    for &id in ids(c, NodeKind::Alias, &only) {
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

fn rows_numeric_literal(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, buf: &mut String, only: Option<u32>) {
    for &id in ids(c, NodeKind::ExprNumberLiteral, &only) {
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

fn rows_call_argument(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, only: Option<u32>) {
    for &id in ids(c, NodeKind::ExprCall, &only) {
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

fn rows_embedded_expression(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, only: Option<u32>) {
    if !c.model.annotation.iter().any(|a| *a) {
        return;
    }
    for &id in ids(c, NodeKind::ExprStringLiteral, &only) {
        if let Some((kind, s, e, exact)) = embedded_expression(c, id) {
            sink.row(rel, &[Cell::Key(kind, s, e), Cell::Node(id), Cell::Text("deferred-type-expression"), Cell::Text(if exact { "exact-content" } else { "whole-host" }), Cell::File, Cell::Span(s, e)]);
        }
    }
}

// ------------------------------------------------------------------------------------------------ function, parameter
fn rows_function(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, only: Option<u32>) {
    for &id in ids(c, NodeKind::StmtFunctionDef, &only) {
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

fn rows_parameter(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, only: Option<u32>) {
    for &id in ids(c, NodeKind::Parameter, &only) {
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

fn rows_decorates(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, buf: &mut String, only: Option<u32>) {
    for &id in ids(c, NodeKind::Decorator, &only) {
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

fn rows_inherits(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, buf: &mut String, only: Option<u32>) {
    for &id in ids(c, NodeKind::StmtClassDef, &only) {
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
fn rows_bytes_literal(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, buf: &mut String, only: Option<u32>) {
    use std::fmt::Write as _;
    for &id in ids(c, NodeKind::ExprBytesLiteral, &only) {
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

fn rows_interpolated_string_literal(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, only: Option<u32>) {
    for &id in ids(c, NodeKind::ExprFString, &only) {
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
    /// hash-consing by a 128-bit fingerprint of the key words (two independent hashes; a collision would merge two classes)
    map: crate::sema::FxMap<(u64, u64), u32>,
    words: Vec<u64>,
    text: String,
}

pub(crate) fn fingerprint(words: &[u64]) -> (u64, u64) {
    let (mut a, mut b) = (0x243f_6a88_85a3_08d3u64, 0x1319_8a2e_0370_7344u64);
    for &w in words {
        a = (a.rotate_left(5) ^ w).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
        b = (b ^ w.rotate_left(31)).wrapping_mul(0x9e37_79b9_7f4a_7c15).rotate_left(23);
    }
    (a ^ (words.len() as u64), b)
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
                if s.value.is_implicit_concatenated() {
                    let mut v = std::mem::take(&mut self.text);
                    v.clear();
                    for p in s.value.iter() {
                        v.push_str(&p.value);
                    }
                    self.lit(&v);
                    self.text = v;
                } else {
                    self.lit(s.value.to_str());
                }
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
                    match i.as_u64() {
                        Some(v) => self.word(W_PROP ^ 1, v),
                        None => self.prop("value", &i.to_string()),
                    }
                }
                ast::Number::Float(f) => {
                    self.prop("kind", "float");
                    self.word(W_PROP ^ 2, f.to_bits());
                }
                ast::Number::Complex { real, imag } => {
                    self.prop("kind", "complex");
                    self.word(W_PROP ^ 2, real.to_bits());
                    self.word(W_PROP ^ 3, imag.to_bits());
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
            let class = *self.map.entry(fingerprint(&self.words)).or_insert(id as u32);
            self.class[id] = class;
        }
        true
    }
}

/// A whole-file analysis runs once per file (facts): 0 not yet, 1 built, 2 built and empty.
fn once(flag: &mut u8, build: impl FnOnce() -> bool) -> bool {
    if *flag == 0 {
        let t = crate::nav::Tick::now();
        *flag = if build() { 1 } else { 2 };
        BUILD_NS.with(|b| b.set(b.get() + t.elapsed().as_nanos() as u64));
    }
    *flag == 1
}

/// A whole fill's caller clocks all of it: the analyses' own clock is dropped.
pub(crate) fn drop_build_clock() {
    BUILD_NS.with(|b| b.set(0));
}

thread_local! {
    /// time in the whole-file analyses on this thread (read into the state after each call)
    static BUILD_NS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

fn rows_expression_shape(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, shapes: &mut Shapes, flag: &mut u8, only: Option<u32>) {
    if !once(flag, || shapes.build(c)) {
        return;
    }
    for id in all_ids(c, only) {
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

fn rows_expression_type(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, types: &mut Types, flag: &mut u8, only: Option<u32>) {
    if !once(flag, || types.build(c)) {
        return;
    }
    for id in all_ids(c, only) {
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

// ------------------------------------------------------------------------------------------------ syntax-child
/// `syntax_child`: the lossless tree's edges (parent, child, ordinal among nodes and tokens, field), from walk's view of the
/// parser's nodes and tokens (`view.rs`), one whitespace leaf per gap; identical rows to `semantic-child` on Python.
/// Builds walk's view of this file once (per file, on first demand) and the kind-name table its cells read.
fn ensure_view(c: &Ctx<'_, '_>, st: &mut State) {
    if st.view_file.is_none() {
        st.sym.reset_file();
        st.view.build(c.table, c.parsed, c.source, &mut st.sym);
        st.view_file = Some(0);
        for &k in &st.view.kind {
            let k = k as usize;
            if k >= st.kind_names.len() {
                st.kind_names.resize(k + 1, "");
            }
            if st.kind_names[k].is_empty() {
                st.kind_names[k] = kind_static(st.sym.str(k as u32).unwrap_or("?"));
            }
        }
    }
}

fn rows_syntax_child(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, st: &mut State, only: Option<u32>) {
    ensure_view(c, st);
    let v = &st.view;
    let sym = &st.sym;
    let names = &st.kind_names;
    let key = |id: usize| Cell::Key(names[v.kind[id] as usize], v.start[id], v.stop[id]);
    for id in 0..v.len() {
        let p = v.parent[id];
        if p == u32::MAX {
            continue;
        }
        let field = sym.str(v.field[id]).unwrap_or("none");
        sink.row(rel, &[key(p as usize), key(id), Cell::Num(i64::from(v.ordinal[id])), Cell::Text(field), Cell::File]);
    }
}

/// Kind names are base symbols that live as long as the worker; leaked once per distinct name for the `Key` cell.
fn kind_static(name: &str) -> &'static str {
    use std::sync::Mutex;
    static NAMES: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    let mut names = NAMES.lock().expect("names");
    if let Some(n) = names.iter().find(|n| **n == name) {
        return n;
    }
    let leaked: &'static str = Box::leak(name.to_owned().into_boxed_str());
    names.push(leaked);
    leaked
}

// ------------------------------------------------------------------------------------------------ walk's tree view, proven
/// `syntax-tree` (= `semantic-syntax` on Python), `source-location`, `comment`, `indent-region` from walk's view: every node and
/// token of the lossless tree with its kind and text (tokens and whitespace only), its lines and columns, the comments, and the
/// indentation regions (the module is region 0).
fn rows_view(c: &Ctx<'_, '_>, sink: &mut dyn Sink, which: &str, rel: usize, st: &mut State, only: Option<u32>) {
    ensure_view(c, st);
    let v = &st.view;
    let sym = &st.sym;
    let names = &st.kind_names;
    let key = |id: usize| Cell::Key(names[v.kind[id] as usize], v.start[id], v.stop[id]);
    match which {
        "syntax-tree" => {
            for id in 0..v.len() {
                let text = if id < v.n_nodes { Cell::Null } else { sym.str(v.text[id]).map_or(Cell::Null, Cell::Text) };
                sink.row(rel, &[key(id), Cell::Text(names[v.kind[id] as usize]), text, Cell::File, Cell::Span(v.start[id], v.stop[id])]);
            }
        }
        "source-location" => {
            for id in 0..v.len() {
                let (s, e) = (v.start[id], v.stop[id]);
                sink.row(rel, &[key(id), Cell::File, Cell::Span(s, e), Cell::Num(i64::from(v.line_of(s))), Cell::Num(i64::from(v.line_of(e))), Cell::Num(i64::from(s)), Cell::Num(i64::from(e)), Cell::Num(i64::from(v.column_of(s))), Cell::Num(i64::from(v.column_of(e)))]);
            }
        }
        "comment" => {
            let k = sym.get_base("comment");
            for id in v.n_nodes..v.len() {
                if Some(v.kind[id]) == k {
                    let text = &c.source[v.start[id] as usize + 1..v.stop[id] as usize];
                    sink.row(rel, &[key(id), Cell::Text(text), Cell::File, Cell::Span(v.start[id], v.stop[id]), Cell::Flag(false)]);
                }
            }
        }
        _ => {
            for r in &v.regions {
                let [rid, parent, ind, s, e] = *r;
                sink.row(rel, &[key(rid as usize), key(parent as usize), Cell::Text(sym.str(ind).unwrap_or("")), Cell::Num(i64::from(s)), Cell::Num(i64::from(e)), Cell::File, Cell::Span(s, e)]);
            }
        }
    }
}

// ------------------------------------------------------------------------------------------------ function-terminal
/// `terminal_flow::build`: one reverse-preorder fold of how each statement ends (implicit, raise, return-or-raise, conditional
/// return) and whether it sometimes breaks out of its loop; a function's outcome is its body's sequence.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Term {
    Implicit,
    Raise,
    Return,
    RaiseOrReturn,
    ConditionalReturn,
    #[default]
    None,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Summary {
    terminal: Term,
    function: Term,
    breaks: bool,
    wildcard: bool,
}

fn and_then(l: Term, r: Term) -> Term {
    use Term::*;
    match (l, r) {
        (None | Implicit, o) | (o, None | Implicit) => o,
        (ConditionalReturn, ConditionalReturn) => ConditionalReturn,
        (Raise, ConditionalReturn | Return) | (ConditionalReturn | Return, Raise) | (RaiseOrReturn, _) | (_, RaiseOrReturn) => RaiseOrReturn,
        (Return, ConditionalReturn | Return) | (ConditionalReturn, Return) => Return,
        (Raise, Raise) => Raise,
    }
}

fn branch(l: Term, r: Term) -> Term {
    use Term::*;
    match (l, r) {
        (None, o) | (o, None) => o,
        (Implicit, Implicit | Raise) | (Raise, Implicit) => Implicit,
        (Implicit, Return | RaiseOrReturn | ConditionalReturn) | (Return | RaiseOrReturn | ConditionalReturn, Implicit) | (ConditionalReturn, ConditionalReturn) => {
            ConditionalReturn
        }
        (Raise, ConditionalReturn | Return) | (ConditionalReturn | Return, Raise) | (RaiseOrReturn, _) | (_, RaiseOrReturn) => RaiseOrReturn,
        (Return, ConditionalReturn | Return) | (ConditionalReturn, Return) => Return,
        (Raise, Raise) => Raise,
    }
}

fn has_return(t: Term) -> bool {
    matches!(t, Term::Return | Term::RaiseOrReturn | Term::ConditionalReturn)
}

#[derive(Default)]
pub(crate) struct Terminals {
    pub s: Vec<Summary>,
}

impl Terminals {
    fn get(&self, stmt: &ast::Stmt) -> Summary {
        self.s[id_of(stmt.into()) as usize]
    }
    fn get_pattern(&self, p: &ast::Pattern) -> Summary {
        self.s[id_of(p.into()) as usize]
    }
    fn sequence(&self, body: &[ast::Stmt]) -> Term {
        let o = body.iter().fold(Term::None, |cur, st| and_then(cur, self.get(st).terminal));
        if o == Term::None { Term::Implicit } else { o }
    }
    fn body_breaks(&self, body: &[ast::Stmt]) -> bool {
        for st in body {
            match st {
                ast::Stmt::Break(_) => return true,
                ast::Stmt::Return(_) | ast::Stmt::Raise(_) => return false,
                _ if self.get(st).breaks => return true,
                _ => {}
            }
        }
        false
    }
    fn always_breaks(body: &[ast::Stmt]) -> bool {
        for st in body {
            match st {
                ast::Stmt::Break(_) => return true,
                ast::Stmt::Return(_) | ast::Stmt::Raise(_) => return false,
                _ => {}
            }
        }
        false
    }

    fn summary(&self, node: AnyNodeRef<'_>) -> Summary {
        use AnyNodeRef as N;
        let d = Summary::default();
        match node {
            N::PatternMatchAs(a) => Summary { wildcard: a.pattern.as_deref().is_none_or(|p| self.get_pattern(p).wildcard), ..d },
            N::PatternMatchOr(o) => {
                Summary { wildcard: !o.patterns.is_empty() && o.patterns.iter().all(|p| self.get_pattern(p).wildcard), ..d }
            }
            N::StmtFunctionDef(f) => Summary { function: self.sequence(&f.body), ..d },
            N::StmtBreak(_) => Summary { terminal: Term::None, breaks: true, ..d },
            N::StmtReturn(_) => Summary { terminal: Term::RaiseOrReturn, ..d },
            N::StmtRaise(_) => Summary { terminal: Term::Raise, ..d },
            N::StmtIf(i) => {
                let mut branches = self.sequence(&i.body);
                let mut exhaustive = false;
                let mut breaks = self.body_breaks(&i.body);
                for clause in &i.elif_else_clauses {
                    exhaustive |= clause.test.is_none();
                    branches = branch(branches, self.sequence(&clause.body));
                    breaks |= self.body_breaks(&clause.body);
                }
                let terminal = if exhaustive {
                    and_then(Term::None, branches)
                } else if has_return(branches) {
                    and_then(Term::None, Term::ConditionalReturn)
                } else {
                    Term::None
                };
                Summary { terminal, breaks, ..d }
            }
            N::StmtMatch(m) => {
                let mut t = Term::None;
                let mut exhaustive = false;
                let mut breaks = false;
                for case in &m.cases {
                    t = branch(t, self.sequence(&case.body));
                    exhaustive |= case.guard.is_none() && self.get_pattern(&case.pattern).wildcard;
                    breaks |= self.body_breaks(&case.body);
                }
                let terminal = if exhaustive {
                    and_then(Term::None, t)
                } else if has_return(t) {
                    and_then(Term::None, Term::ConditionalReturn)
                } else {
                    Term::None
                };
                Summary { terminal, breaks, ..d }
            }
            N::StmtFor(_) | N::StmtWhile(_) => {
                let (body, orelse): (&[ast::Stmt], &[ast::Stmt]) = match node {
                    N::StmtFor(f) => (&f.body, &f.orelse),
                    N::StmtWhile(w) => (&w.body, &w.orelse),
                    _ => unreachable!(),
                };
                let b = self.sequence(body);
                let o = self.sequence(orelse);
                if Self::always_breaks(body) {
                    Summary { terminal: Term::None, breaks: false, ..d }
                } else {
                    let mut terminal = and_then(Term::None, b);
                    if !self.body_breaks(body) {
                        terminal = and_then(terminal, o);
                    }
                    Summary { terminal, breaks: !has_return(b) && self.body_breaks(orelse), ..d }
                }
            }
            N::StmtWith(w) => Summary { terminal: self.sequence(&w.body), breaks: self.body_breaks(&w.body), ..d },
            N::StmtTry(t) => {
                let body = self.sequence(&t.body);
                let mut branches = Term::None;
                let mut handler_breaks = false;
                for h in &t.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = h;
                    branches = branch(branches, self.sequence(&h.body));
                    handler_breaks |= self.body_breaks(&h.body);
                }
                let orelse = self.sequence(&t.orelse);
                let finally = self.sequence(&t.finalbody);
                let mut terminal = Term::None;
                if has_return(body) {
                    terminal = and_then(terminal, Term::ConditionalReturn);
                }
                terminal = and_then(terminal, finally);
                if t.orelse.is_empty() {
                    if has_return(branches) {
                        terminal = and_then(terminal, Term::ConditionalReturn);
                    }
                } else {
                    terminal = and_then(terminal, branch(branches, orelse));
                }
                Summary {
                    terminal,
                    breaks: self.body_breaks(&t.body) || handler_breaks || self.body_breaks(&t.orelse) || self.body_breaks(&t.finalbody),
                    ..d
                }
            }
            _ => d,
        }
    }

    pub(crate) fn build(&mut self, c: &Ctx<'_, '_>) -> bool {
        let n = c.table.len();
        self.s.clear();
        self.s.resize(n, Summary::default());
        if c.incomplete {
            return false;
        }
        for id in (0..n).rev() {
            self.s[id] = self.summary(c.table.node[id]);
        }
        true
    }

    pub(crate) fn outcome(&self, id: u32) -> &'static str {
        match self.s[id as usize].function {
            Term::None | Term::Implicit => "implicit",
            Term::Raise => "raise",
            Term::Return | Term::RaiseOrReturn => "return-or-raise",
            Term::ConditionalReturn => "conditional-return",
        }
    }
}

fn rows_function_terminal(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, t: &mut Terminals, flag: &mut u8, only: Option<u32>) {
    if !once(flag, || t.build(c)) {
        return;
    }
    for &id in ids(c, NodeKind::StmtFunctionDef, &only) {
        sink.row(rel, &[Cell::Node(id), Cell::Text(t.outcome(id)), Cell::File, c.span(id)]);
    }
}

// ------------------------------------------------------------------------------------------------ constant-key
/// `constant_keys::prepare`: Python's equality of constant values as dictionary keys: numbers by exact value across bool, int,
/// float and complex (`True == 1 == 1.0`), a signed number and `real ± imag j` folded, strings by decoded value, bytes, None,
/// Ellipsis, tuples of keyed elements; an integer too large for 64 bits keys by its text. A class is named by its highest member.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RealKey {
    Int(i128),
    Float(u64),
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct NumberKey {
    real: RealKey,
    imag: RealKey,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Numeric {
    Number(NumberKey),
    Structural,
}

fn float_key(v: f64) -> RealKey {
    if v == 0.0 {
        return RealKey::Int(0);
    }
    if v.is_finite() && v.abs() < f64::from_bits(0x43f0_0000_0000_0000) {
        let i = v as i128;
        if i as f64 == v {
            return RealKey::Int(i);
        }
    }
    RealKey::Float(v.to_bits())
}

fn to_float(v: RealKey) -> f64 {
    match v {
        RealKey::Int(i) => i as f64,
        RealKey::Float(b) => f64::from_bits(b),
    }
}

fn negate(v: RealKey) -> Option<RealKey> {
    match v {
        RealKey::Int(i) => i.checked_neg().map(RealKey::Int),
        RealKey::Float(b) => Some(RealKey::Float((-f64::from_bits(b)).to_bits())),
    }
}

#[derive(Default)]
pub(crate) struct ConstantKeys {
    pub key: Vec<u32>,
    num: Vec<Option<Numeric>>,
    map: crate::sema::FxMap<(u64, u64), u32>,
    words: Vec<u64>,
    text: String,
}

impl ConstantKeys {
    fn push_number(&mut self, n: NumberKey) {
        let enc = |r: RealKey| match r {
            RealKey::Int(i) => [1u64, i as u64, (i >> 64) as u64],
            RealKey::Float(b) => [2u64, b, 0],
        };
        self.words.push(0x6e75_6d);
        self.words.extend_from_slice(&enc(n.real));
        self.words.extend_from_slice(&enc(n.imag));
    }
    fn push_literal(&mut self, kind: u64, text: Option<&str>) {
        self.words.push(0x6c69_74 ^ (kind << 32));
        self.words.push(text.map_or(0, h));
    }

    fn build_key(&mut self, node: AnyNodeRef<'_>) -> Option<Option<Numeric>> {
        use AnyNodeRef as N;
        let zero = RealKey::Int(0);
        match node {
            N::ExprNumberLiteral(n) => {
                let k = match &n.value {
                    ast::Number::Int(i) => match i.as_u64() {
                        Some(v) => NumberKey { real: RealKey::Int(i128::from(v)), imag: zero },
                        None => {
                            self.push_literal(1, Some(&i.to_string()));
                            return Some(Some(Numeric::Structural));
                        }
                    },
                    ast::Number::Float(f) => NumberKey { real: float_key(*f), imag: zero },
                    ast::Number::Complex { real, imag } => NumberKey { real: float_key(*real), imag: float_key(*imag) },
                };
                self.push_number(k);
                Some(Some(Numeric::Number(k)))
            }
            N::ExprBooleanLiteral(b) => {
                let k = NumberKey { real: RealKey::Int(i128::from(u8::from(b.value))), imag: zero };
                self.push_number(k);
                Some(Some(Numeric::Number(k)))
            }
            N::ExprStringLiteral(s) => {
                if s.value.is_implicit_concatenated() {
                    let mut v = std::mem::take(&mut self.text);
                    v.clear();
                    for p in s.value.iter() {
                        v.push_str(&p.value);
                    }
                    self.push_literal(2, Some(&v));
                    self.text = v;
                } else {
                    self.push_literal(2, Some(s.value.to_str()));
                }
                Some(None)
            }
            N::ExprBytesLiteral(b) => {
                use std::fmt::Write as _;
                let mut v = String::new();
                for byte in b.value.bytes() {
                    let _ = write!(v, "{byte:02x}");
                }
                self.push_literal(3, Some(&v));
                Some(None)
            }
            N::ExprNoneLiteral(_) => {
                self.push_literal(4, None);
                Some(None)
            }
            N::ExprEllipsisLiteral(_) => {
                self.push_literal(5, None);
                Some(None)
            }
            N::ExprTuple(t) => {
                self.words.push(0x7475_706c_65);
                for e in t.elts.iter() {
                    let k = self.key[id_of(e.into()) as usize];
                    if k == u32::MAX {
                        return None;
                    }
                    self.words.push(u64::from(k));
                }
                self.words.push(0x656e_64);
                Some(None)
            }
            N::ExprUnaryOp(u) => {
                let Some(Numeric::Number(mut n)) = self.num[id_of((&*u.operand).into()) as usize] else { return None };
                match u.op {
                    ast::UnaryOp::UAdd => {}
                    ast::UnaryOp::USub => {
                        n.real = negate(n.real)?;
                        n.imag = negate(n.imag)?;
                    }
                    _ => return None,
                }
                self.push_number(n);
                Some(Some(Numeric::Number(n)))
            }
            N::ExprBinOp(b) => {
                let Expr::NumberLiteral(r) = &*b.right else { return None };
                if !matches!(r.value, ast::Number::Complex { .. }) {
                    return None;
                }
                let minus = match b.op {
                    ast::Operator::Add => false,
                    ast::Operator::Sub => true,
                    _ => return None,
                };
                let Some(Numeric::Number(l)) = self.num[id_of((&*b.left).into()) as usize] else { return None };
                let Some(Numeric::Number(rn)) = self.num[id_of((&*b.right).into()) as usize] else { return None };
                if l.imag != zero || rn.real != zero {
                    return None;
                }
                let mut ri = to_float(rn.imag);
                if minus {
                    ri = -ri;
                }
                let k = NumberKey { real: float_key(to_float(l.real)), imag: float_key(ri) };
                self.push_number(k);
                Some(Some(Numeric::Number(k)))
            }
            _ => None,
        }
    }

    pub(crate) fn build(&mut self, c: &Ctx<'_, '_>) {
        let n = c.table.len();
        self.key.clear();
        self.key.resize(n, u32::MAX);
        self.num.clear();
        self.num.resize(n, None);
        self.map.clear();
        for id in (0..n).rev() {
            let node = c.table.node[id];
            if !matches!(
                node,
                AnyNodeRef::ExprNumberLiteral(_) | AnyNodeRef::ExprBooleanLiteral(_) | AnyNodeRef::ExprStringLiteral(_) | AnyNodeRef::ExprBytesLiteral(_)
                    | AnyNodeRef::ExprNoneLiteral(_) | AnyNodeRef::ExprEllipsisLiteral(_) | AnyNodeRef::ExprTuple(_) | AnyNodeRef::ExprUnaryOp(_)
                    | AnyNodeRef::ExprBinOp(_)
            ) {
                continue;
            }
            self.words.clear();
            if let Some(numeric) = self.build_key(node) {
                let rep = *self.map.entry(fingerprint(&self.words)).or_insert(id as u32);
                self.key[id] = rep;
                self.num[id] = numeric;
            }
        }
    }
}

fn rows_constant_key(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, k: &mut ConstantKeys, flag: &mut u8, only: Option<u32>) {
    once(flag, || {
        k.build(c);
        true
    });
    for id in all_ids(c, only) {
        let key = k.key[id as usize];
        if key != u32::MAX {
            sink.row(rel, &[Cell::Node(id), Cell::Node(key), Cell::File, c.span(id)]);
        }
    }
}

// ------------------------------------------------------------------------------------------------ token-link
/// `token_links::scan`: one linear pass over the parser's tokens: each token's next token, every bracket's matching closer, and
/// the innermost open bracket enclosing each token; an error token or an unbalanced bracket loses the pairing from there on.
pub(crate) fn token_links(c: &Ctx<'_, '_>, out: &mut dyn FnMut(usize, usize, &'static str)) {
    use ruff_python_ast::token::TokenKind as T;
    let tokens = c.parsed.tokens();
    let mut stack: Vec<(T, usize)> = Vec::new();
    let mut previous: Option<usize> = None;
    for (at, t) in tokens.iter().enumerate() {
        let token = t.kind();
        if matches!(token, T::Unknown) {
            previous = None;
            stack.clear();
            continue;
        }
        let mut paired = None;
        let mut enclosing = stack.last().map(|o| o.1);
        if matches!(token, T::Lpar | T::Lsqb | T::Lbrace) {
            stack.push((token, at));
        } else if matches!(token, T::Rpar | T::Rsqb | T::Rbrace) {
            match stack.pop() {
                Some((open, o))
                    if matches!((open, token), (T::Lpar, T::Rpar) | (T::Lsqb, T::Rsqb) | (T::Lbrace, T::Rbrace)) =>
                {
                    paired = Some(o);
                    enclosing = stack.last().map(|o| o.1);
                }
                _ => {
                    stack.clear();
                    enclosing = None;
                }
            }
        }
        if let Some(p) = previous.replace(at) {
            out(p, at, "next-token");
        }
        if let Some(o) = paired {
            out(o, at, "matching-delimiter");
        }
        if let Some(e) = enclosing {
            out(at, e, "enclosing-delimiter");
        }
    }
}

fn rows_token_link(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, only: Option<u32>) {
    let _ = only;
    let tokens = c.parsed.tokens();
    let direct = c.view.is_some();
    let key = |i: usize| {
        if direct {
            return Cell::Token(i as u32);
        }
        let t = &tokens[i];
        Cell::Key(crate::view::token_kind_name(t.kind()), t.start().to_u32(), t.end().to_u32())
    };
    token_links(c, &mut |s, t, k| {
        let st = &tokens[s];
        sink.row(rel, &[key(s), key(t), Cell::Word(k), Cell::File, Cell::Span(st.start().to_u32(), st.end().to_u32())]);
    });
}

/// The index of the string token spanning exactly `[start, end)` (zero-width tokens may share its start).
fn token_at(c: &Ctx<'_, '_>, start: u32, end: u32) -> Option<u32> {
    let tokens = c.parsed.tokens();
    let mut i = tokens.partition_point(|t| t.start().to_u32() < start);
    while let Some(t) = tokens.get(i) {
        if t.start().to_u32() != start {
            return None;
        }
        if t.end().to_u32() == end && matches!(t.kind(), ruff_python_ast::token::TokenKind::String) {
            return Some(i as u32);
        }
        i += 1;
    }
    None
}

/// The bracket structure of the token array (facts): per token its matching bracket and its innermost enclosing open bracket,
/// and whether it is a live token (not an error token). One pass per file, on the first link asked for; the links themselves are
/// index arithmetic on the token array.
#[derive(Default)]
pub(crate) struct Links {
    pub mate: Vec<u32>,
    pub encl: Vec<u32>,
    pub live: Vec<bool>,
}

const NO_TOKEN: u32 = u32::MAX;

impl Links {
    pub(crate) fn build(&mut self, c: &Ctx<'_, '_>) -> bool {
        use ruff_python_ast::token::TokenKind as T;
        let tokens = c.parsed.tokens();
        let n = tokens.len();
        self.mate.clear();
        self.mate.resize(n, NO_TOKEN);
        self.encl.clear();
        self.encl.resize(n, NO_TOKEN);
        self.live.clear();
        self.live.resize(n, true);
        let mut stack: Vec<(T, usize)> = Vec::new();
        for (at, t) in tokens.iter().enumerate() {
            let token = t.kind();
            if matches!(token, T::Unknown) {
                self.live[at] = false;
                stack.clear();
                continue;
            }
            let mut enclosing = stack.last().map(|o| o.1);
            if matches!(token, T::Lpar | T::Lsqb | T::Lbrace) {
                stack.push((token, at));
            } else if matches!(token, T::Rpar | T::Rsqb | T::Rbrace) {
                match stack.pop() {
                    Some((open, o)) if matches!((open, token), (T::Lpar, T::Rpar) | (T::Lsqb, T::Rsqb) | (T::Lbrace, T::Rbrace)) => {
                        self.mate[o] = at as u32;
                        self.mate[at] = o as u32;
                        enclosing = stack.last().map(|o| o.1);
                    }
                    _ => {
                        stack.clear();
                        enclosing = None;
                    }
                }
            }
            if let Some(e) = enclosing {
                self.encl[at] = e as u32;
            }
        }
        true
    }
}

/// `token-link` at one token (facts): the rows whose source (column 0) or target (column 1) is the token with view id `key`.
fn token_link_key(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, col: u8, key: u32, links: &mut Links, flag: &mut u8) {
    let Some(view) = c.view else { return };
    let Some(leaf) = (key as usize).checked_sub(view.n_nodes) else { return };
    let Some(&t) = view.leaf_tok.get(leaf) else { return };
    if t == NO_TOKEN {
        return;
    }
    once(flag, || links.build(c));
    let tokens = c.parsed.tokens();
    let t = t as usize;
    if !links.live[t] {
        return;
    }
    let mut row = |s: usize, d: usize, k: &'static str| {
        let st = &tokens[s];
        sink.row(rel, &[Cell::Token(s as u32), Cell::Token(d as u32), Cell::Word(k), Cell::File, Cell::Span(st.start().to_u32(), st.end().to_u32())]);
    };
    if col == 0 {
        if t + 1 < tokens.len() && links.live[t + 1] {
            row(t, t + 1, "next-token");
        }
        let m = links.mate[t];
        if m != NO_TOKEN && m as usize > t {
            row(t, m as usize, "matching-delimiter");
        }
        if links.encl[t] != NO_TOKEN {
            row(t, links.encl[t] as usize, "enclosing-delimiter");
        }
    } else {
        if t > 0 && links.live[t - 1] {
            row(t - 1, t, "next-token");
        }
        let m = links.mate[t];
        if m != NO_TOKEN && (m as usize) < t {
            row(m as usize, t, "matching-delimiter");
        }
        // the tokens directly inside the bracket `t`: up to its closer, or to the end when nothing closes it
        use ruff_python_ast::token::TokenKind as T;
        if matches!(tokens[t].kind(), T::Lpar | T::Lsqb | T::Lbrace) {
            let end = if m != NO_TOKEN { m as usize } else { tokens.len() };
            for x in t + 1..end {
                if links.encl[x] == t as u32 {
                    row(x, t, "enclosing-delimiter");
                }
            }
        }
    }
}

/// The per-key form (facts): the rows of relation `di` whose column `col` holds the node (or token) `key`.
pub(crate) fn produce_key(c: &Ctx<'_, '_>, di: usize, col: u8, key: u32, sink: &mut dyn Sink, buf: &mut String, st: &mut State) {
    produce_key_inner(c, di, col, key, sink, buf, st);
    st.clock();
}

/// `expression-shape` by class (column 1): the members of the class `key` names, from a chain per class built in one pass over
/// the class array on the first such probe in the file.
fn shape_members(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, key: u32, st: &mut State) {
    if !once(&mut st.built[0], || st.shapes.build(c)) {
        return;
    }
    let n = c.table.len();
    if !st.class_built {
        let t = crate::nav::Tick::now();
        st.class_head.clear();
        st.class_head.resize(n, u32::MAX);
        st.class_next.clear();
        st.class_next.resize(n, u32::MAX);
        for id in (0..n).rev() {
            let class = st.shapes.class[id];
            if class != u32::MAX && (class as usize) < n && is_expression(c.table.node[id]) {
                st.class_next[id] = st.class_head[class as usize];
                st.class_head[class as usize] = id as u32;
            }
        }
        st.class_built = true;
        BUILD_NS.with(|b| b.set(b.get() + t.elapsed().as_nanos() as u64));
    }
    if (key as usize) >= n {
        return;
    }
    let mut id = st.class_head[key as usize];
    while id != u32::MAX {
        sink.row(rel, &[Cell::Node(id), Cell::Node(key), Cell::File, c.span(id)]);
        id = st.class_next[id as usize];
    }
}

fn produce_key_inner(c: &Ctx<'_, '_>, di: usize, col: u8, key: u32, sink: &mut dyn Sink, buf: &mut String, st: &mut State) {
    let only = Some(key);
    match RELATIONS[di] {
        "token-link" => token_link_key(c, sink, di, col, key, &mut st.links, &mut st.built[4]),
        "literal-properties" => rows_literal_properties(c, sink, di, only),
        "string-literal" => rows_string_literal(c, sink, di, buf, only),
        "static-import" => rows_static_import(c, sink, di, buf, only),
        "numeric-literal" => rows_numeric_literal(c, sink, di, buf, only),
        "call-argument" if col == 0 => rows_call_argument(c, sink, di, only),
        "call-argument" => {
            // column 2, the argument: its call is above it (behind a keyword), and the row is the call's row for it
            if (key as usize) >= c.table.len() {
                return;
            }
            let mut up = c.table.parent[key as usize];
            if up != u32::MAX && matches!(c.table.node[up as usize], AnyNodeRef::Keyword(_)) {
                up = c.table.parent[up as usize];
            }
            if up == u32::MAX || !matches!(c.table.node[up as usize], AnyNodeRef::Arguments(_)) {
                return;
            }
            let call = c.table.parent[up as usize];
            if call == u32::MAX {
                return;
            }
            call_arguments(c, call, &mut |position, e, passes| {
                if e == key {
                    sink.row(di, &[Cell::Node(call), Cell::Num(i64::from(position)), Cell::Node(e), passes.map_or(Cell::Null, Cell::Text), Cell::File, c.span(e)]);
                }
            });
        }
        "function" => rows_function(c, sink, di, only),
        "parameter" => rows_parameter(c, sink, di, only),
        "decorates" if col == 0 => rows_decorates(c, sink, di, buf, only),
        "decorates" => {
            // column 1, the decorated definition: its decorators
            if (key as usize) >= c.table.len() {
                return;
            }
            let list: &[ast::Decorator] = match c.table.node[key as usize] {
                AnyNodeRef::StmtFunctionDef(f) => &f.decorator_list,
                AnyNodeRef::StmtClassDef(k) => &k.decorator_list,
                _ => return,
            };
            for d in list {
                rows_decorates(c, sink, di, buf, Some(id_of(d.into())));
            }
        }
        "bytes-literal" => rows_bytes_literal(c, sink, di, buf, only),
        "expression-shape" if col == 0 => rows_expression_shape(c, sink, di, &mut st.shapes, &mut st.built[0], only),
        "expression-shape" => shape_members(c, sink, di, key, st),
        "expression-type" => rows_expression_type(c, sink, di, &mut st.types, &mut st.built[1], only),
        "function-terminal" => rows_function_terminal(c, sink, di, &mut st.terminals, &mut st.built[2], only),
        "constant-key" => rows_constant_key(c, sink, di, &mut st.constant_keys, &mut st.built[3], only),
        "format-parse" => rows_format(c, sink, Some(di), None, buf, st, only),
        "format-field" => rows_format(c, sink, None, Some(di), buf, st, only),
        "brace-format-parse" => rows_brace_format(c, sink, Some(di), None, buf, only),
        "brace-format-field" => rows_brace_format(c, sink, None, Some(di), buf, only),
        "literal-expression-parse" => rows_literal_expression(c, sink, di, buf, st, only),
        "raw-format-segment" => rows_raw_format(c, sink, Some(di), None, st, only),
        "raw-format-field" => rows_raw_format(c, sink, None, Some(di), st, only),
        _ => {}
    }
}

// ------------------------------------------------------------------------------------------------ format-parse, format-field
/// The provider's percent-format grammar (`outbound/format_syntax/adapters/ruff_literal_cformat`, itself adapted from Ruff 0.16.5's
/// `ruff_python_literal::cformat`, MIT), copied because the adapter is crate-private: the conversion specifications of a decoded
/// string, Python's `str % args` mini-language.
pub(crate) mod cformat {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum Quantity {
        None,
        Fixed,
        Star,
        Dot,
    }
    impl Quantity {
        pub fn word(self) -> &'static str {
            match self {
                Quantity::None => "none",
                Quantity::Fixed => "fixed",
                Quantity::Star => "star",
                Quantity::Dot => "dot",
            }
        }
    }
    #[derive(Clone)]
    pub struct Field {
        pub ordinal: usize,
        pub key: Option<String>,
        pub conversion: char,
        pub width: Quantity,
        pub precision: Quantity,
        pub flags: u8,
    }
    #[derive(Clone, Copy, Default)]
    pub struct Summary {
        pub positional: usize,
        pub mapping: bool,
        pub dynamic_width: bool,
        pub dynamic_precision: bool,
    }
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum ErrorKind {
        UnmatchedKeyParentheses,
        IncompleteFormat,
        UnsupportedFormatCharacter,
        WidthOrPrecisionTooLarge,
    }
    impl ErrorKind {
        pub fn word(self) -> &'static str {
            match self {
                ErrorKind::UnmatchedKeyParentheses => "unmatched-key-parentheses",
                ErrorKind::IncompleteFormat => "incomplete-format",
                ErrorKind::UnsupportedFormatCharacter => "unsupported-format-character",
                ErrorKind::WidthOrPrecisionTooLarge => "width-or-precision-too-large",
            }
        }
    }
    #[derive(Clone, Copy)]
    pub struct Error {
        pub kind: ErrorKind,
        pub character: Option<char>,
        pub index: usize,
    }
    struct Cursor<'a> {
        chars: std::iter::Enumerate<std::str::Chars<'a>>,
        peeked: Option<(usize, char)>,
    }
    impl Cursor<'_> {
        fn peek(&mut self) -> Option<(usize, char)> {
            if self.peeked.is_none() {
                self.peeked = self.chars.next();
            }
            self.peeked
        }
        fn advance(&mut self) -> Option<(usize, char)> {
            self.peeked.take().or_else(|| self.chars.next())
        }
    }
    fn err(kind: ErrorKind, index: usize, character: Option<char>) -> Error {
        Error { kind, character, index }
    }
    fn mapping_key(c: &mut Cursor<'_>) -> Result<Option<String>, Error> {
        let Some(open) = c.peek() else { return Ok(None) };
        if open.1 != '(' {
            return Ok(None);
        }
        c.advance();
        let mut depth = 1usize;
        let mut key = String::new();
        while let Some((_, ch)) = c.advance() {
            match ch {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                return Ok(Some(key));
            }
            key.push(ch);
        }
        Err(err(ErrorKind::UnmatchedKeyParentheses, open.0, None))
    }
    fn quantity(c: &mut Cursor<'_>) -> Result<Quantity, Error> {
        match c.peek() {
            Some((_, '*')) => {
                c.advance();
                Ok(Quantity::Star)
            }
            Some((_, d)) if d.is_ascii_digit() => {
                let mut amount = 0i32;
                while let Some((i, d)) = c.peek().filter(|x| x.1.is_ascii_digit()) {
                    let digit = d.to_digit(10).unwrap_or(0) as i32;
                    amount = amount.checked_mul(10).and_then(|n| n.checked_add(digit)).ok_or_else(|| err(ErrorKind::WidthOrPrecisionTooLarge, i, None))?;
                    c.advance();
                }
                Ok(Quantity::Fixed)
            }
            _ => Ok(Quantity::None),
        }
    }
    fn field(c: &mut Cursor<'_>, ordinal: usize) -> Result<Field, Error> {
        let key = mapping_key(c)?;
        let mut flags = 0u8;
        while let Some((_, ch)) = c.peek() {
            let bit = match ch {
                ' ' => 1,
                '#' => 2,
                '+' => 4,
                '-' => 8,
                '0' => 16,
                _ => break,
            };
            flags |= bit;
            c.advance();
        }
        let width = quantity(c)?;
        let precision = if c.peek().is_some_and(|x| x.1 == '.') {
            c.advance();
            match quantity(c)? {
                Quantity::None => Quantity::Dot,
                m => m,
            }
        } else {
            Quantity::None
        };
        if c.peek().is_some_and(|x| matches!(x.1, 'h' | 'l' | 'L')) {
            c.advance();
        }
        let Some((ci, conversion)) = c.advance() else {
            let index = c.peek().map_or(0, |x| x.0);
            return Err(err(ErrorKind::IncompleteFormat, index, None));
        };
        if !matches!(conversion, 'd' | 'i' | 'u' | 'o' | 'x' | 'X' | 'e' | 'E' | 'f' | 'F' | 'g' | 'G' | 'c' | 'r' | 's' | 'a') {
            return Err(err(ErrorKind::UnsupportedFormatCharacter, ci, Some(conversion)));
        }
        Ok(Field { ordinal, key, conversion, width, precision, flags })
    }
    pub fn parse(input: &str, fields: &mut Vec<Field>) -> Result<Summary, Error> {
        fields.clear();
        if !input.as_bytes().contains(&b'%') {
            // no conversion specification can start: the parse reads every character and finds none
            return Ok(Summary::default());
        }
        let mut c = Cursor { chars: input.chars().enumerate(), peeked: None };
        let mut summary = Summary::default();
        while let Some((i, ch)) = c.advance() {
            if ch != '%' {
                continue;
            }
            let Some(next) = c.peek() else {
                return Err(err(ErrorKind::IncompleteFormat, i + 1, None));
            };
            if next.1 == '%' {
                c.advance();
                continue;
            }
            let f = field(&mut c, fields.len())?;
            if f.key.is_some() {
                summary.mapping = true;
            } else {
                summary.positional += 1;
            }
            if f.width == Quantity::Star {
                summary.positional += 1;
                summary.dynamic_width = true;
            }
            if f.precision == Quantity::Star {
                summary.positional += 1;
                summary.dynamic_precision = true;
            }
            fields.push(f);
        }
        Ok(summary)
    }
}

/// `format_parsing::decoded_into`: the decoded value of a string occurrence (a string, or an f-string with no interpolation);
/// None when it interpolates (the input is unknown).
pub(crate) fn format_input<'n>(node: AnyNodeRef<'n>, buf: &'n mut String) -> Option<Option<&'n str>> {
    match node {
        AnyNodeRef::ExprStringLiteral(_) | AnyNodeRef::ExprFString(_) => Some(string_value(node, buf)),
        _ => None,
    }
}

fn rows_format(c: &Ctx<'_, '_>, sink: &mut dyn Sink, parse_rel: Option<usize>, field_rel: Option<usize>, buf: &mut String, st: &mut State, only: Option<u32>) {
    let fields = &mut st.cfields;
    for k in [NodeKind::ExprStringLiteral, NodeKind::ExprFString] {
        for &id in ids(c, k, &only) {
            let Some(input) = format_input(c.table.node[id as usize], buf) else { continue };
            let head = [Cell::Node(id), Cell::Text("percent-string"), Cell::Text("decoded-string")];
            let tail = |sink: &mut dyn Sink, outcome: &str, cat: &str, ch: Option<char>, idx: usize, s: cformat::Summary| {
                if let Some(r) = parse_rel {
                    let mut b = [0u8; 4];
                    let ch = ch.map_or("", |x| &*x.encode_utf8(&mut b));
                    sink.row(r, &[head[0], head[1], head[2], Cell::Text(outcome), Cell::Text(cat), Cell::Text(ch), Cell::Num(idx as i64), Cell::Num(s.positional as i64), Cell::Flag(s.mapping), Cell::Flag(s.dynamic_width), Cell::Flag(s.dynamic_precision), Cell::File, c.span(id)]);
                }
            };
            match input {
                None => tail(sink, "unknown-input", "none", None, 0, cformat::Summary::default()),
                Some(text) => match cformat::parse(text, fields) {
                    Ok(summary) => {
                        tail(sink, "parsed", "none", None, 0, summary);
                        if let Some(r) = field_rel {
                            for f in fields.iter() {
                                let mut b = [0u8; 4];
                                sink.row(r, &[Cell::Node(id), Cell::Num(f.ordinal as i64), Cell::Text(if f.key.is_some() { "mapping" } else { "positional" }), Cell::Text(f.key.as_deref().unwrap_or("")), Cell::Text(f.conversion.encode_utf8(&mut b)), Cell::Text(f.width.word()), Cell::Text(f.precision.word()), Cell::File, c.span(id)]);
                            }
                        }
                    }
                    Err(e) => tail(sink, "malformed", e.kind.word(), e.character, e.index, cformat::Summary::default()),
                },
            }
        }
    }
}

// ------------------------------------------------------------------------------------------------ brace-format-parse, brace-format-field
/// The provider's brace-format grammar (`outbound/brace_format/adapters/ruff_literal_braceformat`, adapted from Ruff 0.16.5's
/// `ruff_python_literal::format`, MIT), copied because the adapter is crate-private: the replacement fields of `str.format`.
pub(crate) mod braceformat {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum Error {
        UnmatchedBracket,
        MissingStartBracket,
        PlaceholderRecursionExceeded,
        UnknownConversion,
        EmptyAttribute,
        MissingRightBracket,
        InvalidCharacterAfterRightBracket,
    }
    impl Error {
        pub fn word(self) -> &'static str {
            match self {
                Error::UnmatchedBracket => "unmatched-bracket",
                Error::MissingStartBracket => "missing-start-bracket",
                Error::PlaceholderRecursionExceeded => "placeholder-recursion-exceeded",
                Error::UnknownConversion => "unknown-conversion",
                Error::EmptyAttribute => "empty-attribute",
                Error::MissingRightBracket => "missing-right-bracket",
                Error::InvalidCharacterAfterRightBracket => "invalid-character-after-right-bracket",
            }
        }
    }
    pub enum Selector {
        Automatic(usize),
        Index(usize, String),
        Keyword(String),
    }
    pub struct Field {
        pub ordinal: usize,
        pub parent: Option<usize>,
        pub selector: Selector,
        pub spec: Option<String>,
        pub selector_text: String,
        pub conversion: Option<char>,
    }
    #[derive(Default)]
    pub struct Parsed {
        pub fields: Vec<Field>,
        pub automatic: usize,
        pub indexed: usize,
        pub keyword: usize,
        pub nested: usize,
    }
    enum FieldType {
        Automatic,
        Index(usize, String),
        Keyword(String),
    }
    struct Part {
        name: String,
        spec: Option<String>,
        conversion: Option<char>,
    }
    fn field_name(name: &str) -> Result<FieldType, Error> {
        let chars: Vec<char> = name.chars().collect();
        let root_end = chars.iter().position(|c| matches!(c, '.' | '[')).unwrap_or(chars.len());
        let root: String = chars[..root_end].iter().collect();
        let t = if root.is_empty() {
            FieldType::Automatic
        } else if let Ok(i) = root.parse::<usize>() {
            FieldType::Index(i, root)
        } else {
            FieldType::Keyword(root)
        };
        let mut at = root_end;
        while let Some(c) = chars.get(at) {
            match c {
                '.' => {
                    at += 1;
                    let start = at;
                    while chars.get(at).is_some_and(|n| !matches!(n, '.' | '[')) {
                        at += 1;
                    }
                    if start == at {
                        return Err(Error::EmptyAttribute);
                    }
                }
                '[' => {
                    at += 1;
                    let start = at;
                    while chars.get(at).is_some_and(|n| *n != ']') {
                        at += 1;
                    }
                    if at == chars.len() {
                        return Err(Error::MissingRightBracket);
                    }
                    if start == at {
                        return Err(Error::EmptyAttribute);
                    }
                    at += 1;
                    if chars.get(at).is_some_and(|n| !matches!(n, '.' | '[')) {
                        return Err(Error::InvalidCharacterAfterRightBracket);
                    }
                }
                _ => return Err(Error::InvalidCharacterAfterRightBracket),
            }
        }
        Ok(t)
    }
    fn part(body: &str) -> Result<Part, Error> {
        let mut split = body.splitn(2, ':');
        let field = split.next().unwrap_or_default();
        let spec = split.next().map(str::to_owned);
        let mut parts = field.splitn(2, '!');
        let name = parts.next().unwrap_or_default();
        let mut conversion = None;
        if let Some(text) = parts.next() {
            let mut cs = text.chars();
            conversion = Some(cs.next().ok_or(Error::UnknownConversion)?);
            if cs.next().is_some() {
                return Err(Error::UnknownConversion);
            }
        }
        field_name(name)?;
        Ok(Part { name: name.to_owned(), spec, conversion })
    }
    fn parts(text: &str) -> Result<Vec<Part>, Error> {
        let chars: Vec<char> = text.chars().collect();
        let mut out = Vec::new();
        let mut at = 0;
        let mut pending_escape = false;
        while let Some(c) = chars.get(at).copied() {
            if pending_escape && c == 'N' && chars.get(at + 1) == Some(&'{') {
                if let Some(rel) = chars[at + 2..].iter().position(|n| *n == '}') {
                    at += rel + 3;
                    pending_escape = false;
                    continue;
                }
            }
            if c == '{' && chars.get(at + 1) == Some(&'{') {
                at += 2;
                pending_escape = false;
                continue;
            }
            if c == '}' && chars.get(at + 1) == Some(&'}') {
                at += 2;
                pending_escape = false;
                continue;
            }
            if c == '}' {
                return Err(Error::MissingStartBracket);
            }
            if c != '{' {
                pending_escape = c == '\\' && !pending_escape;
                at += 1;
                continue;
            }
            at += 1;
            let start = at;
            let mut nested = false;
            while let Some(m) = chars.get(at) {
                match m {
                    '{' if nested => return Err(Error::PlaceholderRecursionExceeded),
                    '{' => {
                        nested = true;
                        at += 1;
                    }
                    '}' if nested => {
                        nested = false;
                        at += 1;
                    }
                    '}' => break,
                    _ => at += 1,
                }
            }
            if at == chars.len() {
                return Err(Error::UnmatchedBracket);
            }
            let body: String = chars[start..at].iter().collect();
            at += 1;
            out.push(part(&body)?);
            pending_escape = false;
        }
        Ok(out)
    }
    fn add(p: &mut Parsed, name: &str, parent: Option<usize>, spec: Option<String>, conversion: Option<char>) -> Result<usize, Error> {
        let ordinal = p.fields.len();
        let selector = match field_name(name)? {
            FieldType::Automatic => {
                let i = p.automatic;
                p.automatic += 1;
                Selector::Automatic(i)
            }
            FieldType::Index(i, t) => {
                p.indexed += 1;
                Selector::Index(i, t)
            }
            FieldType::Keyword(n) => {
                p.keyword += 1;
                Selector::Keyword(n)
            }
        };
        p.fields.push(Field { ordinal, parent, selector, spec, selector_text: name.to_owned(), conversion });
        Ok(ordinal)
    }
    pub fn parse(input: &str) -> Result<Parsed, Error> {
        if !input.as_bytes().iter().any(|b| matches!(b, b'{' | b'}')) {
            // no brace: no field, no escape and no error can occur
            return Ok(Parsed::default());
        }
        let ps = parts(input)?;
        let mut p = Parsed::default();
        for part in ps {
            let parent = add(&mut p, &part.name, None, part.spec.clone(), part.conversion)?;
            let spec = part.spec.unwrap_or_default();
            for nested in parts(&spec)? {
                add(&mut p, &nested.name, Some(parent), nested.spec, nested.conversion)?;
                p.nested += 1;
            }
        }
        Ok(p)
    }
}

fn rows_brace_format(c: &Ctx<'_, '_>, sink: &mut dyn Sink, parse_rel: Option<usize>, field_rel: Option<usize>, buf: &mut String, only: Option<u32>) {
    for k in [NodeKind::ExprStringLiteral, NodeKind::ExprFString, NodeKind::InterpolatedStringFormatSpec] {
        for &id in ids(c, k, &only) {
            let node = c.table.node[id as usize];
            let input: Option<&str> = match node {
                AnyNodeRef::InterpolatedStringFormatSpec(spec) => {
                    buf.clear();
                    let mut dynamic = false;
                    for e in spec.elements.iter() {
                        match e {
                            ast::InterpolatedStringElement::Literal(l) => buf.push_str(&l.value),
                            ast::InterpolatedStringElement::Interpolation(_) => dynamic = true,
                        }
                    }
                    if dynamic { None } else { Some(buf.as_str()) }
                }
                _ => string_value(node, buf),
            };
            let row = |sink: &mut dyn Sink, outcome: &str, err: &str, p: Option<&braceformat::Parsed>| {
                if let Some(r) = parse_rel {
                    let n = |f: fn(&braceformat::Parsed) -> usize| Cell::Num(p.map_or(0, f) as i64);
                    sink.row(r, &[Cell::Node(id), Cell::Text("decoded-string"), Cell::Text(outcome), Cell::Text(err), n(|p| p.automatic), n(|p| p.indexed), n(|p| p.keyword), n(|p| p.nested), Cell::File, c.span(id)]);
                }
            };
            match input {
                None => row(sink, "unknown-input", "none", None),
                Some(text) => match braceformat::parse(text) {
                    Err(e) => row(sink, "malformed", e.word(), None),
                    Ok(p) => {
                        row(sink, "parsed", "none", Some(&p));
                        if let Some(r) = field_rel {
                            for f in &p.fields {
                                let (kind, index, name) = match &f.selector {
                                    braceformat::Selector::Automatic(i) => ("automatic", *i as i64, ""),
                                    braceformat::Selector::Index(i, t) => ("index", i64::try_from(*i).unwrap_or(0), t.as_str()),
                                    braceformat::Selector::Keyword(n) => ("keyword", 0, n.as_str()),
                                };
                                sink.row(r, &[Cell::Node(id), Cell::Num(f.ordinal as i64), Cell::Text(if f.parent.is_some() { "format-spec" } else { "literal" }), Cell::Num(f.parent.map_or(-1, |p| p as i64)), Cell::Text(kind), Cell::Num(index), Cell::Flag(false), Cell::Text(name), Cell::File, c.span(id)]);
                            }
                        }
                    }
                },
            }
        }
    }
}

// ------------------------------------------------------------------------------------------------ literal-expression-parse
/// `literal_expression_parsing` + the `ruff_string_annotation` adapter: whether a string literal's value parses as a Python
/// expression the way a string annotation is read: its string tokens as written, one per line, in parentheses, parsed by the
/// parser's own string-annotation entry point; memoised per distinct carrier text within the file.
fn raw_body(token: &str) -> Option<&str> {
    let b = token.as_bytes();
    let prefix = usize::from(matches!(b.first(), Some(b'r' | b'R' | b'u' | b'U')));
    let quoted = token.get(prefix..)?;
    let quote = *quoted.as_bytes().first()?;
    if !matches!(quote, b'\'' | b'"') {
        return None;
    }
    let width = if quoted.as_bytes().get(..3).is_some_and(|f| f.iter().all(|x| *x == quote)) { 3 } else { 1 };
    let end = quoted.len().checked_sub(width)?;
    if end < width || !quoted.as_bytes().get(end..)?.iter().all(|x| *x == quote) {
        return None;
    }
    quoted.get(width..end)
}

pub(crate) fn literal_expression(c: &Ctx<'_, '_>, id: u32, carrier: &mut String, memo: &mut crate::sema::FxMap<u64, &'static str>) -> Option<&'static str> {
    let AnyNodeRef::ExprStringLiteral(s) = c.table.node[id as usize] else { return None };
    carrier.clear();
    carrier.push_str("(\n");
    for part in s.value.iter() {
        let text = &c.source[part.range.start().to_usize()..part.range.end().to_usize()];
        if raw_body(text).is_none() {
            return Some("unavailable");
        }
        carrier.push_str(text);
        carrier.push('\n');
    }
    carrier.push(')');
    let key = crate::sema::hash_text(carrier);
    if let Some(&o) = memo.get(&key) {
        return Some(o);
    }
    let outcome = match ruff_python_parser::parse_expression(carrier) {
        Ok(parsed) => match parsed.expr() {
            Expr::StringLiteral(literal) => {
                if ruff_python_parser::typing::parse_type_annotation(literal, carrier).is_ok() { "parsed" } else { "malformed" }
            }
            _ => "unavailable",
        },
        Err(_) => "unavailable",
    };
    memo.insert(key, outcome);
    Some(outcome)
}

fn rows_literal_expression(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, buf: &mut String, st: &mut State, only: Option<u32>) {
    if only.is_none() {
        // the memo is by carrier text (a pure function of it): cleared per whole fill only to bound its size
        st.carriers.clear();
    }
    for &id in ids(c, NodeKind::ExprStringLiteral, &only) {
        if let Some(o) = literal_expression(c, id, buf, &mut st.carriers) {
            sink.row(rel, &[Cell::Node(id), Cell::Text("python-string-annotation"), Cell::Text(o), Cell::File, c.span(id)]);
        }
    }
}

// ------------------------------------------------------------------------------------------------ raw-format-segment, raw-format-field
/// `raw_format`: each string token of a string literal parsed as a percent format as written (its body between the quotes, not
/// decoded), one segment per token; its fields numbered across the literal's segments, only when every segment parsed.
const FLAG_SPELLINGS: [&str; 32] = [
    "", " ", "#", " #", "+", " +", "#+", " #+", "-", " -", "#-", " #-", "+-", " +-", "#+-", " #+-", "0", " 0", "#0", " #0", "+0", " +0",
    "#+0", " #+0", "-0", " -0", "#-0", " #-0", "+-0", " +-0", "#+-0", " #+-0",
];

fn rows_raw_format(c: &Ctx<'_, '_>, sink: &mut dyn Sink, seg_rel: Option<usize>, field_rel: Option<usize>, st: &mut State, only: Option<u32>) {
    let mut fields: Vec<cformat::Field> = Vec::new();
    let mut parsed: Vec<(u32, u32, Result<cformat::Summary, Option<cformat::Error>>, Vec<cformat::Field>)> = Vec::new();
    let _ = st;
    for &id in ids(c, NodeKind::ExprStringLiteral, &only) {
        let AnyNodeRef::ExprStringLiteral(s) = c.table.node[id as usize] else { continue };
        parsed.clear();
        let mut complete = true;
        for part in s.value.iter() {
            let (ps, pe) = (part.range.start().to_u32(), part.range.end().to_u32());
            let r = match raw_body(&c.source[ps as usize..pe as usize]) {
                None => {
                    complete = false;
                    Err(None)
                }
                Some(body) => cformat::parse(body, &mut fields).map_err(Some),
            };
            parsed.push((ps, pe, r, fields.clone()));
        }
        if parsed.is_empty() {
            complete = false;
        }
        let mut first_field = 0usize;
        for (ordinal, (ps, pe, r, fs)) in parsed.iter().enumerate() {
            let seg = match token_at(c, *ps, *pe) {
                Some(i) if c.view.is_some() => Cell::Token(i),
                _ => Cell::Key("string", *ps, *pe),
            };
            if let Some(rel) = seg_rel {
                let (outcome, cat, ch, idx, sm) = match r {
                    Ok(sm) => ("parsed", "none", None, 0, *sm),
                    Err(Some(e)) => ("malformed", e.kind.word(), e.character, e.index, cformat::Summary::default()),
                    Err(None) => ("unavailable", "none", None, 0, cformat::Summary::default()),
                };
                let mut b = [0u8; 4];
                let ch = ch.map_or("", |x: char| &*x.encode_utf8(&mut b));
                sink.row(rel, &[Cell::Node(id), seg, Cell::Num(ordinal as i64), Cell::Text("percent-string"), Cell::Text(outcome), Cell::Text(cat), Cell::Text(ch), Cell::Num(idx as i64), Cell::Num(sm.positional as i64), Cell::Flag(sm.mapping), Cell::Flag(sm.dynamic_width), Cell::Flag(sm.dynamic_precision), Cell::File, Cell::Span(*ps, *pe)]);
            }
            if let (Some(rel), true, Ok(_)) = (field_rel, complete, r) {
                for f in fs {
                    let mut b = [0u8; 4];
                    sink.row(rel, &[Cell::Node(id), seg, Cell::Num(ordinal as i64), Cell::Num((first_field + f.ordinal) as i64), Cell::Num(f.ordinal as i64), Cell::Text(if f.key.is_some() { "mapping" } else { "positional" }), Cell::Text(f.key.as_deref().unwrap_or("")), Cell::Text(f.conversion.encode_utf8(&mut b)), Cell::Text(f.width.word()), Cell::Text(f.precision.word()), Cell::File, Cell::Span(*ps, *pe), Cell::Text(FLAG_SPELLINGS[f.flags as usize & 31])]);
                }
            }
            if r.is_ok() {
                first_field += fs.len();
            }
        }
    }
}

// ------------------------------------------------------------------------------------------------ format-specification
/// `format_specification::observe` + `standard_format_spec::recognize`: every format spec, of a `str.format` field in a string
/// (from the brace parse) or of an f-string interpolation (its own syntax), recognised against Python's standard format-spec
/// mini-language for the target version.
#[derive(Clone, Copy)]
enum SpecOutcome {
    Recognized,
    Dynamic,
    InvalidType(char),
    OtherError,
}

fn is_decimal(c: char) -> bool {
    use unicode_properties::{GeneralCategory, UnicodeGeneralCategory};
    c.general_category() == GeneralCategory::DecimalNumber
}

fn recognize(raw: &str, nested: bool, target: (i64, i64)) -> SpecOutcome {
    if nested {
        return SpecOutcome::Dynamic;
    }
    let align = |c: char| matches!(c, '<' | '>' | '=' | '^');
    let mut chars = raw.chars().peekable();
    let mut look = chars.clone();
    let (first, second) = (look.next(), look.next());
    if second.is_some_and(align) {
        chars.next();
        chars.next();
    } else if first.is_some_and(align) {
        chars.next();
    }
    if chars.peek().is_some_and(|c| matches!(*c, '+' | '-' | ' ')) {
        chars.next();
    }
    if target >= (3, 11) && chars.peek() == Some(&'z') {
        chars.next();
    }
    if chars.peek() == Some(&'#') {
        chars.next();
    }
    if chars.peek() == Some(&'0') {
        chars.next();
    }
    while chars.peek().is_some_and(|c| is_decimal(*c)) {
        chars.next();
    }
    if chars.peek().is_some_and(|c| matches!(*c, ',' | '_')) {
        chars.next();
    }
    if chars.peek() == Some(&'.') {
        chars.next();
        if !chars.peek().is_some_and(|c| is_decimal(*c)) {
            return SpecOutcome::OtherError;
        }
        while chars.peek().is_some_and(|c| is_decimal(*c)) {
            chars.next();
        }
    }
    let Some(p) = chars.next() else { return SpecOutcome::Recognized };
    if !matches!(p, 's' | 'b' | 'c' | 'd' | 'o' | 'x' | 'X' | 'n' | 'e' | 'E' | 'f' | 'F' | 'g' | 'G' | '%') {
        return SpecOutcome::InvalidType(p);
    }
    if chars.next().is_some() { SpecOutcome::OtherError } else { SpecOutcome::Recognized }
}

fn rows_format_specification(c: &Ctx<'_, '_>, sink: &mut dyn Sink, rel: usize, buf: &mut String, only: Option<u32>) {
    let emit = |sink: &mut dyn Sink, id: u32, ordinal: usize, parent: Option<usize>, origin: u32, kind: &str, value: Option<&str>, o: SpecOutcome, selector: &str, conversion: &str| {
        let mut b = [0u8; 4];
        let (label, invalid) = match o {
            SpecOutcome::Recognized => ("recognized", ""),
            SpecOutcome::Dynamic => ("dynamic", ""),
            SpecOutcome::InvalidType(ch) => ("invalid-type", &*ch.encode_utf8(&mut b)),
            SpecOutcome::OtherError => ("other-error", ""),
        };
        sink.row(rel, &[Cell::Node(id), Cell::Num(ordinal as i64), Cell::Num(parent.map_or(-1, |p| p as i64)), Cell::Node(origin), Cell::Text(kind), value.map_or(Cell::Null, Cell::Text), Cell::Text(label), Cell::Text(invalid), Cell::Text(selector), Cell::Text(conversion), Cell::File, c.span(origin)]);
    };
    // brace strings: string literals whose brace parse succeeded
    for &id in ids(c, NodeKind::ExprStringLiteral, &only) {
        let Some(text) = string_value(c.table.node[id as usize], buf) else { continue };
        let Ok(p) = braceformat::parse(text) else { continue };
        for f in &p.fields {
            let Some(spec) = &f.spec else { continue };
            let nested = p.fields.iter().any(|n| n.parent == Some(f.ordinal));
            let mut cb = [0u8; 4];
            let conv = f.conversion.map_or("", |ch| &*ch.encode_utf8(&mut cb));
            emit(sink, id, f.ordinal, f.parent, id, "brace-string", Some(spec), recognize(spec, nested, TARGET), &f.selector_text, conv);
        }
    }
    // f-strings: the interpolations of each f-string (not of a spec), numbered in order; a spec's parent is the enclosing one
    for &owner in c.kind(NodeKind::ExprFString) {
        let (start, end) = (owner as usize + 1, c.table.end[owner as usize] as usize);
        let owner_of = |mut x: u32| -> u32 {
            loop {
                x = c.table.parent[x as usize];
                if matches!(c.table.node[x as usize], AnyNodeRef::ExprFString(_)) || x == u32::MAX || x == 0 {
                    return x;
                }
            }
        };
        let group: Vec<u32> = (start..end)
            .map(|i| i as u32)
            .filter(|&i| matches!(c.table.node[i as usize], AnyNodeRef::InterpolatedElement(_)) && owner_of(i) == owner)
            .collect();
        for (ordinal, &field) in group.iter().enumerate() {
            let AnyNodeRef::InterpolatedElement(ie) = c.table.node[field as usize] else { continue };
            let Some(spec) = &ie.format_spec else { continue };
            let spec_id = id_of((&**spec).into());
            let mut above = c.table.parent[field as usize];
            let parent = loop {
                if above == owner || above == u32::MAX {
                    break None;
                }
                if let Some(found) = group.iter().position(|g| *g == above) {
                    break Some(found);
                }
                above = c.table.parent[above as usize];
            };
            buf.clear();
            let mut dynamic = false;
            for e in spec.elements.iter() {
                match e {
                    ast::InterpolatedStringElement::Literal(l) => buf.push_str(&l.value),
                    ast::InterpolatedStringElement::Interpolation(_) => dynamic = true,
                }
            }
            let mut cb = [0u8; 4];
            let conv = ie.conversion.to_char().map_or("", |ch| &*ch.encode_utf8(&mut cb));
            if dynamic {
                emit(sink, owner, ordinal, parent, spec_id, "fstring", None, SpecOutcome::Dynamic, "", conv);
            } else {
                let o = recognize(buf, false, TARGET);
                emit(sink, owner, ordinal, parent, spec_id, "fstring", Some(buf.as_str()), o, "", conv);
            }
        }
    }
}

// ------------------------------------------------------------------------------------------------ driver
/// Reusable per-worker state of the derived analyses.
#[derive(Default)]
pub(crate) struct State {
    pub shapes: Shapes,
    pub types: Types,
    pub view: crate::view::View,
    pub sym: crate::symbols::Symbols,
    pub view_file: Option<usize>,
    pub kind_names: Vec<&'static str>,
    pub terminals: Terminals,
    pub constant_keys: ConstantKeys,
    pub cfields: Vec<cformat::Field>,
    pub carriers: crate::sema::FxMap<u64, &'static str>,
    /// facts: the whole-file analyses built for the current file (shapes, types, terminals, constant keys, token links)
    pub built: [u8; 5],
    pub links: Links,
    /// time in whole-file analyses: over the run, and the running total a caller compares before and after a call
    pub build_ns: u64,
    pub build_total_ns: u64,
    /// members of each shape class, chained by node id (built on the first probe by class)
    pub class_head: Vec<u32>,
    pub class_next: Vec<u32>,
    pub class_built: bool,
}

impl State {
    /// a new file: nothing of the last one is built
    pub(crate) fn new_file(&mut self) {
        self.built = [0; 5];
        self.view_file = None;
        self.class_built = false;
    }

    fn clock(&mut self) {
        let ns = BUILD_NS.with(|b| b.replace(0));
        self.build_ns += ns;
        self.build_total_ns += ns;
    }
}

pub(crate) fn produce(c: &Ctx<'_, '_>, wanted: u64, sink: &mut dyn Sink, buf: &mut String, st: &mut State, only: Option<u32>) {
    let want = |name: &str| index_of(name).filter(|i| wanted & (1 << i) != 0);
    if let Some(r) = want("identifier") {
        rows_identifier(c, sink, r, only);
    }
    if let Some(r) = want("literal-properties") {
        rows_literal_properties(c, sink, r, only);
    }
    if let Some(r) = want("string-literal") {
        rows_string_literal(c, sink, r, buf, only);
    }
    if let Some(r) = want("static-import") {
        rows_static_import(c, sink, r, buf, only);
    }
    if let Some(r) = want("numeric-literal") {
        rows_numeric_literal(c, sink, r, buf, only);
    }
    if let Some(r) = want("call-argument") {
        rows_call_argument(c, sink, r, only);
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
        rows_embedded_expression(c, sink, r, only);
    }
    if let Some(r) = want("function") {
        rows_function(c, sink, r, only);
    }
    if let Some(r) = want("parameter") {
        rows_parameter(c, sink, r, only);
    }
    if let Some(r) = want("decorates") {
        rows_decorates(c, sink, r, buf, only);
    }
    if let Some(r) = want("inherits") {
        rows_inherits(c, sink, r, buf, only);
    }
    if let Some(r) = want("bytes-literal") {
        rows_bytes_literal(c, sink, r, buf, only);
    }
    if let Some(r) = want("interpolated-string-literal") {
        rows_interpolated_string_literal(c, sink, r, only);
    }
    if let Some(r) = want("expression-shape") {
        rows_expression_shape(c, sink, r, &mut st.shapes, &mut st.built[0], only);
    }
    if let Some(r) = want("expression-type") {
        rows_expression_type(c, sink, r, &mut st.types, &mut st.built[1], only);
    }
    if let Some(r) = want("syntax-child") {
        rows_syntax_child(c, sink, r, st, only);
    }
    for which in ["syntax-tree", "source-location", "comment", "indent-region"] {
        if let Some(r) = want(which) {
            rows_view(c, sink, which, r, st, only);
        }
    }
    if let Some(r) = want("function-terminal") {
        rows_function_terminal(c, sink, r, &mut st.terminals, &mut st.built[2], only);
    }
    if let Some(r) = want("constant-key") {
        rows_constant_key(c, sink, r, &mut st.constant_keys, &mut st.built[3], only);
    }
    if let Some(r) = want("token-link") {
        rows_token_link(c, sink, r, only);
    }
    let (fp, ff) = (want("format-parse"), want("format-field"));
    if fp.is_some() || ff.is_some() {
        rows_format(c, sink, fp, ff, buf, st, only);
    }
    let (bp, bf) = (want("brace-format-parse"), want("brace-format-field"));
    if bp.is_some() || bf.is_some() {
        rows_brace_format(c, sink, bp, bf, buf, only);
    }
    if let Some(r) = want("literal-expression-parse") {
        rows_literal_expression(c, sink, r, buf, st, only);
    }
    let (rs, rf) = (want("raw-format-segment"), want("raw-format-field"));
    if rs.is_some() || rf.is_some() {
        rows_raw_format(c, sink, rs, rf, st, only);
    }
    if let Some(r) = want("format-specification") {
        rows_format_specification(c, sink, r, buf, only);
    }
    if let Some(r) = want("filesystem.executable") {
        // the executable bit of the file as the run reads it (host)
        use std::os::unix::fs::PermissionsExt as _;
        let exec = std::fs::metadata(c.path).map(|m| m.permissions().mode() & 0o111 != 0).unwrap_or(false);
        sink.row(r, &[Cell::File, Cell::Flag(exec)]);
    }
}
