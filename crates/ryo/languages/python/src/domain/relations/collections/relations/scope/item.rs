//! The `scope` relation of the language: the module of a file, and a function, a class, a lambda or a comprehension, each under the
//! scope that encloses it, the module's for the outermost; the module encloses itself.








/// Reads the `scope` rows of one Python file from its syntax tree.
pub(crate) struct ScopeItem;
