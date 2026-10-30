//! The types of the `labels` concept: the roles of the children of a Python tree, by the field names of `CPython`'s `ast` (its ASDL,
//! pinned in `tests/fixtures/standard/Python.asdl`), in kebab case: `decorator_list` is `decorator-list`. A child is in the field of the
//! nearest node of the standard above it that holds it; an edge the standard gives no field has none.

/// The role of an element under its parent.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
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
pub(crate) enum Field {
    /// The edge has no field of the standard: a keyword, a piece of punctuation, trivia, or a part the standard does not have.
    #[default]
    None,
    /// `body`: the statements of a module, a definition, a loop, a branch, a handler or a case; a lambda's expression; the value an
    /// `if` expression gives when its test holds.
    Body,
    /// `name`: the name a definition, a type parameter, an alias, a handler or a capture pattern binds or names.
    Name,
    /// `args`: a function's or a lambda's parameters (`arguments`), the parameters before `*` and after `/`, or a call's positional
    /// arguments.
    Args,
    /// `decorator_list`: a decorator of a function or a class.
    DecoratorList,
    /// `returns`: a function's return annotation.
    Returns,
    /// `type_params`: the type parameters of a function, a class or a type alias.
    TypeParams,
    /// `bases`: a base class.
    Bases,
    /// `keywords`: a keyword argument of a call or of a class.
    Keywords,
    /// `value`: what a statement or an expression evaluates, assigns or holds, or a constant's literal.
    Value,
    /// `targets`: what an assignment or a `del` binds or removes.
    Targets,
    /// `target`: what an augmented, annotated or named assignment, a loop or a comprehension binds.
    Target,
    /// `op`: the operator of a binary, unary, boolean or augmented operation.
    Op,
    /// `annotation`: the annotation of an annotated assignment or of a parameter.
    Annotation,
    /// `iter`: what a loop or a comprehension iterates.
    Iter,
    /// `orelse`: what a loop's, an `if`'s or a `try`'s `else` holds, or the value an `if` expression gives when its test fails.
    Orelse,
    /// `test`: the condition of an `if`, a `while`, an `assert` or an `if` expression.
    Test,
    /// `items`: the context managers of a `with`.
    Items,
    /// `context_expr`: the expression a `with` item enters.
    ContextExpr,
    /// `optional_vars`: what a `with` item binds after `as`.
    OptionalVars,
    /// `subject`: what a `match` matches.
    Subject,
    /// `cases`: a case of a `match`.
    Cases,
    /// `pattern`: the pattern of a case or of a capture pattern.
    Pattern,
    /// `guard`: the `if` of a case.
    Guard,
    /// `exc`: what a `raise` raises.
    Exc,
    /// `cause`: what a `raise` names after `from`.
    Cause,
    /// `handlers`: an `except` clause of a `try`.
    Handlers,
    /// `finalbody`: the statements of a `try`'s `finally`.
    Finalbody,
    /// `msg`: the message of an `assert`.
    Msg,
    /// `names`: what an import imports, or the names a `global` or a `nonlocal` declares.
    Names,
    /// `module`: the module a `from` import imports from.
    Module,
    /// `level`: the dots of a relative import.
    Level,
    /// `values`: the operands of a boolean operation, the parts of an f-string or a t-string.
    Values,
    /// `left`: the left operand of a binary operation or of a comparison.
    Left,
    /// `right`: the right operand of a binary operation.
    Right,
    /// `operand`: the operand of a unary operation.
    Operand,
    /// `keys`: a key of a dictionary display or of a mapping pattern.
    Keys,
    /// `elt`: the element a list, set or generator comprehension makes.
    Elt,
    /// `generators`: a `for` clause of a comprehension.
    Generators,
    /// `key`: the key a dictionary comprehension makes.
    Key,
    /// `ifs`: an `if` clause of a comprehension.
    Ifs,
    /// `is_async`: the `async` of an asynchronous comprehension.
    IsAsync,
    /// `ops`: an operator of a comparison.
    Ops,
    /// `comparators`: an operand of a comparison after the first.
    Comparators,
    /// `func`: what a call calls.
    Func,
    /// `conversion`: the conversion of an interpolation, `!r`, `!s` or `!a`.
    Conversion,
    /// `format_spec`: the format spec of an interpolation.
    FormatSpec,
    /// `attr`: the attribute an attribute reference names.
    Attr,
    /// `slice`: the index or the slice of a subscript.
    Slice,
    /// `id`: the identifier a name writes.
    Id,
    /// `elts`: an element of a list, a tuple or a set display.
    Elts,
    /// `lower`: the start of a slice.
    Lower,
    /// `upper`: the end of a slice.
    Upper,
    /// `step`: the step of a slice.
    Step,
    /// `patterns`: a subpattern of a sequence, mapping, class or or-pattern.
    Patterns,
    /// `rest`: the name a mapping pattern binds after `**`.
    Rest,
    /// `cls`: the class a class pattern matches.
    Cls,
    /// `kwd_attrs`: the attribute a keyword of a class pattern names.
    KwdAttrs,
    /// `kwd_patterns`: the pattern a keyword of a class pattern matches.
    KwdPatterns,
    /// `type`: the exception an `except` clause catches.
    Type,
    /// `posonlyargs`: a parameter before `/`.
    Posonlyargs,
    /// `vararg`: the `*` parameter.
    Vararg,
    /// `kwonlyargs`: a parameter after `*`.
    Kwonlyargs,
    /// `kw_defaults`: the default of a keyword-only parameter.
    KwDefaults,
    /// `kwarg`: the `**` parameter.
    Kwarg,
    /// `defaults`: the default of a positional parameter.
    Defaults,
    /// `arg`: the name a parameter or a keyword argument writes.
    Arg,
    /// `bound`: the bound of a type variable.
    Bound,
    /// `default_value`: the default of a type parameter.
    DefaultValue,
    /// `asname`: the name an import binds after `as`.
    Asname,
}

/// Where the parameters of an `arguments` stand, by the `/` and the `*` before or after them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Section {
    /// Before a `/` that follows.
    PositionalOnly,
    /// Neither before a `/` nor after a `*`.
    Positional,
    /// After a `*`, bare or a parameter's.
    KeywordOnly,
}
