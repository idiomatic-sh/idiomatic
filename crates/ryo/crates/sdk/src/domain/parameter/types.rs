//! The SDK's `parameter` concept: the values a rule's query parameters are bound to.

/// The name of a parameter of a rule's query.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterName(pub Box<str>);

/// The text a parameter is bound to, or one member of the set it is bound to.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterText(pub Box<str>);

/// A whole number a parameter is bound to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterNumber(pub i64);

/// A list or a set of whole numbers a parameter is bound to.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterNumbers(pub Vec<i64>);

/// A yes or no a parameter is bound to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FlagState {
    /// The flag is raised.
    On,
    /// The flag is not raised.
    Off,
}

/// What a parameter of a rule is bound to.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParameterValue {
    /// A whole number.
    Number(ParameterNumber),
    /// A yes or no.
    Flag(FlagState),
    /// Text.
    Text(ParameterText),
    /// A list or a set of numbers.
    Numbers(ParameterNumbers),
    /// A list or a set of text.
    Texts(Vec<ParameterText>),
    /// A list of sets of text: the sets in order.
    TextSets(Vec<Vec<ParameterText>>),
    /// A list of tables of text, each a row: what a `list<table>` parameter is bound to.
    Tables(Vec<ParameterTable>),
}

/// One table of a `list<table>` parameter: a row, its cells by key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParameterTable(pub Vec<ParameterCell>);

/// One cell of a table of a `list<table>` parameter: the key it is under and its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParameterCell {
    /// The key.
    pub key: ParameterText,
    /// The text.
    pub value: ParameterText,
}
