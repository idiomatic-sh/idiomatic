//! The types of the `counting` concept: the work ledger of a measured run.

use ryo_relations::domain::relations::types::{Count, Relation};
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// The work of a measured run, one row per unit: each rule's own program, each definition pattern, each made table; then one
/// row for each kind of work the language did for the programs (each relation it filled, and its rendering), whose time no
/// unit holds.
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
///     owner_steps: vec![None],
///     rows: Count(2),
///     steps: Count(8),
///     nanos: Count(0),
///     idle_nanos: Count(0),
///     decide_nanos: Count(0),
///     provider_nanos: Count(0),
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
    /// For each of `owners`, in its order: the step of that rule's plan whose rows the unit makes, by which the host names the
    /// statement it was written as; none when no step of the plan bears the unit's node (a cut that is one side of a step).
    pub owner_steps: Vec<Option<StepId>>,
    /// The rows it gave: the size of its work, whatever the engine.
    pub rows: Count,
    /// The steps it took and the rows its stages read and gave.
    pub steps: Count,
    /// The thread time spent in it, what it called left out.
    pub nanos: Count,
    /// The part of that time spent in files where it gave no row.
    pub idle_nanos: Count,
    /// The part of that time spent deciding, before any of its branches ran, whether the file can hold a row of it: a
    /// rule's own program only.
    pub decide_nanos: Count,
    /// The time the language spent filling while the unit ran, which it asked for: not in `nanos`, and in the row of the
    /// work that filled.
    pub provider_nanos: Count,
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
    /// Work the language did for the programs.
    Provider(ProviderWork),
}

/// Work the language does for the programs, timed apart from the unit that asked for it.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::counting::types::ProviderWork;
/// use ryo_relations::domain::relations::types::Relation;
///
/// assert_ne!(ProviderWork::Relation(Relation::Comment), ProviderWork::Render);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProviderWork {
    /// Rows of a relation it filled, whole or for a key.
    Relation(Relation),
    /// A node it rendered.
    Render,
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
    /// The language's own work: a relation's fill, or a rendering.
    ProviderFill,
}

/// What one kind of work the language did for the programs came to over the files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProviderCounts {
    /// The work.
    pub(crate) work: ProviderWork,
    /// The times it was asked: a whole fill, a key's fill, a rendering.
    pub(crate) asks: Count,
    /// The rows it wrote.
    pub(crate) rows: Count,
    /// The thread time it took.
    pub(crate) nanos: Count,
    /// The part of that time in asks that wrote no row.
    pub(crate) idle_nanos: Count,
}

/// The language's work for one file, as the facts the scan reads it through time it: what is not yet taken out of the time of
/// the unit running when it was done, and each piece of work of the file so far. Shared between a measuring worker's state
/// and the facts of its scan, so its counts are atomic; a plain worker has none in use.
#[derive(Debug, Default)]
pub(crate) struct FillClock {
    /// Thread time the language spent since the unit running last changed: the running unit's, taken out of its own.
    pub(crate) unpaid: std::sync::atomic::AtomicU64,
    /// Each piece of work done in the file so far.
    pub(crate) done: std::sync::Mutex<Vec<ProviderFill>>,
}

/// One piece of the language's work: what it was, the rows it wrote and its thread time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProviderFill {
    /// What the language did.
    pub(crate) work: ProviderWork,
    /// The rows it wrote.
    pub(crate) rows: Count,
    /// Its thread time.
    pub(crate) nanos: Count,
}
