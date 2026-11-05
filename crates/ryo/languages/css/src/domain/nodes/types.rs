//! The types of the `nodes` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// A node of a CSS syntax tree: one of the parsing results of CSS Syntax Level 3 (§5.2, `STANDARD`), named as it names them. A
/// rule's prelude, its declarations and its child rules are its children, between the `{-token` and the `}-token` of its body
/// when it has one; a declaration's name, colon, value and `!important` are its; a component value is a preserved token, a
/// function or a simple block (`component_value`). One closed list, each name once.
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
pub(crate) enum CssNode {
    /// A stylesheet: a list of rules, the whole file.
    Stylesheet,
    /// A qualified rule: a prelude, then its declarations and child rules, `a > b { .. }`.
    QualifiedRule,
    /// An at-rule: a name, a prelude, and for a block at-rule its declarations and child rules, `@media screen { .. }`.
    AtRule,
    /// A declaration: a name, a value, and an important flag, `color: red !important`.
    Declaration,
    /// A function: a name and a value, `rgb(..)`, `url("..")`.
    Function,
    /// A simple block: an associated token and a value, a `{}`-, `[]`- or `()`-block among component values.
    SimpleBlock,
}

/// Which of the standard's categories a node is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Category {
    /// The stylesheet.
    Stylesheet,
    /// A rule: an at-rule or a qualified rule.
    Rule,
    /// A declaration.
    Declaration,
    /// A component value: a function or a simple block (the preserved tokens are tokens).
    ComponentValue,
}
