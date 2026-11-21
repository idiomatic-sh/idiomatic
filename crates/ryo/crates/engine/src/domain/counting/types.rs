//! The types of the `counting` concept: the work ledger of a measured run.

use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// The work of a measured run, one row per unit: each rule's own program, each definition pattern, each made table.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::counting::types::WorkLedger;
///
/// assert!(WorkLedger::default().0.is_empty());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkLedger(pub Vec<LedgerRow>);

/// The work of one unit over the files counted.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::counting::types::{LedgerRow, LedgerUnit, WorkClass};
/// use ryo_relations::domain::relations::types::Count;
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
///
/// let rule = SelectedRuleOrdinal(0);
/// let row = LedgerRow {
///     unit: LedgerUnit::Rule(rule),
///     class: WorkClass::Navigation,
///     owners: vec![rule],
///     rows: Count(2),
///     steps: Count(8),
///     nanos: Count(0),
///     idle_nanos: Count(0),
/// };
/// assert_eq!(row.steps.0 / row.rows.0, 4);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerRow {
    /// What the unit is.
    pub unit: LedgerUnit,
    /// The class its reference cost per row is read by.
    pub class: WorkClass,
    /// The rules that reach it.
    pub owners: Vec<SelectedRuleOrdinal>,
    /// The rows it gave: the size of its work, whatever the engine.
    pub rows: Count,
    /// The steps it took and the rows its stages read and gave.
    pub steps: Count,
    /// The thread time spent in it, what it called left out.
    pub nanos: Count,
    /// The part of that time spent in files where it gave no row.
    pub idle_nanos: Count,
}

/// What a ledger row is of.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::counting::types::LedgerUnit;
/// use ryo_relations::domain::relations::types::Count;
///
/// assert_ne!(LedgerUnit::Pattern(Count(0)), LedgerUnit::Table(Count(0)));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedgerUnit {
    /// A rule's own program.
    Rule(SelectedRuleOrdinal),
    /// A definition's pattern, by its unit's place among the programs' units.
    Pattern(Count),
    /// A made table, by its unit's place.
    Table(Count),
}

/// The classes a reference cost per row is measured for.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::counting::types::WorkClass;
///
/// assert_ne!(WorkClass::Navigation, WorkClass::RowsFill);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkClass {
    /// A navigation program: a rule's own, or a pattern in call form.
    Navigation,
    /// A definition's rows.
    RowsFill,
    /// A group table.
    GroupFill,
    /// An ordered table.
    OrderedFill,
    /// A graph table.
    GraphFill,
}
