//! Data of a canonical document.
use std::collections::BTreeMap;

/// A name or a text of a canonical document.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Label(pub String);

impl From<&str> for Label {
    fn from(text: &str) -> Self {
        Self(text.to_owned())
    }
}

/// A whole number of a canonical document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Quantity(pub i64);

/// The compact text of a canonical document: sorted keys, no padding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Document(pub String);

/// One value of a canonical document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Fragment {
    /// Nothing.
    Null,
    /// A whole number.
    Number(Quantity),
    /// A text.
    Text(Label),
    /// Values in order.
    List(Vec<Fragment>),
    /// Values by name, in name order.
    Object(BTreeMap<Label, Fragment>),
}

impl From<&str> for Fragment {
    fn from(text: &str) -> Self {
        Self::Text(Label::from(text))
    }
}
