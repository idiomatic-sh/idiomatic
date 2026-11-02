//! The names of the `ruff_python_parser` adapter: what it says of a file beside what the library says.

/// What the adapter reports of bytes that are not UTF-8, where they are.
pub(super) const NOT_UTF8: &str =
    "the bytes here are not UTF-8: the lexer read them as text it could not read";

/// The character the adapter hands the library for each byte that is not UTF-8: one byte long, so every offset stays the file's, and
/// no token of Python, so the lexer reports it as text it could not read.
pub(super) const UNREADABLE: char = '$';

/// What `CPython`'s parser says of an `else` of a `try` with no `except`, which the library reads without an error.
pub(super) const ELSE_WITHOUT_EXCEPT: &str = "expected 'except' or 'finally' block";

/// The features `from __future__ import` takes, as `__future__.all_feature_names` of `CPython` 3.15.0rc2 lists them (pinned in
/// `tests/fixtures/standard/future_features.txt`): any other is refused by `CPython`'s parser.
pub(super) const FUTURE_FEATURES: [&str; 10] = [
    "nested_scopes",
    "generators",
    "division",
    "absolute_import",
    "with_statement",
    "print_function",
    "unicode_literals",
    "barry_as_FLUFL",
    "generator_stop",
    "annotations",
];

/// The module whose features an import from turns on.
pub(super) const FUTURE: &str = "__future__";

/// The feature `CPython` answers with a joke instead of the message for an unknown feature.
pub(super) const BRACES: &str = "braces";

/// What `CPython`'s parser says of `from __future__ import braces`.
pub(super) const NOT_A_CHANCE: &str = "not a chance";

/// What `CPython`'s parser says of a feature it does not know, before the feature's name.
pub(super) const UNKNOWN_FEATURE: &str = "future feature ";

/// What `CPython`'s parser says of a feature it does not know, after the feature's name.
pub(super) const NOT_DEFINED: &str = " is not defined";

/// Stable provider-owned codes for parser and validation diagnostics.
/// Parser syntax diagnostics normalized to the provider-owned `parser-error` code.
pub(super) const PARSER_ERROR: ryo::languages::DiagnosticCode =
    ryo::languages::DiagnosticCode::provider_owned("parser-error");
/// Target-version syntax unsupported by the selected Python version.
pub(super) const UNSUPPORTED_SYNTAX: ryo::languages::DiagnosticCode =
    ryo::languages::DiagnosticCode::provider_owned("unsupported-syntax");
/// Invalid UTF-8 rejected before the parser receives source text.
pub(super) const INVALID_UTF8: ryo::languages::DiagnosticCode =
    ryo::languages::DiagnosticCode::provider_owned("invalid-utf8");
/// An `else` clause without a matching `except` clause.
pub(super) const ELSE_WITHOUT_EXCEPT_CODE: ryo::languages::DiagnosticCode =
    ryo::languages::DiagnosticCode::provider_owned("else-without-except");
/// A future feature name not present in the parser's supported feature set.
pub(super) const FUTURE_FEATURE_NOT_DEFINED: ryo::languages::DiagnosticCode =
    ryo::languages::DiagnosticCode::provider_owned("future-feature-not-defined");
/// A repeated parameter name in one function or lambda signature.
pub(super) const DUPLICATE_PARAMETER: ryo::languages::DiagnosticCode =
    ryo::languages::DiagnosticCode::provider_owned("duplicate-parameter");
/// A repeated type-parameter name in one generic declaration.
pub(super) const DUPLICATE_TYPE_PARAMETER: ryo::languages::DiagnosticCode =
    ryo::languages::DiagnosticCode::provider_owned("duplicate-type-parameter");
/// A required type parameter follows one with a default.
pub(super) const TYPE_PARAMETER_DEFAULT_ORDER: ryo::languages::DiagnosticCode =
    ryo::languages::DiagnosticCode::provider_owned("type-parameter-default-order");
/// A call or class keyword argument is specified more than once.
pub(super) const DUPLICATE_KEYWORD_ARGUMENT: ryo::languages::DiagnosticCode =
    ryo::languages::DiagnosticCode::provider_owned("duplicate-keyword-argument");
/// A `nonlocal` declaration occurs at module scope.
pub(super) const NONLOCAL_AT_MODULE: ryo::languages::DiagnosticCode =
    ryo::languages::DiagnosticCode::provider_owned("nonlocal-at-module");
/// A `nonlocal` declaration names a current function parameter.
pub(super) const NONLOCAL_PARAMETER: ryo::languages::DiagnosticCode =
    ryo::languages::DiagnosticCode::provider_owned("nonlocal-parameter");
/// A `global` declaration names a current function parameter.
pub(super) const GLOBAL_PARAMETER: ryo::languages::DiagnosticCode =
    ryo::languages::DiagnosticCode::provider_owned("global-parameter");
/// An annotation targets a name already declared global in its scope.
pub(super) const ANNOTATED_GLOBAL: ryo::languages::DiagnosticCode =
    ryo::languages::DiagnosticCode::provider_owned("annotated-global");
/// A named expression rebinds an enclosing comprehension's iteration variable.
pub(super) const COMPREHENSION_REBIND: ryo::languages::DiagnosticCode =
    ryo::languages::DiagnosticCode::provider_owned("comprehension-rebind");

/// A starred target has no enclosing tuple or list unpacking target.
pub(super) const BARE_STARRED_TARGET: ryo::languages::DiagnosticCode =
    ryo::languages::DiagnosticCode::provider_owned("starred-assignment-target");
