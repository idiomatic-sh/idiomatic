//! The types of the `plan_lowering` concept: the texts of a pack, and the stages a lowering runs to.

use ryo_values::domain::descriptions::types::{ColumnName, RelationName};
use ryo_values::domain::observations::types::ObservationName;
use ryo_values::domain::program::types::{ParsedModules, WrittenProgram};
use ryo_values::domain::runs::types::RuleId;

/// A pack as the query language's one parser read it: each rule's program, in rule order, and every fragment a program
/// may import, each parsed once for the whole pack. A fragment is named by the last part of the path an `import` writes.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::plan_lowering::types::PackPrograms;
/// use ryo_values::domain::program::types::ParsedModules;
///
/// let pack = PackPrograms { rules: Vec::new(), fragments: ParsedModules(Default::default()), context: Default::default() };
/// assert!(pack.rules.is_empty());
/// ```
#[derive(Clone, Debug)]
pub struct PackPrograms {
    /// The rules, in the order they are lowered.
    pub rules: Vec<PackRule>,
    /// The fragments, by name.
    pub fragments: ParsedModules,
    /// What the run gives the rules beside the catalog: its context tables and its rule observations.
    pub context: RuleContext,
}

/// What a run gives its rules beside the catalog, as the host hands it to the compiler: tables of constant rows that a
/// rule reads by relation name, and the outputs one rule observes for others to read by name.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::plan_lowering::types::RuleContext;
///
/// assert!(RuleContext::default().tables.is_empty());
/// ```
#[derive(Clone, Debug, Default)]
pub struct RuleContext {
    /// The context tables.
    pub tables: Vec<ContextTable>,
    /// The rule observations.
    pub observations: Vec<ContextObservation>,
}

/// A context table: a relation name a rule reads, and the rows the run gives it.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::plan_lowering::types::ContextTable;
/// use ryo_values::domain::descriptions::types::{ColumnName, RelationName};
///
/// let table = ContextTable { relation: RelationName("config.selected".into()), columns: vec![ColumnName("code".into())], rows: vec![vec!["F401".into()]] };
/// assert_eq!(table.rows.len(), 1);
/// ```
#[derive(Clone, Debug)]
pub struct ContextTable {
    /// The relation name a rule writes.
    pub relation: RelationName,
    /// Its columns.
    pub columns: Vec<ColumnName>,
    /// Its rows, each a value a column.
    pub rows: Vec<Vec<Box<str>>>,
}

/// A rule observation: a name a rule reads as a relation, the columns it has, and the rules whose observed outputs it
/// unites.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::plan_lowering::types::ContextObservation;
/// use ryo_values::domain::observations::types::ObservationName;
///
/// let seen = ContextObservation { name: ObservationName("matches".into()), columns: Vec::new(), producers: Vec::new() };
/// assert!(seen.producers.is_empty());
/// ```
#[derive(Clone, Debug)]
pub struct ContextObservation {
    /// The name a reading rule writes.
    pub name: ObservationName,
    /// The columns kept from each producer's output.
    pub columns: Vec<ColumnName>,
    /// The producers, in the order they are united.
    pub producers: Vec<ObservationProducer>,
}

/// A rule and the name of the output it observes (`observe <step> as <output>`).
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::plan_lowering::types::ObservationProducer;
/// use ryo_values::domain::observations::types::ObservationName;
/// use ryo_values::domain::runs::types::RuleId;
///
/// let producer = ObservationProducer { rule: RuleId("E501".into()), output: ObservationName("matches".into()) };
/// assert_eq!(&*producer.output.0, "matches");
/// ```
#[derive(Clone, Debug)]
pub struct ObservationProducer {
    /// The rule.
    pub rule: RuleId,
    /// Its observed output's name.
    pub output: ObservationName,
}

/// A rule of a pack and its parsed program, or why it has none.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::plan_lowering::types::{PackRule, RuleProgram, UnreadQuery};
/// use ryo_values::domain::runs::types::RuleId;
///
/// let rule = PackRule { rule: RuleId("F401".into()), program: RuleProgram::Unread(UnreadQuery("no query".into())) };
/// assert!(matches!(rule.program, RuleProgram::Unread(_)));
/// ```
#[derive(Clone, Debug)]
pub struct PackRule {
    /// The rule.
    pub rule: RuleId,
    /// Its program.
    pub program: RuleProgram,
}

/// A rule's program as the parser read it, or why there is none: no query in the pack, or a query the parser refused.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::plan_lowering::types::{RuleProgram, UnreadQuery};
///
/// assert!(matches!(RuleProgram::Unread(UnreadQuery("no query".into())), RuleProgram::Unread(_)));
/// ```
#[derive(Clone, Debug)]
pub enum RuleProgram {
    /// The program.
    Parsed(Box<WrittenProgram>),
    /// Why there is none.
    Unread(UnreadQuery),
}

/// Why a rule has no program, in a sentence.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::plan_lowering::types::UnreadQuery;
///
/// assert!(!UnreadQuery("no query".into()).0.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnreadQuery(pub Box<str>);

/// How far a lowering runs: each stage is the one before it and one more pass.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::plan_lowering::types::PlanningStage;
///
/// assert!(PlanningStage::Expanded < PlanningStage::Axes);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlanningStage {
    /// Every rule's query lowered, the fragments it imports inlined (`lower6d.py`).
    Expanded,
    /// Then the semijoin marks (`pass_semi.py`).
    Semijoins,
    /// Then the count marks (`pass_count.py`).
    Counts,
    /// Then the existence marks on axes (`pass_axis.py`): the plan the finish runs.
    Axes,
}

/// What of a query a lowering names when it refuses it: a step, a token, an operator or an argument, as written.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::plan_lowering::types::WrittenText;
///
/// assert_eq!(&*WrittenText("calls".into()).0, "calls");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrittenText(pub Box<str>);

impl std::fmt::Display for WrittenText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
