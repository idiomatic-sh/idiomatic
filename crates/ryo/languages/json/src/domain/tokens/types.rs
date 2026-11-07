//! The types of the `tokens` concept: plain data, public fields and derives, with no constructor and no inherent impl. A token is named
//! as the standard names it, in kebab case.

/// What a token of a json syntax tree is. One closed list, each name once.
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
    /// `[`, which opens an array. `begin-array`.
    BeginArray,
    /// `{`, which opens an object. `begin-object`.
    BeginObject,
    /// `]`, which closes an array. `end-array`.
    EndArray,
    /// `}`, which closes an object. `end-object`.
    EndObject,
    /// `:`, between a member's name and its value. `name-separator`.
    NameSeparator,
    /// `,`, between members or values. `value-separator`.
    ValueSeparator,
    /// A string, a member's name or a value, quotes included. `string`.
    String,
    /// A number. `number`.
    Number,
    /// The literal `false`. `false`.
    False,
    /// The literal `null`. `null`.
    Null,
    /// The literal `true`. `true`.
    True,
    /// White space between the parts of the grammar: spaces, tabs and line ends. `ws`.
    Ws,
}
