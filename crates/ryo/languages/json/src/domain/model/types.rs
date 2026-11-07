//! The `model` concept's types: what a file keeps for its readers, and what a container counts.

use ryo::languages;

/// What is worked out of a file once, when it is parsed: the place of every member among the members of its object and of every value
/// among the values of its array, so that a reader never counts the siblings before an element.
#[derive(Debug, Default)]
pub(crate) struct Model {
    /// The place of each element among those of its container, by the position of the element; zero for an element that is in no container.
    pub places: Vec<languages::Integer>,
}

/// What a container counts the places of: the members of an object, the values of an array.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Counted {
    /// The members of an object.
    Members,
    /// The values of an array.
    Values,
}
