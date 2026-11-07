//! The types of the `tokens` concept: plain data, public fields and derives, with no constructor and no inherent impl. A token is a
//! terminal rule of `toml.abnf`, named by the rule in kebab case.

/// What a token of a TOML syntax tree is. One closed list, each name once.
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
pub(crate) enum Token {
    /// Spaces and tabs: `ws`.
    Ws,
    /// The end of a line: `newline`.
    Newline,
    /// A comment, from `#` to the end of the line: `comment`.
    Comment,
    /// A key written bare: `unquoted-key`.
    UnquotedKey,
    /// A string between `"`: `basic-string`.
    BasicString,
    /// A string between `'`: `literal-string`.
    LiteralString,
    /// A string between `"""`: `ml-basic-string`.
    MlBasicString,
    /// A string between `'''`: `ml-literal-string`.
    MlLiteralString,
    /// A whole number: `integer`.
    Integer,
    /// A number with a fraction or an exponent, `inf` or `nan`: `float`.
    Float,
    /// `true` or `false`: `boolean`.
    Boolean,
    /// A date, a time, or both: `date-time`.
    DateTime,
    /// `=`, between a key and its value: `keyval-sep`.
    KeyvalSep,
    /// `.`, between the parts of a dotted key: `dot-sep`.
    DotSep,
    /// `,`, between the values of an array: `array-sep`.
    ArraySep,
    /// `,`, between the entries of an inline table: `inline-table-sep`.
    InlineTableSep,
    /// `[`, opening a table's header: `std-table-open`.
    StdTableOpen,
    /// `]`, closing a table's header: `std-table-close`.
    StdTableClose,
    /// `[[`: `array-table-open`.
    ArrayTableOpen,
    /// `]]`: `array-table-close`.
    ArrayTableClose,
    /// `[`, opening an array: `array-open`.
    ArrayOpen,
    /// `]`, closing an array: `array-close`.
    ArrayClose,
    /// `{`: `inline-table-open`.
    InlineTableOpen,
    /// `}`: `inline-table-close`.
    InlineTableClose,
}
