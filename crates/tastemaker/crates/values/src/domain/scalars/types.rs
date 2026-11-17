//! The types of the `scalars` concept: plain data, public fields and derives, with no constructor and no inherent impl. The
//! scalar values a definition writes, text, numbers and yes or no, and the keys of its tables.

/// Text a value holds.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextValue(pub Box<str>);

/// A whole number a value holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WholeValue(pub i64);

/// A yes or no a value holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FlagValue {
    /// `true`.
    Yes,
    /// `false`.
    No,
}

/// A float a value holds, as written, so a value stays comparable and hashable.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FloatText(pub Box<str>);

/// The key of a table value.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValueKey(pub Box<str>);
