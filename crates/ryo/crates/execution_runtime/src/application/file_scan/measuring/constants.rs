//! The constants of the `measuring` part: the names a ledger line is written with.

/// The names of a ledger line's values, each as it is written before its value: the unit's kind and number, its work class,
/// the rows it gave, its steps, its time, the part of it in files where it gave no row, and the rules that reach it.
pub(crate) const LINE: [&str; 8] = [
    "unit=",
    ":",
    " class=",
    " rows=",
    " steps=",
    " nanos=",
    " idle_nanos=",
    " owners=",
];

/// What separates the rules that reach a unit.
pub(crate) const OWNERS_BETWEEN: &str = ",";

/// A rule's own program.
pub(crate) const RULE: &str = "Rule";

/// A definition's pattern.
pub(crate) const PATTERN: &str = "Pattern";

/// A made table.
pub(crate) const TABLE: &str = "Table";

/// The class of a navigation program.
pub(crate) const NAVIGATION: &str = "Navigation";

/// The class of a definition's rows.
pub(crate) const ROWS_FILL: &str = "RowsFill";

/// The class of a group table.
pub(crate) const GROUP_FILL: &str = "GroupFill";

/// The class of an ordered table.
pub(crate) const ORDERED_FILL: &str = "OrderedFill";

/// The class of a graph table.
pub(crate) const GRAPH_FILL: &str = "GraphFill";
