//! The types of the `nodes` concept: the kinds of the nodes of a Python tree, by the names of the standard, `CPython`'s `ast` (its
//! ASDL, pinned in `tests/fixtures/standard/Python.asdl`), in kebab case: `FunctionDef` is `function-def`, `match_case` is
//! `match-case`. One flat list, each name once; the standard's categories are `category` in the service.

/// A node of the standard. Constructors the parser does not expose directly (such as operators) remain kinds so the list covers the
/// standard's whole ASDL; `AsyncFunctionDef`, `AsyncFor` and `AsyncWith` are retained from Ruff's async flags.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    strum::EnumIter,
    strum::IntoStaticStr,
    strum::FromRepr,
    strum::EnumCount,
)]
#[strum(serialize_all = "kebab-case")]
#[repr(u16)]
pub(crate) enum Node {
    /// `Module(stmt* body, type_ignore* type_ignores)`.
    Module,
    /// `Interactive(stmt* body)`.
    Interactive,
    /// `Expression(expr body)`.
    Expression,
    /// `FunctionType(expr* argtypes, expr returns)`.
    FunctionType,
    /// `FunctionDef(identifier name, arguments args, stmt* body, expr* decorator_list, expr? returns, string? type_comment, type_param* type_params)`.
    FunctionDef,
    /// `AsyncFunctionDef(identifier name, arguments args, stmt* body, expr* decorator_list, expr? returns, string? type_comment, type_param* type_params)`.
    AsyncFunctionDef,
    /// `ClassDef(identifier name, expr* bases, keyword* keywords, stmt* body, expr* decorator_list, type_param* type_params)`.
    ClassDef,
    /// `Return(expr? value)`.
    Return,
    /// `Delete(expr* targets)`.
    Delete,
    /// `Assign(expr* targets, expr value, string? type_comment)`.
    Assign,
    /// `TypeAlias(expr name, type_param* type_params, expr value)`.
    TypeAlias,
    /// `AugAssign(expr target, operator op, expr value)`.
    AugAssign,
    /// `AnnAssign(expr target, expr annotation, expr? value, int simple)`.
    AnnAssign,
    /// `For(expr target, expr iter, stmt* body, stmt* orelse, string? type_comment)`.
    For,
    /// `AsyncFor(expr target, expr iter, stmt* body, stmt* orelse, string? type_comment)`.
    AsyncFor,
    /// `While(expr test, stmt* body, stmt* orelse)`.
    While,
    /// `If(expr test, stmt* body, stmt* orelse)`.
    If,
    /// `With(withitem* items, stmt* body, string? type_comment)`.
    With,
    /// `AsyncWith(withitem* items, stmt* body, string? type_comment)`.
    AsyncWith,
    /// `Match(expr subject, match_case* cases)`.
    Match,
    /// `Raise(expr? exc, expr? cause)`.
    Raise,
    /// `Try(stmt* body, excepthandler* handlers, stmt* orelse, stmt* finalbody)`.
    Try,
    /// `TryStar(stmt* body, excepthandler* handlers, stmt* orelse, stmt* finalbody)`.
    TryStar,
    /// `Assert(expr test, expr? msg)`.
    Assert,
    /// `Import(alias* names, int? is_lazy)`.
    Import,
    /// `ImportFrom(identifier? module, alias* names, int? level, int? is_lazy)`.
    ImportFrom,
    /// `Global(identifier* names)`.
    Global,
    /// `Nonlocal(identifier* names)`.
    Nonlocal,
    /// `Expr(expr value)`.
    Expr,
    /// `Pass`.
    Pass,
    /// `Break`.
    Break,
    /// `Continue`.
    Continue,
    /// `BoolOp(boolop op, expr* values)`.
    BoolOp,
    /// `NamedExpr(expr target, expr value)`.
    NamedExpr,
    /// `BinOp(expr left, operator op, expr right)`.
    BinOp,
    /// `UnaryOp(unaryop op, expr operand)`.
    UnaryOp,
    /// `Lambda(arguments args, expr body)`.
    Lambda,
    /// `IfExp(expr test, expr body, expr orelse)`.
    IfExp,
    /// `Dict(expr?* keys, expr* values)`.
    Dict,
    /// `Set(expr* elts)`.
    Set,
    /// `ListComp(expr elt, comprehension* generators)`.
    ListComp,
    /// `SetComp(expr elt, comprehension* generators)`.
    SetComp,
    /// `DictComp(expr key, expr? value, comprehension* generators)`.
    DictComp,
    /// `GeneratorExp(expr elt, comprehension* generators)`.
    GeneratorExp,
    /// `Await(expr value)`.
    Await,
    /// `Yield(expr? value)`.
    Yield,
    /// `YieldFrom(expr value)`.
    YieldFrom,
    /// `Compare(expr left, cmpop* ops, expr* comparators)`.
    Compare,
    /// `Call(expr func, expr* args, keyword* keywords)`.
    Call,
    /// `FormattedValue(expr value, int conversion, expr? format_spec)`.
    FormattedValue,
    /// `Interpolation(expr value, constant str, int conversion, expr? format_spec)`.
    Interpolation,
    /// `JoinedStr(expr* values)`.
    JoinedStr,
    /// `TemplateStr(expr* values)`.
    TemplateStr,
    /// `Constant(constant value, string? kind)`.
    Constant,
    /// `Attribute(expr value, identifier attr, expr_context ctx)`.
    Attribute,
    /// `Subscript(expr value, expr slice, expr_context ctx)`.
    Subscript,
    /// `Starred(expr value, expr_context ctx)`.
    Starred,
    /// `Name(identifier id, expr_context ctx)`.
    Name,
    /// `List(expr* elts, expr_context ctx)`.
    List,
    /// `Tuple(expr* elts, expr_context ctx)`.
    Tuple,
    /// `Slice(expr? lower, expr? upper, expr? step)`.
    Slice,
    /// `MatchValue(expr value)`.
    MatchValue,
    /// `MatchSingleton(constant value)`.
    MatchSingleton,
    /// `MatchSequence(pattern* patterns)`.
    MatchSequence,
    /// `MatchMapping(expr* keys, pattern* patterns, identifier? rest)`.
    MatchMapping,
    /// `MatchClass(expr cls, pattern* patterns, identifier* kwd_attrs, pattern* kwd_patterns)`.
    MatchClass,
    /// `MatchStar(identifier? name)`.
    MatchStar,
    /// `MatchAs(pattern? pattern, identifier? name)`.
    MatchAs,
    /// `MatchOr(pattern* patterns)`.
    MatchOr,
    /// `comprehension(expr target, expr iter, expr* ifs, int is_async)`.
    Comprehension,
    /// `ExceptHandler(expr? type, identifier? name, stmt* body)`.
    ExceptHandler,
    /// `arguments(arg* posonlyargs, arg* args, arg? vararg, arg* kwonlyargs, expr?* kw_defaults, arg? kwarg, expr* defaults)`.
    Arguments,
    /// `arg(identifier arg, expr? annotation, string? type_comment)`.
    Arg,
    /// `keyword(identifier? arg, expr value)`.
    Keyword,
    /// `alias(identifier name, identifier? asname)`.
    Alias,
    /// `withitem(expr context_expr, expr? optional_vars)`.
    Withitem,
    /// `match_case(pattern pattern, expr? guard, stmt* body)`.
    MatchCase,
    /// `TypeVar(identifier name, expr? bound, expr? default_value)`.
    TypeVar,
    /// `ParamSpec(identifier name, expr? default_value)`.
    ParamSpec,
    /// `TypeVarTuple(identifier name, expr? default_value)`.
    TypeVarTuple,
    /// `TypeIgnore(int lineno, string tag)`.
    TypeIgnore,
    /// `Load`.
    Load,
    /// `Store`.
    Store,
    /// `Del`.
    Del,
    /// `And`.
    And,
    /// `Or`.
    Or,
    /// `Add`.
    Add,
    /// `Sub`.
    Sub,
    /// `Mult`.
    Mult,
    /// `MatMult`.
    MatMult,
    /// `Div`.
    Div,
    /// `Mod`.
    Mod,
    /// `Pow`.
    Pow,
    /// `LShift`.
    LShift,
    /// `RShift`.
    RShift,
    /// `BitOr`.
    BitOr,
    /// `BitXor`.
    BitXor,
    /// `BitAnd`.
    BitAnd,
    /// `FloorDiv`.
    FloorDiv,
    /// `Invert`.
    Invert,
    /// `Not`.
    Not,
    /// `UAdd`.
    UAdd,
    /// `USub`.
    USub,
    /// `Eq`.
    Eq,
    /// `NotEq`.
    NotEq,
    /// `Lt`.
    Lt,
    /// `LtE`.
    LtE,
    /// `Gt`.
    Gt,
    /// `GtE`.
    GtE,
    /// `Is`.
    Is,
    /// `IsNot`.
    IsNot,
    /// `In`.
    In,
    /// `NotIn`.
    NotIn,
}

/// The standard's category of a node: the sum or product type of the ASDL it is a constructor of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Category {
    /// `mod`: a module, or an expression or a line parsed alone.
    Module,
    /// `stmt`: a statement.
    Statement,
    /// `expr`: an expression.
    Expression,
    /// `pattern`: a pattern of a `case`.
    Pattern,
    /// A part of a statement or an expression: `arguments`, `arg`, `keyword`, `alias`, `withitem`, `match_case`, `comprehension`,
    /// `excepthandler`, `type_param`, `type_ignore`.
    Part,
    /// `expr_context`, `boolop`, `operator`, `unaryop`, `cmpop`: never a node of the tree.
    Operator,
    /// No node of the standard: a token, or a node the standard lacks.
    Other,
}
