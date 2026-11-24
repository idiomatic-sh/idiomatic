//! The types of the `finish_plans` concept: a plan, its steps and what each rule emits. Plain data.

use super::arguments::literals::types::{ExtendArguments, RowsArguments, UnlistedArguments};
use super::arguments::groups::types::{
    GroupCountArguments, GroupOrderArguments, GroupOrdinalArguments, ProjectArguments,
};
use super::arguments::joins::types::{
    AntijoinArguments, AsOfArguments, OrderedMatchArguments, UnionArguments,
};
use super::arguments::texts::types::{
    CaptureArguments, FitsRenderedArguments, RenderSyntaxArguments, ReplaceArguments,
    PrefixesArguments, SourceMatchesArguments, SpanBetweenArguments, SpanBoundsArguments,
};
use super::arguments::types::StepNumber;
use super::arguments::walks::types::{
    AxisArguments, EdgeArguments, OrderedDfsArguments, SubtreeClassArguments,
};
use super::expressions::types::{PlanExpression, SourceAlias};
use crate::domain::descriptions::types::{ColumnName, RelationName};
use crate::domain::runs::types::RuleId;

/// A finish plan: the steps every rule shares, what each rule that lowered emits, and why each other rule did not.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::types::FinishPlan;
///
/// let plan = FinishPlan { steps: Vec::new(), rules: Vec::new(), unplanned: Vec::new() };
/// assert!(plan.steps.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinishPlan {
    /// The steps, each reading only steps before it.
    pub steps: Vec<FinishStep>,
    /// The rules that lowered, in rule order.
    pub rules: Vec<PlannedRule>,
    /// The rules that did not, in rule order.
    pub unplanned: Vec<UnplannedRule>,
}

/// A rule that lowered, and what it emits.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::types::{PlannedRule, RuleEmit};
/// use ryo_values::domain::runs::types::RuleId;
///
/// let rule = PlannedRule { rule: RuleId("F401".into()), emit: RuleEmit::Unwritten };
/// assert_eq!(rule.emit, RuleEmit::Unwritten);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedRule {
    /// The rule.
    pub rule: RuleId,
    /// What it emits.
    pub emit: RuleEmit,
}

/// What a rule emits: the rows of a step, or nothing when its query writes no `emit`.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_values::domain::finish_plans::types::{EmitSubject, PlanEmit, RuleEmit};
///
/// let emit = RuleEmit::Written(PlanEmit { step: StepNumber(4), subject: EmitSubject("file".into()), key: Vec::new() });
/// assert_ne!(emit, RuleEmit::Unwritten);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleEmit {
    /// The query writes no `emit`.
    Unwritten,
    /// The query emits this.
    Written(PlanEmit),
}

/// A rule's emit: the step whose rows are its findings, what they are about, and the columns that tell two apart.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_values::domain::finish_plans::types::{EmitSubject, PlanEmit};
/// use ryo_values::domain::descriptions::types::ColumnName;
///
/// let emit = PlanEmit { step: StepNumber(4), subject: EmitSubject("file".into()), key: vec![ColumnName("span".into())] };
/// assert_eq!(emit.key.len(), 1);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanEmit {
    /// The step emitted.
    pub step: StepNumber,
    /// What a finding is about.
    pub subject: EmitSubject,
    /// The key columns.
    pub key: Vec<ColumnName>,
}

/// What an emit's findings are about, as the query writes it after `subject`: `file` when it writes none.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::types::EmitSubject;
///
/// assert_eq!(&*EmitSubject("file".into()).0, "file");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmitSubject(pub Box<str>);

/// A rule that did not lower, and why.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::types::{UnplannedReason, UnplannedRule};
/// use ryo_values::domain::runs::types::RuleId;
///
/// let rule = UnplannedRule { rule: RuleId("RUF100".into()), reason: UnplannedReason("unknown step x".into()) };
/// assert_eq!(&*rule.reason.0, "unknown step x");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnplannedRule {
    /// The rule.
    pub rule: RuleId,
    /// Why, in a sentence.
    pub reason: UnplannedReason,
}

/// Why a rule did not lower, in a sentence.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::types::UnplannedReason;
///
/// assert!(!UnplannedReason("unknown step x".into()).0.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnplannedReason(pub Box<str>);

/// One step of a plan: one kind per operator of the step set, a relation of the catalog, a select, or an operator
/// outside the set.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::types::FinishStep;
/// use ryo_values::domain::descriptions::types::RelationName;
///
/// let base = FinishStep::Base(RelationName("syntax-tree".into()));
/// assert!(matches!(base, FinishStep::Base(_)));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FinishStep {
    /// `base`: a relation of the catalog, by name.
    Base(RelationName),
    /// `select`: sources joined, tested and projected.
    Select(SelectStep),
    /// `union`.
    Union(UnionArguments),
    /// `antijoin`.
    Antijoin(AntijoinArguments),
    /// `as-of`.
    AsOf(AsOfArguments),
    /// `ordered-match`.
    OrderedMatch(OrderedMatchArguments),
    /// `axis`.
    Axis(AxisStep),
    /// `closure`.
    Closure(EdgeArguments),
    /// `follow`.
    Follow(EdgeArguments),
    /// `ordered-dfs`.
    OrderedDfs(OrderedDfsArguments),
    /// `group-count`.
    GroupCount(GroupCountArguments),
    /// `group-min`.
    GroupMin(GroupOrderArguments),
    /// `group-max`.
    GroupMax(GroupOrderArguments),
    /// `group-ordinal`.
    GroupOrdinal(GroupOrdinalArguments),
    /// `project`.
    Project(ProjectArguments),
    /// `extend`.
    Extend(ExtendArguments),
    /// `span-bounds`.
    SpanBounds(SpanBoundsArguments),
    /// `span-between`.
    SpanBetween(SpanBetweenArguments),
    /// `replace`.
    Replace(ReplaceArguments),
    /// `source-matches`.
    SourceMatches(SourceMatchesArguments),
    /// `capture`.
    Capture(CaptureArguments),
    /// `fits-rendered`.
    FitsRendered(FitsRenderedArguments),
    /// `render-syntax`.
    RenderSyntax(RenderSyntaxArguments),
    /// `rows`.
    Rows(RowsArguments),
    /// `prefixes`.
    Prefixes(PrefixesArguments),
    /// `subtree-class`.
    SubtreeClass(SubtreeClassArguments),
    /// An operator outside the step set.
    Unlisted(UnlistedArguments),
}

/// A select: its sources, the columns it keeps, its tests, and the planner's marks.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::types::{CountMark, ProbeMark, RowCondition, SelectStep, SelectedColumns};
///
/// let select = SelectStep {
///     sources: Vec::new(), columns: SelectedColumns::All, condition: RowCondition::Every,
///     probe: ProbeMark::Unmarked, count: CountMark::Unmarked,
/// };
/// assert_eq!(select.probe, ProbeMark::Unmarked);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SelectStep {
    /// The sources, in the order the query joins them.
    pub sources: Vec<SelectSource>,
    /// The columns kept.
    pub columns: SelectedColumns,
    /// The tests: the `on` keys and the `where`, as one condition.
    pub condition: RowCondition,
    /// Whether the planner runs it as a semijoin, and which source it probes.
    pub probe: ProbeMark,
    /// Whether the planner answers it as a count of the probed rows of each kept row.
    pub count: CountMark,
}

/// A source of a select: a step, under an alias or none.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_values::domain::finish_plans::expressions::types::SourceAlias;
/// use ryo_values::domain::finish_plans::types::SelectSource;
///
/// let source = SelectSource { step: StepNumber(0), alias: SourceAlias::Unwritten };
/// assert_eq!(source.step, StepNumber(0));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SelectSource {
    /// The step read.
    pub step: StepNumber,
    /// Its alias.
    pub alias: SourceAlias,
}

/// The columns a select keeps: every column of its sources (`*`), or the items it names.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::types::SelectedColumns;
///
/// assert_ne!(SelectedColumns::Items(Vec::new()), SelectedColumns::All);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SelectedColumns {
    /// `*`.
    All,
    /// These items, in order.
    Items(Vec<SelectItem>),
}

/// One item of a select: a column kept under a name, computed from the sources' columns.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::expressions::types::{PlanConstant, PlanExpression};
/// use ryo_values::domain::finish_plans::types::SelectItem;
/// use ryo_values::domain::descriptions::types::ColumnName;
///
/// let item = SelectItem { name: ColumnName("id".into()), value: PlanExpression::Literal(PlanConstant::Absent) };
/// assert_eq!(&*item.name.0, "id");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SelectItem {
    /// The name it is kept under.
    pub name: ColumnName,
    /// What it holds.
    pub value: PlanExpression,
}

/// A select's tests: none, or one condition every kept row passes.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::expressions::types::{PlanConstant, PlanExpression};
/// use ryo_values::domain::finish_plans::types::RowCondition;
///
/// assert_ne!(RowCondition::Tested(PlanExpression::Literal(PlanConstant::Absent)), RowCondition::Every);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RowCondition {
    /// No test: every row of the sources is kept.
    Every,
    /// This condition.
    Tested(PlanExpression),
}

/// The planner's semijoin mark on a two-source select: none, or the source probed (its place among the sources).
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::types::{ProbeMark, SourceOrdinal};
///
/// assert_ne!(ProbeMark::Probed(SourceOrdinal(1)), ProbeMark::Unmarked);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ProbeMark {
    /// Not a semijoin.
    Unmarked,
    /// This source is probed; the other is kept.
    Probed(SourceOrdinal),
    /// A necessary test from the sole consumer, using the original join order.
    Filtered(Box<PlanExpression>),
}

/// A source's place among a select's sources.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::types::SourceOrdinal;
///
/// assert_eq!(SourceOrdinal(1).0, 1);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SourceOrdinal(pub usize);

/// The planner's count mark on a select whose one reader counts its rows.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::types::CountMark;
///
/// assert_ne!(CountMark::Counted, CountMark::Unmarked);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CountMark {
    /// Made as rows.
    Unmarked,
    /// Never made: its reader is answered by counts.
    Counted,
}

/// An `axis` step: its arguments, and the planner's existence mark.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_values::domain::finish_plans::arguments::walks::types::AxisArguments;
/// use ryo_values::domain::finish_plans::types::{AxisStep, ExistsMark};
/// use ryo_values::domain::plan_parts::types::Direction;
///
/// let arguments = AxisArguments { from: StepNumber(2), relation: StepNumber(0), direction: Direction::Ancestor, domain: StepNumber(1) };
/// let axis = AxisStep { arguments, exists: ExistsMark::Unmarked };
/// assert_eq!(axis.exists, ExistsMark::Unmarked);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AxisStep {
    /// What it walks.
    pub arguments: AxisArguments,
    /// Whether a start's first admitted relative answers it.
    pub exists: ExistsMark,
}

/// The planner's existence mark on an axis: none, or a start keeps one row when it has an admitted relative.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::finish_plans::types::ExistsMark;
///
/// assert_ne!(ExistsMark::Marked, ExistsMark::Unmarked);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExistsMark {
    /// Every relative is a row.
    Unmarked,
    /// Its readers ask only whether a relative exists.
    Marked,
}
