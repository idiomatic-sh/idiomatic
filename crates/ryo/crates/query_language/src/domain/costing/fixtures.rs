//! Plans written by hand for the `costing` cases: the steps, the columns each answers and the statement that wrote each, and
//! the cost `classified` reads from them, on one line.

use super::service::classified;
use super::types::{AssembledSteps, PlanCost, SiteHome, SiteOrigin, WrittenSite};
use crate::domain::lowering::types::{LoweredStep, PlannedColumn, StepLabel, Uniqueness};
use ryo_relations::domain::catalog::service::relation_size;
use ryo_relations::domain::relations::types::{CellKind, Relation, RelationSize};
use ryo_relations::domain::terms::types::Vocabulary;
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as Reads, JoinColumn, Operand, PlanComparison, Side,
    StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::query_tokens::types::{QuerySourceId, StepSpelling};

/// The catalog's sizes, with the token links declared a row for a pair of elements: no relation is so declared today.
pub(crate) fn sizes(relation: Relation) -> Option<RelationSize> {
    if relation == Relation::TokenLink {
        Some(RelationSize::PerPair)
    } else {
        relation_size(relation)
    }
}

/// A plan written by hand: its steps, the columns of each and the statement that wrote each.
#[derive(Default)]
pub(crate) struct Written {
    /// The steps.
    pub steps: Vec<PlanStep>,
    /// The columns each answers.
    pub lowered: Vec<LoweredStep>,
    /// The statement that wrote each.
    pub written: Vec<Option<WrittenSite>>,
}

/// Adds to `plan` a step the statement `statement` of the query's own document wrote, answering columns of `kinds`, and
/// answers its place.
pub(crate) fn push(
    plan: &mut Written,
    statement: &str,
    operation: Operation,
    kinds: &[CellKind],
) -> Reads {
    let place = StepId(u32::try_from(plan.steps.len()).unwrap_or(u32::MAX));
    plan.steps.push(PlanStep {
        operation,
        scope: StepScope::PerFile,
    });
    plan.lowered.push(LoweredStep {
        name: StepLabel::Unnamed,
        fields: kinds
            .iter()
            .map(|kind| PlannedColumn {
                name: ColumnName("column".into()),
                kind: *kind,
                unique: Uniqueness::Shared,
                words: Vocabulary::Open,
            })
            .collect(),
        scope: StepScope::PerFile,
    });
    plan.written.push(Some(WrittenSite {
        site: SiteOrigin {
            document: QuerySourceId("<query>".into()),
            statement: StepSpelling(statement.into()),
        },
        home: SiteHome::Query,
    }));
    Reads::Step(place)
}

/// A cost on one line: its class, then each finding as its statement, its shape and its class.
pub(crate) fn rendered(cost: &PlanCost) -> String {
    let findings = cost
        .findings
        .iter()
        .map(|finding| {
            let (shape, class) = (finding.shape, finding.class);
            format!("{} {shape:?} {class:?}", finding.site.statement)
        })
        .collect::<Vec<_>>();
    format!("{:?} [{}]", cost.class, findings.join("; "))
}

/// The cost of `plan` read from the step `root`, on one line.
pub(crate) fn cost(plan: &Written, root: Reads) -> String {
    let Reads::Step(root) = root else {
        return String::from("the root is no step");
    };
    rendered(&classified(
        AssembledSteps {
            steps: &plan.steps,
            lowered: &plan.lowered,
            observations: &[],
            written: &plan.written,
            roots: &[root],
        },
        sizes,
    ))
}

/// The node, the name, the file and the span of a function; a comment's are its node, its text, its file and its span.
pub(crate) const ROW: &[CellKind] = &[
    CellKind::Node,
    CellKind::Symbol,
    CellKind::File,
    CellKind::Span,
];
/// The file of both.
pub(crate) const FILE: ColumnIndex = ColumnIndex(2);
/// The functions.
pub(crate) const FUNCTIONS: Reads = Reads::Relation(Relation::Function);
/// The comments.
pub(crate) const COMMENTS: Reads = Reads::Relation(Relation::Comment);

/// A join of `left` and `right` on these columns, giving the left node, the left span, the right node and the right span.
pub(crate) fn join(
    left: Reads,
    right: Reads,
    left_on: Vec<ColumnIndex>,
    right_on: Vec<ColumnIndex>,
) -> Operation {
    let column = |side, column| JoinColumn {
        side,
        column: ColumnIndex(column),
    };
    Operation::Join {
        left,
        right,
        left_on,
        right_on,
        output: vec![
            column(Side::Left, 0),
            column(Side::Left, 3),
            column(Side::Right, 0),
            column(Side::Right, 3),
        ],
        filter: Vec::new(),
        view: Vec::new(),
        reach: None,
    }
}

/// The rows of `from` that these tests of two of its columns hold for.
pub(crate) fn filter(from: Reads, tests: &[(Test, u32, u32)]) -> Operation {
    Operation::Select {
        from,
        comparisons: tests
            .iter()
            .map(|(test, left, right)| PlanComparison {
                test: *test,
                left: Operand::Column(ColumnIndex(*left)),
                right: Operand::Column(ColumnIndex(*right)),
                origin: ComparisonOrigin::Unrecorded,
            })
            .collect(),
        kept: Vec::new(),
    }
}
