//! The `reference` relation of the language: a name read or deleted, and a dotted name read, by the whole dotted text: `a.b.c` is one
//! reference, not also `a`. A name bound is no reference; in `obj.x = 1` the object is.










/// Reads the `reference` rows of one Python file from its syntax tree.
pub(crate) struct ReferenceItem;
