//! The provider-contract view a tree-in-place engine reads: the provider's CST relations (`semantic-syntax`, `semantic-child`,
//! `identifier`, `source-location`) answered from the parser's own nodes (the flat table) and the parser's own tokens, with no
//! fact rows. Ids 0..n are the table's AST nodes (pre-order), n.. are the parser's tokens. Kinds and fields carry the provider's
//! names (the `ast` standard's, kebab-case); fields come from the typed node (structural), not from token context.

use crate::symbols::{NULL, Symbols};
use crate::table::Table;
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::{AnyNodeRef, Expr};
use ruff_text_size::Ranged as _;

/// a token text not read yet
pub(crate) const TEXT_UNSET: u32 = u32::MAX - 3;

#[derive(Default)]
pub(crate) struct View {
    pub n_nodes: usize,
    pub kind: Vec<u32>,
    /// facts: a token's or gap's text code, interned when first read (TEXT_UNSET until then; NULL for nodes)
    pub text: Vec<std::cell::Cell<u32>>,
    pub start: Vec<u32>,
    pub stop: Vec<u32>,
    pub parent: Vec<u32>,
    pub field: Vec<u32>,
    pub ordinal: Vec<u32>,
    pub child_off: Vec<u32>,
    pub children: Vec<u32>,
    /// line starts (byte offsets), for source-location
    pub lines: Vec<u32>,
    /// indent regions: (id, parent region id, indentation symbol, start byte, end byte); the module is region 0
    pub regions: Vec<[u32; 5]>,
    pub region_of: crate::sema::FxMap<u32, u32>,
    pub(crate) leaves: Vec<(Option<TokenKind>, u32, u32)>,
    /// facts: the symbol of each node kind (by the parser's kind number; kinds with an async form are named per node) and of
    /// each token kind, found once per worker instead of hashed per id
    node_code: Vec<u32>,
    token_code: Vec<u32>,
    last_field: (usize, u32),
    counter: Vec<u32>,
    stack: Vec<u32>,
    /// facts: the parser's token index -> leaf index (the token's id is n_nodes + leaf) and back (u32::MAX: a gap)
    pub tok_leaf: Vec<u32>,
    pub leaf_tok: Vec<u32>,
    /// facts: build time by part over the run: leaves, node columns and fields, token loop, children, lines
    pub split_ns: [u64; 5],
}

pub(crate) fn node_kind_name(node: AnyNodeRef<'_>) -> &'static str {
    use AnyNodeRef as N;
    match node {
        N::ModModule(_) => "module",
        N::ModExpression(_) => "expression",
        N::StmtFunctionDef(f) => {
            if f.is_async {
                "async-function-def"
            } else {
                "function-def"
            }
        }
        N::StmtClassDef(_) => "class-def",
        N::StmtReturn(_) => "return",
        N::StmtDelete(_) => "delete",
        N::StmtTypeAlias(_) => "type-alias",
        N::StmtAssign(_) => "assign",
        N::StmtAugAssign(_) => "aug-assign",
        N::StmtAnnAssign(_) => "ann-assign",
        N::StmtFor(f) => {
            if f.is_async {
                "async-for"
            } else {
                "for"
            }
        }
        N::StmtWhile(_) => "while",
        N::StmtIf(_) => "if",
        N::StmtWith(w) => {
            if w.is_async {
                "async-with"
            } else {
                "with"
            }
        }
        N::StmtMatch(_) => "match",
        N::StmtRaise(_) => "raise",
        N::StmtTry(_) => "try",
        N::StmtAssert(_) => "assert",
        N::StmtImport(_) => "import",
        N::StmtImportFrom(_) => "import-from",
        N::StmtGlobal(_) => "global",
        N::StmtNonlocal(_) => "nonlocal",
        N::StmtExpr(_) => "expr",
        N::StmtPass(_) => "pass",
        N::StmtBreak(_) => "break",
        N::StmtContinue(_) => "continue",
        N::StmtIpyEscapeCommand(_) | N::ExprIpyEscapeCommand(_) => "ipy-escape-command",
        N::ExprBoolOp(_) => "bool-op",
        N::ExprNamed(_) => "named-expr",
        N::ExprBinOp(_) => "bin-op",
        N::ExprUnaryOp(_) => "unary-op",
        N::ExprLambda(_) => "lambda",
        N::ExprIf(_) => "if-exp",
        N::ExprDict(_) => "dict",
        N::ExprSet(_) => "set",
        N::ExprListComp(_) => "list-comp",
        N::ExprSetComp(_) => "set-comp",
        N::ExprDictComp(_) => "dict-comp",
        N::ExprGenerator(_) => "generator-exp",
        N::ExprAwait(_) => "await",
        N::ExprYield(_) => "yield",
        N::ExprYieldFrom(_) => "yield-from",
        N::ExprCompare(_) => "compare",
        N::ExprCall(_) => "call",
        N::ExprFString(_) | N::InterpolatedStringFormatSpec(_) => "joined-str",
        N::ExprTString(_) => "template-str",
        N::ExprStringLiteral(_)
        | N::ExprBytesLiteral(_)
        | N::ExprNumberLiteral(_)
        | N::ExprBooleanLiteral(_)
        | N::ExprNoneLiteral(_)
        | N::ExprEllipsisLiteral(_)
        | N::InterpolatedStringLiteralElement(_) => "constant",
        N::ExprAttribute(_) => "attribute",
        N::ExprSubscript(_) => "subscript",
        N::ExprStarred(_) => "starred",
        N::ExprName(_) => "name",
        N::ExprList(_) => "list",
        N::ExprTuple(_) => "tuple",
        N::ExprSlice(_) => "slice",
        N::ExceptHandlerExceptHandler(_) => "except-handler",
        N::PatternMatchValue(_) => "match-value",
        N::PatternMatchSingleton(_) => "match-singleton",
        N::PatternMatchSequence(_) => "match-sequence",
        N::PatternMatchMapping(_) => "match-mapping",
        N::PatternMatchClass(_) => "match-class",
        N::PatternMatchStar(_) => "match-star",
        N::PatternMatchAs(_) => "match-as",
        N::PatternMatchOr(_) => "match-or",
        N::TypeParamTypeVar(_) => "type-var",
        N::TypeParamTypeVarTuple(_) => "type-var-tuple",
        N::TypeParamParamSpec(_) => "param-spec",
        N::PatternArguments(_) => "pattern-arguments",
        N::PatternKeyword(_) => "pattern-keyword",
        N::Comprehension(_) => "comprehension",
        N::Arguments(_) => "call-arguments",
        N::Parameters(_) => "arguments",
        N::Parameter(_) => "arg",
        N::ParameterWithDefault(_) => "parameter-with-default",
        N::Keyword(_) => "keyword",
        N::Alias(_) => "alias",
        N::WithItem(_) => "withitem",
        N::MatchCase(_) => "match-case",
        N::Decorator(_) => "decorator",
        N::ElifElseClause(_) => "elif-else-clause",
        N::TypeParams(_) => "type-params",
        N::FString(_) => "f-string-part",
        N::TString(_) => "t-string-part",
        N::StringLiteral(_) => "string-part",
        N::BytesLiteral(_) => "bytes-part",
        N::Identifier(_) => "identifier",
        N::InterpolatedElement(_) => "formatted-value",
    }
}

/// Every name the view may answer for a ruff node kind (for the kinds-present skip): async variants are the same node kind.
pub(crate) fn token_kind_name(kind: TokenKind) -> &'static str {
    use TokenKind as T;
    match kind {
        T::Int | T::Float | T::Complex => "number",
        T::String => "string",
        T::FStringStart => "fstring-start",
        T::FStringMiddle => "fstring-middle",
        T::FStringEnd => "fstring-end",
        T::TStringStart => "tstring-start",
        T::TStringMiddle => "tstring-middle",
        T::TStringEnd => "tstring-end",
        T::IpyEscapeCommand | T::Question | T::Unknown => "errortoken",
        T::Comment => "comment",
        T::Newline => "newline",
        T::NonLogicalNewline => "nl",
        T::Indent => "indent",
        T::Dedent => "dedent",
        T::EndOfFile => "endmarker",
        T::Exclamation => "exclamation",
        T::Lpar => "lpar",
        T::Rpar => "rpar",
        T::Lsqb => "lsqb",
        T::Rsqb => "rsqb",
        T::Colon => "colon",
        T::Comma => "comma",
        T::Semi => "semi",
        T::Plus => "plus",
        T::Minus => "minus",
        T::Star => "star",
        T::Slash => "slash",
        T::Vbar => "vbar",
        T::Amper => "amper",
        T::Less => "less",
        T::Greater => "greater",
        T::Equal => "equal",
        T::Dot => "dot",
        T::Percent => "percent",
        T::Lbrace => "lbrace",
        T::Rbrace => "rbrace",
        T::EqEqual => "eqequal",
        T::NotEqual => "notequal",
        T::LessEqual => "lessequal",
        T::GreaterEqual => "greaterequal",
        T::Tilde => "tilde",
        T::CircumFlex => "circumflex",
        T::LeftShift => "leftshift",
        T::RightShift => "rightshift",
        T::DoubleStar => "doublestar",
        T::DoubleStarEqual => "doublestarequal",
        T::PlusEqual => "plusequal",
        T::MinusEqual => "minequal",
        T::StarEqual => "starequal",
        T::SlashEqual => "slashequal",
        T::PercentEqual => "percentequal",
        T::AmperEqual => "amperequal",
        T::VbarEqual => "vbarequal",
        T::CircumflexEqual => "circumflexequal",
        T::LeftShiftEqual => "leftshiftequal",
        T::RightShiftEqual => "rightshiftequal",
        T::DoubleSlash => "doubleslash",
        T::DoubleSlashEqual => "doubleslashequal",
        T::ColonEqual => "colonequal",
        T::At => "at",
        T::AtEqual => "atequal",
        T::Rarrow => "rarrow",
        T::Ellipsis => "ellipsis",
        _ => "name-token",
    }
}

fn is_operator_token(kind: TokenKind) -> bool {
    use TokenKind as T;
    !matches!(
        kind,
        T::Lpar | T::Rpar | T::Lsqb | T::Rsqb | T::Lbrace | T::Rbrace | T::Comma | T::Comment | T::Newline
            | T::NonLogicalNewline | T::Indent | T::Dedent
    )
}

/// The provider's field of each direct AST child, from the typed parent: `out(field, child)` per field occurrence.
fn fields<'a>(parent: AnyNodeRef<'a>, out: &mut dyn FnMut(&'static str, AnyNodeRef<'a>)) {
    use AnyNodeRef as N;
    let e = |out: &mut dyn FnMut(&'static str, AnyNodeRef<'a>), f: &'static str, x: &'a Expr| out(f, AnyNodeRef::from(x));
    match parent {
        N::ModModule(m) => m.body.iter().for_each(|s| out("body", s.into())),
        N::StmtFunctionDef(f) => {
            f.decorator_list.iter().for_each(|d| out("decorator-list", d.into()));
            out("name", (&f.name).into());
            if let Some(t) = &f.type_params {
                out("type-params", t.as_ref().into());
            }
            out("args", f.parameters.as_ref().into());
            if let Some(r) = &f.returns {
                e(out, "returns", r);
            }
            f.body.iter().for_each(|s| out("body", s.into()));
        }
        N::StmtClassDef(c) => {
            c.decorator_list.iter().for_each(|d| out("decorator-list", d.into()));
            out("name", (&c.name).into());
            if let Some(t) = &c.type_params {
                out("type-params", t.as_ref().into());
            }
            c.body.iter().for_each(|s| out("body", s.into()));
        }
        N::Arguments(a) => {
            // call-arguments: under a class-def its positional arguments are `bases`
            a.args.iter().for_each(|x| e(out, "args", x));
            a.keywords.iter().for_each(|k| out("keywords", k.into()));
        }
        N::StmtReturn(r) => {
            if let Some(v) = &r.value {
                e(out, "value", v);
            }
        }
        N::StmtDelete(d) => d.targets.iter().for_each(|t| e(out, "targets", t)),
        N::StmtAssign(a) => {
            a.targets.iter().for_each(|t| e(out, "targets", t));
            e(out, "value", &a.value);
        }
        N::StmtAugAssign(a) => {
            e(out, "target", &a.target);
            e(out, "value", &a.value);
        }
        N::StmtAnnAssign(a) => {
            e(out, "target", &a.target);
            e(out, "annotation", &a.annotation);
            if let Some(v) = &a.value {
                e(out, "value", v);
            }
        }
        N::StmtFor(f) => {
            e(out, "target", &f.target);
            e(out, "iter", &f.iter);
            f.body.iter().for_each(|s| out("body", s.into()));
            f.orelse.iter().for_each(|s| out("orelse", s.into()));
        }
        N::StmtWhile(w) => {
            e(out, "test", &w.test);
            w.body.iter().for_each(|s| out("body", s.into()));
            w.orelse.iter().for_each(|s| out("orelse", s.into()));
        }
        N::StmtIf(i) => {
            e(out, "test", &i.test);
            i.body.iter().for_each(|s| out("body", s.into()));
            i.elif_else_clauses.iter().for_each(|c| out("orelse", c.into()));
        }
        N::ElifElseClause(c) => {
            if let Some(t) = &c.test {
                e(out, "test", t);
            }
            c.body.iter().for_each(|s| out("body", s.into()));
        }
        N::StmtWith(w) => {
            w.items.iter().for_each(|i| out("items", i.into()));
            w.body.iter().for_each(|s| out("body", s.into()));
        }
        N::WithItem(w) => {
            e(out, "context-expr", &w.context_expr);
            if let Some(v) = &w.optional_vars {
                e(out, "optional-vars", v);
            }
        }
        N::StmtRaise(r) => {
            if let Some(x) = &r.exc {
                e(out, "exc", x);
            }
            if let Some(x) = &r.cause {
                e(out, "cause", x);
            }
        }
        N::StmtTry(t) => {
            t.body.iter().for_each(|s| out("body", s.into()));
            t.handlers.iter().for_each(|h| out("handlers", h.into()));
            t.orelse.iter().for_each(|s| out("orelse", s.into()));
            t.finalbody.iter().for_each(|s| out("finalbody", s.into()));
        }
        N::ExceptHandlerExceptHandler(h) => {
            if let Some(t) = &h.type_ {
                e(out, "type", t);
            }
            if let Some(n) = &h.name {
                out("name", n.into());
            }
            h.body.iter().for_each(|s| out("body", s.into()));
        }
        N::StmtAssert(a) => {
            e(out, "test", &a.test);
            if let Some(m) = &a.msg {
                e(out, "msg", m);
            }
        }
        N::StmtImport(i) => i.names.iter().for_each(|a| out("names", a.into())),
        N::StmtImportFrom(i) => {
            if let Some(m) = &i.module {
                out("module", m.into());
            }
            i.names.iter().for_each(|a| out("names", a.into()));
        }
        N::Alias(a) => {
            out("name", (&a.name).into());
            if let Some(n) = &a.asname {
                out("asname", n.into());
            }
        }
        N::StmtGlobal(g) => g.names.iter().for_each(|n| out("names", n.into())),
        N::StmtNonlocal(g) => g.names.iter().for_each(|n| out("names", n.into())),
        N::StmtExpr(x) => e(out, "value", &x.value),
        N::ExprBoolOp(b) => b.values.iter().for_each(|v| e(out, "values", v)),
        N::ExprNamed(n) => {
            e(out, "target", &n.target);
            e(out, "value", &n.value);
        }
        N::ExprBinOp(b) => {
            e(out, "left", &b.left);
            e(out, "right", &b.right);
        }
        N::ExprUnaryOp(u) => e(out, "operand", &u.operand),
        N::ExprLambda(l) => {
            if let Some(p) = &l.parameters {
                out("args", p.as_ref().into());
            }
            e(out, "body", &l.body);
        }
        N::ExprIf(i) => {
            e(out, "body", &i.body);
            e(out, "test", &i.test);
            e(out, "orelse", &i.orelse);
        }
        N::ExprDict(d) => {
            for item in &d.items {
                if let Some(k) = &item.key {
                    e(out, "keys", k);
                }
                e(out, "values", &item.value);
            }
        }
        N::ExprSet(s) => s.elts.iter().for_each(|x| e(out, "elts", x)),
        N::ExprList(s) => s.elts.iter().for_each(|x| e(out, "elts", x)),
        N::ExprTuple(s) => s.elts.iter().for_each(|x| e(out, "elts", x)),
        N::ExprListComp(c) => {
            e(out, "elt", &c.elt);
            c.generators.iter().for_each(|g| out("generators", g.into()));
        }
        N::ExprSetComp(c) => {
            e(out, "elt", &c.elt);
            c.generators.iter().for_each(|g| out("generators", g.into()));
        }
        N::ExprGenerator(c) => {
            e(out, "elt", &c.elt);
            c.generators.iter().for_each(|g| out("generators", g.into()));
        }
        N::ExprDictComp(c) => {
            if let Some(k) = &c.key {
                e(out, "key", k);
            }
            e(out, "value", &c.value);
            c.generators.iter().for_each(|g| out("generators", g.into()));
        }
        N::Comprehension(c) => {
            e(out, "target", &c.target);
            e(out, "iter", &c.iter);
            c.ifs.iter().for_each(|x| e(out, "ifs", x));
        }
        N::ExprAwait(a) => e(out, "value", &a.value),
        N::ExprYield(y) => {
            if let Some(v) = &y.value {
                e(out, "value", v);
            }
        }
        N::ExprYieldFrom(y) => e(out, "value", &y.value),
        N::ExprCompare(c) => {
            if let Some((first, rest)) = c.operands.split_first() {
                e(out, "left", first);
                rest.iter().for_each(|x| e(out, "comparators", x));
            }
        }
        N::ExprCall(c) => e(out, "func", &c.func),
        N::ExprAttribute(a) => {
            e(out, "value", &a.value);
            out("attr", (&a.attr).into());
        }
        N::ExprSubscript(s) => {
            e(out, "value", &s.value);
            e(out, "slice", &s.slice);
        }
        N::ExprStarred(s) => e(out, "value", &s.value),
        N::ExprSlice(s) => {
            if let Some(x) = &s.lower {
                e(out, "lower", x);
            }
            if let Some(x) = &s.upper {
                e(out, "upper", x);
            }
            if let Some(x) = &s.step {
                e(out, "step", x);
            }
        }
        N::ExprStringLiteral(s) => s.value.iter().for_each(|p| out("value", p.into())),
        N::ExprBytesLiteral(s) => s.value.iter().for_each(|p| out("value", p.into())),
        N::Keyword(k) => {
            if let Some(a) = &k.arg {
                out("arg", a.into());
            }
            e(out, "value", &k.value);
        }
        N::Parameters(p) => {
            p.posonlyargs.iter().for_each(|a| out("posonlyargs", a.into()));
            p.args.iter().for_each(|a| out("args", a.into()));
            if let Some(v) = &p.vararg {
                out("vararg", v.as_ref().into());
            }
            p.kwonlyargs.iter().for_each(|a| out("kwonlyargs", a.into()));
            if let Some(v) = &p.kwarg {
                out("kwarg", v.as_ref().into());
            }
        }
        N::Parameter(p) => {
            out("arg", (&p.name).into());
            if let Some(a) = &p.annotation {
                e(out, "annotation", a);
            }
        }
        N::Decorator(d) => e(out, "none", &d.expression),
        // f-strings, interpolations and patterns (bead ryo-ger6, proven against the provider's syntax-child)
        N::ExprFString(f) => f.value.iter().for_each(|p| match p {
            ruff_python_ast::FStringPartRef::Literal(l) => out("values", l.into()),
            ruff_python_ast::FStringPartRef::FString(x) => out("none", x.into()),
        }),
        N::ExprTString(t) => t.value.iter().for_each(|x| out("values", x.into())),
        N::FString(f) => f.elements.iter().for_each(|x| out("values", x.into())),
        N::TString(f) => f.elements.iter().for_each(|x| out("values", x.into())),
        N::InterpolatedStringFormatSpec(f) => f.elements.iter().for_each(|x| out("values", x.into())),
        N::InterpolatedElement(i) => {
            e(out, "value", &i.expression);
            if let Some(f) = &i.format_spec {
                out("format-spec", f.as_ref().into());
            }
        }
        N::StmtMatch(m) => {
            e(out, "subject", &m.subject);
            m.cases.iter().for_each(|c| out("cases", c.into()));
        }
        N::MatchCase(c) => {
            out("pattern", (&c.pattern).into());
            if let Some(g) = &c.guard {
                e(out, "guard", g);
            }
            c.body.iter().for_each(|s| out("body", s.into()));
        }
        N::PatternMatchValue(v) => e(out, "value", &v.value),
        N::PatternMatchClass(c) => {
            e(out, "cls", &c.cls);
            out("none", (&c.arguments).into());
        }
        N::PatternArguments(a) => {
            a.patterns.iter().for_each(|x| out("patterns", x.into()));
            a.keywords.iter().for_each(|x| out("keywords", x.into()));
        }
        N::PatternKeyword(k) => {
            out("attr", (&k.attr).into());
            out("pattern", (&k.pattern).into());
        }
        N::PatternMatchOr(o) => o.patterns.iter().for_each(|x| out("patterns", x.into())),
        N::PatternMatchSequence(o) => o.patterns.iter().for_each(|x| out("patterns", x.into())),
        N::PatternMatchMapping(m) => {
            m.keys.iter().for_each(|k| e(out, "keys", k));
            m.patterns.iter().for_each(|x| out("patterns", x.into()));
            if let Some(r) = &m.rest {
                out("rest", r.into());
            }
        }
        N::PatternMatchAs(a) => {
            if let Some(p) = &a.pattern {
                out("pattern", p.as_ref().into());
            }
            if let Some(n) = &a.name {
                out("name", n.into());
            }
        }
        N::PatternMatchStar(a) => {
            if let Some(n) = &a.name {
                out("name", n.into());
            }
        }
        _ => {}
    }
}

impl View {
    pub(crate) fn build(
        &mut self,
        table: &Table<'_>,
        parsed: &ruff_python_parser::Parsed<ruff_python_ast::Mod>,
        source: &str,
        sym: &mut Symbols,
    ) {
        let n = table.len();
        let tokens = parsed.tokens();
        let tk0 = crate::nav::Tick::now();
        // the leaves of the lossless tree: the parser's tokens and a `whitespace` leaf for every gap between them
        self.leaves.clear();
        self.tok_leaf.clear();
        self.leaf_tok.clear();
        let mut prev = 0u32;
        for (ti, token) in tokens.iter().enumerate() {
            let (ts, te) = (token.start().to_u32(), token.end().to_u32());
            if ts > prev {
                self.leaves.push((None, prev, ts));
                self.leaf_tok.push(u32::MAX);
            }
            self.tok_leaf.push(self.leaves.len() as u32);
            self.leaf_tok.push(ti as u32);
            self.leaves.push((Some(token.kind()), ts, te));
            prev = prev.max(te);
        }
        if (source.len() as u32) > prev {
            self.leaves.push((None, prev, source.len() as u32));
            self.leaf_tok.push(u32::MAX);
        }
        let total = n + self.leaves.len();
        self.n_nodes = n;
        let tk1 = crate::nav::Tick::now();
        self.split_ns[0] += tk1.since(tk0);
        for v in [&mut self.kind, &mut self.start, &mut self.stop, &mut self.parent, &mut self.field, &mut self.ordinal] {
            v.clear();
            v.resize(total, 0);
        }
        self.text.clear();
        self.text.resize(n, std::cell::Cell::new(NULL));
        self.text.resize(total, std::cell::Cell::new(TEXT_UNSET));
        let none = sym.base("none");
        self.field[..].fill(none);
        self.parent[..].fill(u32::MAX);
        if self.node_code.is_empty() {
            self.node_code.resize(256, u32::MAX);
            self.token_code.resize(256, u32::MAX);
        }
        for id in 0..n {
            let node = table.node[id];
            let k = table.kind[id] as usize;
            let mut code = self.node_code[k];
            if code == u32::MAX {
                code = sym.base(node_kind_name(node));
                // a definition, a loop and a with statement have an async form with its own kind name: named per node
                if !matches!(node, AnyNodeRef::StmtFunctionDef(_) | AnyNodeRef::StmtFor(_) | AnyNodeRef::StmtWith(_)) {
                    self.node_code[k] = code;
                }
            }
            self.kind[id] = code;
            self.start[id] = table.start[id];
            self.stop[id] = table.stop[id];
            self.parent[id] = table.parent[id];
        }
        // structural fields of AST children
        let class_def = sym.base("class-def");
        let bases = sym.base("bases");
        let args = sym.base("args");
        for id in 0..n {
            let node = table.node[id];
            let mut set = |f: &'static str, child: AnyNodeRef<'_>| {
                let c = Table::id_of(child) as usize;
                if c < n {
                    if self.last_field.0 != f.as_ptr() as usize {
                        self.last_field = (f.as_ptr() as usize, sym.base_word(f));
                    }
                    self.field[c] = self.last_field.1;
                }
            };
            fields(node, &mut set);
            if let AnyNodeRef::ParameterWithDefault(p) = node {
                // the provider's parameter-with-default: its parameter is under the role of the owning list, its default `defaults`
                let owner = table.parent[id] as usize;
                let role = self.field[id];
                let pc = Table::id_of((&p.parameter).into()) as usize;
                if pc < n {
                    self.field[pc] = role;
                }
                if let Some(d) = &p.default {
                    let dc = Table::id_of(AnyNodeRef::from(d.as_ref())) as usize;
                    if dc < n {
                        self.field[dc] = sym.base(if owner < n && self.kind_is_kwonly(role, sym) { "kw-defaults" } else { "defaults" });
                    }
                }
            }
        }
        // call-arguments under a class-def: positional arguments are `bases`
        for id in 0..n {
            if let AnyNodeRef::Arguments(_) = table.node[id] {
                let p = table.parent[id] as usize;
                if p < n && self.kind[p] == class_def {
                    let end = table.end[id] as usize;
                    for c in id + 1..end {
                        if table.parent[c] as usize == id && self.field[c] == args {
                            self.field[c] = bases;
                        }
                    }
                }
            }
        }
        let tk2 = crate::nav::Tick::now();
        self.split_ns[1] += tk2.since(tk1);
        // tokens: parent = deepest node containing it, merged in source order; ordinals over nodes and tokens together
        self.counter.clear();
        self.counter.resize(total, 0);
        self.stack.clear();
        let mut next_node = 0usize;
        let op = sym.base("op");
        let ops = sym.base("ops");
        let idf = sym.base("id");
        let value = sym.base("value");
        let is_async = sym.base("is-async");
        let k_unary = sym.base("unary-op");
        let k_bin = sym.base("bin-op");
        let k_aug = sym.base("aug-assign");
        let k_bool = sym.base("bool-op");
        let k_cmp = sym.base("compare");
        let k_name = sym.base("name");
        let k_const = sym.base("constant");
        let k_comp = sym.base("comprehension");
        let k_from = sym.base("import-from");
        let k_fv = sym.base("formatted-value");
        let k_single = sym.base("match-singleton");
        let conversion = sym.base("conversion");
        let level = sym.base("level");
        let k_part = sym.base("string-part");
        let k_bpart = sym.base("bytes-part");
        let ordinal_node = |this: &mut Self, id: usize| {
            let p = this.parent[id];
            if p != u32::MAX {
                this.ordinal[id] = this.counter[p as usize];
                this.counter[p as usize] += 1;
            }
        };
        let whitespace = sym.base("whitespace");
        self.regions.clear();
        self.region_of.clear();
        self.regions.push([0, 0, sym.text(""), 0, source.len() as u32]);
        self.region_of.insert(0, 0);
        let mut open_regions: Vec<usize> = vec![0];
        let leaves = std::mem::take(&mut self.leaves);
        for (t, &(token_kind, ts, te)) in leaves.iter().enumerate() {
            // enter every node starting at or before this token (the stack follows the table's parent links)
            // a zero-width token (dedent) at a node's start is written before the node is entered, as the provider's writer does
            while next_node < n && table.start[next_node] <= ts && (table.start[next_node] < te || table.start[next_node] < ts) {
                let p = table.parent[next_node];
                while let Some(&top) = self.stack.last() {
                    if top != p && self.stack.contains(&p) {
                        self.stack.pop();
                    } else {
                        break;
                    }
                }
                ordinal_node(self, next_node);
                self.stack.push(next_node as u32);
                next_node += 1;
            }
            // leave every node that ends at or before the token's start
            while let Some(&top) = self.stack.last() {
                if table.stop[top as usize] <= ts && !(top == 0) {
                    self.stack.pop();
                } else {
                    break;
                }
            }
            let id = n + t;
            let parent = self.stack.last().copied().unwrap_or(u32::MAX);
            self.parent[id] = parent;
            self.kind[id] = match token_kind {
                None => whitespace,
                Some(k) => {
                    let slot = k as u8 as usize;
                    if self.token_code[slot] == u32::MAX {
                        self.token_code[slot] = sym.base(token_kind_name(k));
                    }
                    self.token_code[slot]
                }
            };
            self.start[id] = ts;
            self.stop[id] = te;
            ordinal_node(self, id);
            match token_kind {
                Some(TokenKind::Indent) => {
                    let p = self.regions[*open_regions.last().unwrap_or(&0)][0];
                    self.region_of.insert(id as u32, self.regions.len() as u32);
                    open_regions.push(self.regions.len());
                    // an indentation region carries its indentation's text: this token's text is read now
                    let indent = sym.text(&source[ts as usize..te as usize]);
                    self.text[id].set(indent);
                    self.regions.push([id as u32, p, indent, ts, source.len() as u32]);
                }
                Some(TokenKind::Dedent) => {
                    if open_regions.len() > 1 {
                        let r = open_regions.pop().unwrap_or(0);
                        self.regions[r][4] = ts;
                    }
                }
                _ => {}
            }
            let Some(kind) = token_kind else { continue };
            if parent != u32::MAX {
                let pk = self.kind[parent as usize];
                if pk == k_name && matches!(kind, TokenKind::Identifier) {
                    self.field[id] = idf;
                } else if (pk == k_unary || pk == k_bin || pk == k_aug || pk == k_bool) && is_operator_token(kind) {
                    self.field[id] = op;
                } else if pk == k_cmp && is_operator_token(kind) {
                    self.field[id] = ops;
                } else if pk == k_const && !matches!(kind, TokenKind::Lpar | TokenKind::Rpar | TokenKind::NonLogicalNewline | TokenKind::Comment) {
                    self.field[id] = value;
                } else if pk == k_single && token_kind_name(kind) == "name-token" {
                    self.field[id] = value;
                } else if pk == k_fv && token_kind_name(kind) == "name-token" {
                    self.field[id] = conversion;
                } else if pk == k_from && matches!(kind, TokenKind::Dot | TokenKind::Ellipsis) {
                    self.field[id] = level;
                } else if pk == k_comp && matches!(kind, TokenKind::Async) {
                    self.field[id] = is_async;
                } else if pk == k_part || pk == k_bpart {
                    // the string token of a part: field none, as the provider has it
                }
            }
        }
        self.leaves = leaves;
        while next_node < n {
            ordinal_node(self, next_node);
            next_node += 1;
        }
        let tk3 = crate::nav::Tick::now();
        self.split_ns[2] += tk3.since(tk2);
        // children CSR in ordinal order (nodes and tokens merged)
        self.child_off.clear();
        self.child_off.resize(total + 1, 0);
        for id in 0..total {
            let p = self.parent[id];
            if p != u32::MAX {
                self.child_off[p as usize + 1] += 1;
            }
        }
        for i in 0..total {
            self.child_off[i + 1] += self.child_off[i];
        }
        self.children.clear();
        self.children.resize(self.child_off[total] as usize, 0);
        for id in 0..total {
            let p = self.parent[id];
            if p != u32::MAX {
                let slot = self.child_off[p as usize] + self.ordinal[id];
                if let Some(cell) = self.children.get_mut(slot as usize) {
                    *cell = id as u32;
                }
            }
        }
        let tk4 = crate::nav::Tick::now();
        self.split_ns[3] += tk4.since(tk3);
        // line starts
        self.lines.clear();
        self.lines.push(0);
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                self.lines.push(i as u32 + 1);
            }
        }
        self.split_ns[4] += crate::nav::Tick::now().since(tk4);
    }

    fn kind_is_kwonly(&self, role: u32, sym: &Symbols) -> bool {
        sym.get_base("kwonlyargs") == Some(role)
    }

    pub(crate) fn len(&self) -> usize {
        self.kind.len()
    }

    /// The text code of an id: NULL for a node; a token's or a gap's text is interned on its first read (facts).
    #[inline]
    pub(crate) fn text_of(&self, i: usize, sym: &mut Symbols, source: &str) -> u32 {
        let t = self.text[i].get();
        if t != TEXT_UNSET {
            return t;
        }
        self.text_slow(i, sym, source)
    }

    #[inline(never)]
    fn text_slow(&self, i: usize, sym: &mut Symbols, source: &str) -> u32 {
        let t = sym.text(&source[self.start[i] as usize..self.stop[i] as usize]);
        self.text[i].set(t);
        t
    }

    pub(crate) fn children_of(&self, id: u32) -> &[u32] {
        let i = id as usize;
        &self.children[self.child_off[i] as usize..self.child_off[i + 1] as usize]
    }

    /// 1-based line of a byte offset
    pub(crate) fn line_of(&self, offset: u32) -> u32 {
        match self.lines.binary_search(&offset) {
            Ok(i) => i as u32 + 1,
            Err(i) => i as u32,
        }
    }

    pub(crate) fn column_of(&self, offset: u32) -> u32 {
        let line = self.line_of(offset);
        offset - self.lines[line as usize - 1]
    }
}
