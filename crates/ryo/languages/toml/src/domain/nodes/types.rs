//! The types of the `nodes` concept: plain data, public fields and derives, with no constructor and no inherent impl. A node is a rule
//! of `toml.abnf` that opens a construct of a document, named by the rule in kebab case.

/// What a node of a TOML syntax tree is. One closed list, each name once.
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
pub(crate) enum TomlNode {
    /// The whole file: `toml`.
    Toml,
    /// A table opened by its header, `[a.b]`, with the entries below it: `std-table`.
    StdTable,
    /// One member of an array of tables, `[[a.b]]`, with the entries below it: `array-table`.
    ArrayTable,
    /// A key, an equals sign and a value: `keyval`.
    Keyval,
    /// A key, simple or dotted: `key`.
    Key,
    /// An array value, `[1, 2]`: `array`.
    Array,
    /// An inline table value, `{ a = 1 }`: `inline-table`.
    InlineTable,
}
