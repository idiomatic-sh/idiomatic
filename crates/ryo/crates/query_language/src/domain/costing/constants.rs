//! The words the `costing` concept says a cost in.

/// The class of a query whose work grows with the rows it reads and gives.
pub(super) const LINEAR: &str = "linear";
/// The same, with a sort of each group.
pub(super) const LINEAR_WITH_SORTS: &str = "linear with sorts";
/// The class of a join of an input with itself on an order within a key group.
pub(super) const QUADRATIC_IN_GROUP: &str = "quadratic in a group";
/// The class of a join with no key within the file.
pub(super) const QUADRATIC_IN_FILE: &str = "quadratic in the file";
/// A join whose keys do not go beyond the file.
pub(super) const PRODUCT: &str = "a join with no key within the file";
/// A join of an input with itself that orders a column against itself, and the operations that say it with a sort.
pub(super) const ORDERED_PAIRS: &str = "a join of an input with itself on an order within a key group, which `as-of`, `group-min`, `group-max` or `group-ordinal` says with a sort";
/// An operation that sorts.
pub(super) const A_SORT: &str = "a sort of each group";
