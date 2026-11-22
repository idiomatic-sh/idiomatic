//! The constants of the `measuring` part: the names a ledger line is written with.

/// The names of a ledger line's values, each as it is written before its value: the unit's kind and number (a provider's
/// work: what it worked on), its work class, the rows it gave, its steps (a provider's work: the times it was asked), its time,
/// the part of it in files where it gave no row, the rules that reach it by ordinal; then the part of its time spent
/// deciding whether the file can hold a row, the language's time it asked for (not in its own), the rules that reach it by
/// id, and where it was written: each statement once, with how many rules reach it there.
pub(crate) const LINE: [&str; 12] = [
    "unit=",
    ":",
    " class=",
    " rows=",
    " steps=",
    " nanos=",
    " idle_nanos=",
    " owners=",
    " decide_nanos=",
    " provider_nanos=",
    " rules=",
    " written=",
];

/// What separates the statements a unit was written as.
pub(crate) const SITES_BETWEEN: &str = ";";

/// What stands between a statement's document and its `let` name.
pub(crate) const IN_DOCUMENT: &str = ":";

/// What stands before each document that reaches a module's statement, innermost first (the rule's own query is left out).
pub(crate) const CALLED_FROM: &str = "<";

/// What stands before how many rules reach a unit at one statement, when more than one does.
pub(crate) const RULES_AT: &str = "*";

/// The statement of a step no statement wrote (a step the compiler added with no record), or of a unit whose node is no
/// step of the rule's plan, or of a run that kept no record of where its steps were written.
pub(crate) const UNWRITTEN: &str = "?";

/// A provider's work on a relation.
pub(crate) const RELATION: &str = "Relation";

/// A provider's rendering.
pub(crate) const RENDER: &str = "Render";

/// What a provider's rendering worked on.
pub(crate) const RENDERED: &str = "nodes";

/// The class of a provider's work.
pub(crate) const PROVIDER_FILL: &str = "ProviderFill";

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
