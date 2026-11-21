//! The types of the `costing` concept: how a query's work grows with a file, the statement that sets it, and what a pack's
//! manifest declares of its rules' cost. Plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_values::domain::observations::types::BoundObservationInput;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::PlanStep;
use ryo_values::domain::query_tokens::types::{QuerySourceId, QuerySpan, StepSpelling};
use ryo_values::domain::runs::types::RuleId;

use super::constants::{
    A_SORT, LINEAR, LINEAR_WITH_SORTS, ORDERED_PAIRS, PRODUCT, QUADRATIC_IN_FILE,
    QUADRATIC_IN_GROUP,
};
use crate::domain::lowering::types::LoweredStep;

/// How a query's work grows with a file, whatever engine runs it. The classes are ordered: a later one is the greater cost.
///
/// # Examples
///
/// ```
/// use ryo_query_language::domain::costing::types::PlanCostClass;
///
/// assert!(PlanCostClass::QuadraticInFile > PlanCostClass::LinearWithSorts);
/// assert_eq!(PlanCostClass::QuadraticInGroup.to_string(), "quadratic in a group");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlanCostClass {
    /// In the rows it reads and gives.
    Linear,
    /// The same, with a sort of each group.
    LinearWithSorts,
    /// In the square of a key group's rows: a join of an input with itself on its keys and an order between a column and itself.
    QuadraticInGroup,
    /// In the square of the file's rows: a join with no key within the file.
    QuadraticInFile,
}

impl std::fmt::Display for PlanCostClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Linear => LINEAR,
            Self::LinearWithSorts => LINEAR_WITH_SORTS,
            Self::QuadraticInGroup => QUADRATIC_IN_GROUP,
            Self::QuadraticInFile => QUADRATIC_IN_FILE,
        })
    }
}

/// The shapes that put a statement above linear.
///
/// # Examples
///
/// ```
/// use ryo_query_language::domain::costing::types::CostShape;
///
/// assert_eq!(CostShape::Product.to_string(), "a join with no key within the file");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CostShape {
    /// A join whose keys do not go beyond the file, with no range on one column and neither side one row for a file.
    Product,
    /// A join of an input with itself on keys beyond the file, with an order between a column and itself.
    OrderedPairs,
    /// An operation that sorts its groups, or a join on the file alone with a range on one column.
    Sort,
}

impl std::fmt::Display for CostShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Product => PRODUCT,
            Self::OrderedPairs => ORDERED_PAIRS,
            Self::Sort => A_SORT,
        })
    }
}

/// Where a statement is written: the document and the name its `let` gives. The same in every query that imports the document,
/// whatever inputs and parameters the query passes, and unmoved by an edit anywhere else. Never the path of imports that
/// reached it.
///
/// # Examples
///
/// ```
/// use ryo_query_language::domain::costing::types::SiteOrigin;
/// use ryo_values::domain::query_tokens::types::{QuerySourceId, StepSpelling};
///
/// let site = SiteOrigin { document: QuerySourceId("pack/noqa".into()), statement: StepSpelling("file_matches".into()) };
/// assert_eq!(&*site.statement, "file_matches");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SiteOrigin {
    /// The document that writes the statement: a module's, or the query's own.
    pub document: QuerySourceId,
    /// The name the statement's `let` gives, which a document writes once.
    pub statement: StepSpelling,
}

/// Which kind of document writes a statement: a query's own document is named by whoever compiles it, so two queries may give
/// theirs one name, and a statement there is one query's alone.
///
/// # Examples
///
/// ```
/// use ryo_query_language::domain::costing::types::SiteHome;
///
/// assert_ne!(SiteHome::Module, SiteHome::Query);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SiteHome {
    /// A module: the same statement in every query that imports it.
    Module,
    /// The query's own document.
    Query,
}

/// One statement above linear.
///
/// # Examples
///
/// ```
/// use ryo_query_language::domain::costing::types::{CostFinding, CostShape, PlanCostClass, SiteHome, SiteOrigin};
/// use ryo_values::domain::query_tokens::types::{ByteOffset, QuerySourceId, QuerySpan, StepSpelling};
///
/// let finding = CostFinding {
///     site: SiteOrigin { document: QuerySourceId("<query>".into()), statement: StepSpelling("pairs".into()) },
///     home: SiteHome::Query,
///     span: QuerySpan { start: ByteOffset(0), end: ByteOffset(96) },
///     shape: CostShape::Product,
///     class: PlanCostClass::QuadraticInFile,
/// };
/// assert_eq!(finding.to_string(), "statement `pairs`: a join with no key within the file");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostFinding {
    /// The statement.
    pub site: SiteOrigin,
    /// Which kind of document writes it.
    pub home: SiteHome,
    /// Where it is written in its document, in bytes: the place a message shows, with the line the document's text gives it.
    /// Not part of the site: an edit above the statement moves this and not the site.
    pub span: QuerySpan,
    /// Its shape.
    pub shape: CostShape,
    /// The class the shape gives.
    pub class: PlanCostClass,
}

impl std::fmt::Display for CostFinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let statement = &self.site.statement;
        match self.home {
            SiteHome::Query => write!(f, "statement `{statement}`: {}", self.shape),
            SiteHome::Module => write!(
                f,
                "statement `{statement}` of `{}`: {}",
                self.site.document.0, self.shape
            ),
        }
    }
}

/// The cost of a compiled query, as its text states it: a compile answers it and refuses nothing for it.
///
/// # Examples
///
/// ```
/// use ryo_query_language::domain::costing::types::{PlanCost, PlanCostClass};
///
/// let cost = PlanCost { class: PlanCostClass::Linear, findings: Vec::new() };
/// assert!(cost.findings.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanCost {
    /// The greatest class of its statements; linear when none is above it.
    pub class: PlanCostClass,
    /// Each statement above linear that the query's roots reach, once, in the order of its site.
    pub findings: Vec<CostFinding>,
}

/// The cost of one rule of a pack.
///
/// # Examples
///
/// ```
/// use ryo_query_language::domain::costing::types::{PlanCost, PlanCostClass, RuleCost};
/// use ryo_values::domain::runs::types::RuleId;
///
/// let cost = RuleCost { rule: RuleId("F401".into()), cost: PlanCost { class: PlanCostClass::Linear, findings: Vec::new() } };
/// assert_eq!(&*cost.rule.0, "F401");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleCost {
    /// The rule.
    pub rule: RuleId,
    /// Its query's cost.
    pub cost: PlanCost,
}

/// Whose a site a pack lists is.
///
/// # Examples
///
/// ```
/// use ryo_query_language::domain::costing::types::SiteOwner;
/// use ryo_values::domain::runs::types::RuleId;
///
/// assert_ne!(SiteOwner::Pack, SiteOwner::Rule(RuleId("F401".into())));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SiteOwner {
    /// A module's statement: one site, whichever rules of the pack import it.
    Pack,
    /// A statement of this rule's own query, and of no other rule's.
    Rule(RuleId),
}

/// A site a pack's manifest lists.
///
/// # Examples
///
/// ```
/// use ryo_query_language::domain::costing::types::{ListedSite, PlanCostClass, SiteOrigin, SiteOwner};
/// use ryo_values::domain::query_tokens::types::{QuerySourceId, StepSpelling};
///
/// let site = SiteOrigin { document: QuerySourceId("pack/noqa".into()), statement: StepSpelling("file_matches".into()) };
/// let listed = ListedSite { site, owner: SiteOwner::Pack, class: PlanCostClass::QuadraticInFile };
/// assert_eq!(listed.class, PlanCostClass::QuadraticInFile);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListedSite {
    /// The site.
    pub site: SiteOrigin,
    /// Whose it is.
    pub owner: SiteOwner,
    /// The class it is listed at.
    pub class: PlanCostClass,
}

/// What a pack's manifest declares of its rules' cost.
///
/// # Examples
///
/// ```
/// use ryo_query_language::domain::costing::types::{PackCeiling, PlanCostClass};
///
/// let pack = PackCeiling { ceiling: PlanCostClass::LinearWithSorts, listed: Vec::new() };
/// assert!(pack.listed.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackCeiling {
    /// No site of the pack is above this.
    pub ceiling: PlanCostClass,
    /// Every site above `LinearWithSorts`, and no other.
    pub listed: Vec<ListedSite>,
}

/// How a pack fails its cost check.
///
/// # Examples
///
/// ```
/// use ryo_query_language::domain::costing::service::held;
/// use ryo_query_language::domain::costing::types::{CostBreach, ListedSite, PackCeiling, PlanCostClass, SiteOrigin, SiteOwner};
/// use ryo_values::domain::query_tokens::types::{QuerySourceId, StepSpelling};
///
/// let site = SiteOrigin { document: QuerySourceId("pack/pairs".into()), statement: StepSpelling("pairs".into()) };
/// let listed = ListedSite { site, owner: SiteOwner::Pack, class: PlanCostClass::QuadraticInFile };
/// let pack = PackCeiling { ceiling: PlanCostClass::QuadraticInFile, listed: vec![listed.clone()] };
/// assert_eq!(held(&[], &pack), vec![CostBreach::Stale(listed)]);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CostBreach {
    /// A rule reaches a site above the pack's ceiling.
    AboveCeiling(RuleId, SiteOrigin),
    /// A rule reaches a site above `LinearWithSorts` that is not listed.
    Unlisted(RuleId, SiteOrigin),
    /// A rule reaches a listed site at a class above the one it is listed at.
    Rose(ListedSite),
    /// A listed site that no rule reaches above `LinearWithSorts` any more, or an entry that repeats one: the list only shrinks.
    Stale(ListedSite),
}

/// Where a statement is written and which kind of document writes it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct WrittenSite {
    /// Where.
    pub site: SiteOrigin,
    /// Which kind of document.
    pub home: SiteHome,
    /// Where the statement is written in that document, in bytes.
    pub span: QuerySpan,
}

/// A query's steps as the compiler assembled them, before it rewrites them: each still the work of one statement.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AssembledSteps<'a> {
    /// The steps.
    pub steps: &'a [PlanStep],
    /// The columns each step answers, by its place.
    pub lowered: &'a [LoweredStep],
    /// The inputs the query's producers bind, whose columns a step may read.
    pub observations: &'a [BoundObservationInput],
    /// The statement each step was written by, or none for a step no statement wrote.
    pub written: &'a [Option<WrittenSite>],
    /// The steps the query emits and observes: a step none of them reads is not classified.
    pub roots: &'a [StepId],
}

/// Whether one side of a join is proved to hold each value of the join's keys once, from the catalog and the operations alone
/// (`distinctness`): on the file alone that is one row for a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SideRows {
    /// A side meets one row at most for each row of the other: the join gives no more rows than that other side has.
    Bounded,
    /// Not proved of either side.
    Unproved,
}

/// What a statement's shape is and the class it gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ShapeClass {
    /// The shape.
    pub shape: CostShape,
    /// The class.
    pub class: PlanCostClass,
}

/// What the keys of a join say of the pairs it makes, before the `where` of its statement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct JoinFacts {
    /// Whether its keys go beyond the file.
    pub keys: JoinKeys,
    /// Whether one of its sides is proved one row at most for each value of the keys.
    pub side: SideRows,
}

/// Whether the columns a join equates go beyond the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum JoinKeys {
    /// Every column it equates holds the file: within a file, every row of one side meets every row of the other.
    FileAlone,
    /// It equates a column that holds something else.
    Beyond,
}

/// What the `where` of a join's statement adds to its keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Refining {
    /// Whether it equates a column of one side with a column of the other that holds more than the file.
    pub keys: JoinKeys,
    /// Whether it bounds one column of one side from below and from above by columns of the other.
    pub range: RangeBound,
    /// Whether the join is of an input with itself and it orders a column of one side against the same column of the other.
    pub order: SelfOrder,
}

/// Whether a `where` bounds one column of one side of a join on both ends by the other side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RangeBound {
    /// One column has a lower and an upper comparison against the other side.
    TwoSided,
    /// No column has both.
    Open,
}

/// Whether a `where` orders a column of one side of a join of an input with itself against the same column of the other side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelfOrder {
    /// It does.
    Ordered,
    /// It does not.
    Unordered,
}

/// Whether the `where` of a join's statement has been read into its verdict yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Filtering {
    /// No filter of the statement was read: the verdict is of the join's keys alone.
    Bare,
    /// A filter was read: the verdict is the greatest of the alternatives read so far.
    Filtered,
}

/// What the classifier keeps of a step it has read, for the steps after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CostedStep {
    /// Its shape and class when it is above linear.
    pub verdict: Option<ShapeClass>,
    /// What its keys say when it is a join, for the `where` that follows it in its statement.
    pub join: Option<JoinFacts>,
    /// Whether that `where` was read.
    pub filtering: Filtering,
}
