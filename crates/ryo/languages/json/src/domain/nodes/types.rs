//! The types of the `nodes` concept: plain data, public fields and derives, with no constructor and no inherent impl. A node is named
//! as the standard names it, in kebab case.

/// What a node of a json syntax tree is. One closed list, each name once.
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
pub(crate) enum JsonNode {
    /// The whole file: the root rule of the grammar, a value with white space around it. `JSON-text`.
    JsonText,
    /// An object: a pair of braces around members. `object`.
    Object,
    /// A member of an object: a name, the colon and a value. `member`.
    Member,
    /// An array: a pair of brackets around values. `array`.
    Array,
}
