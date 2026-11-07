//! The types of the `extras` concept: the kinds of a Python tree the standards lack, a closed list, in kebab case. Each is kept
//! because dropping it would drop the tokens it holds or the place it has; the projection of the tree onto the standard drops them.

/// An element of the tree no standard names.
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
pub(crate) enum Extra {
    /// White space between tokens: the standard's tokenizer gives no token for spaces, tabs, a line continuation or a byte-order mark, and the tree spells the file only with it.
    Whitespace,
    /// An identifier: the standard holds a name as a string; the node holds its token and its place.
    Identifier,
    /// A decorator: the standard holds the expression in `decorator_list`; the node holds the `@` and the place among the decorators.
    Decorator,
    /// An `elif` or `else` clause of an `if`: the standard nests an `elif` as an `If` in `orelse`; the node keeps the clauses flat, in the order written.
    ElifElseClause,
    /// The parenthesized arguments of a call or a class: the standard holds them as `args` and `keywords` (or `bases`); the node holds the parentheses.
    CallArguments,
    /// A parameter with its default: the standard holds the default apart, in `defaults` or `kw_defaults`; the node keeps it with its `arg`.
    ParameterWithDefault,
    /// The brackets of a definition's type parameters: the standard holds them as `type_params`.
    TypeParams,
    /// The parenthesized arguments of a class pattern: the standard holds them as `patterns`, `kwd_attrs` and `kwd_patterns`.
    PatternArguments,
    /// One keyword of a class pattern, `x=0`: the standard holds the name in `kwd_attrs` and the pattern in `kwd_patterns`.
    PatternKeyword,
    /// One string of an implicit concatenation, `'a' 'b'`: the standard joins them into one `Constant`.
    StringPart,
    /// One bytes literal of an implicit concatenation: the standard joins them into one `Constant`.
    BytesPart,
    /// One f-string of an implicit concatenation: the standard joins the parts into one `JoinedStr`.
    FStringPart,
    /// One t-string of an implicit concatenation: the standard joins the parts into one `TemplateStr`.
    TStringPart,
    /// An `IPython` escape command, `%time` or `!ls`: no Python, the library reads it in a notebook.
    IpyEscapeCommand,
}
