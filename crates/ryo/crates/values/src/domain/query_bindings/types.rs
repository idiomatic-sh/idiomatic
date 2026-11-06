//! Owned portable query bindings, independent of execution state.
use crate::domain::plan_parts::types::{FlagState, LiteralText, WholeNumber};
/// What a parameter is bound to when a query runs: text that came from the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParameterBinding {
    /// A whole number.
    Number(WholeNumber),
    /// A yes or no.
    Flag(FlagState),
    /// Text.
    Text(LiteralText),
    /// A list or a set of numbers.
    Numbers(Vec<WholeNumber>),
    /// A list or a set of text.
    Texts(Vec<LiteralText>),
    /// A list of sets of text: the sets in order.
    TextSets(Vec<Vec<LiteralText>>),
    /// A list of tables of text, each a row.
    Tables(Vec<BindingTable>),
}

/// One table a `list<table>` parameter holds: a row, its cells by key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingTable(pub Vec<BindingCell>);

/// One cell of a table a `list<table>` parameter holds: the key it is under and its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingCell {
    /// The key.
    pub key: LiteralText,
    /// The text.
    pub value: LiteralText,
}
