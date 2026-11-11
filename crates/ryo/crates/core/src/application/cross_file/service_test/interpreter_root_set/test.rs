//! A set of project roots prepares once and delivers in order.

use super::fixtures::{
    project_root_set_prepares_shared_publications_once_before_ordered_delivery,
    project_root_set_preserves_distinct_actual_population_policy_in_both_orders,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::rule_requests::errors::RuleRequestsError;

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `project_root_set_prepares_shared_publications_once_before_ordered_delivery` scenario.
    ProjectRootSetPreparesSharedPublicationsOnceBeforeOrderedDelivery,
    /// The `project_root_set_preserves_distinct_actual_population_policy_in_both_orders` scenario.
    ProjectRootSetPreservesDistinctActualPopulationPolicyInBothOrders,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `|`.
type Value = &'static str;
/// The rule requests a scenario's run refuses.
type Failure = RuleRequestsError;

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "project_root_set_prepares_shared_publications_once_before_ordered_delivery",
        ),
        word: Word::Succeeds,
        input: || Given::ProjectRootSetPreparesSharedPublicationsOnceBeforeOrderedDelivery,
        expect: Expect::Value(
            "3|true|true|true|1|1|1|2|1|3|Some(Rendered { text: RenderedText(\"rulerule\"), cells: [Some(Text(TextRange { start: RowsTextAt(0), end: RowsTextAt(4) })), Some(Number(Integer(2))), Some(Number(Integer(0))), None, None, Some(Text(TextRange { start: RowsTextAt(4), end: RowsTextAt(8) })), Some(Number(Integer(1))), Some(Number(Integer(0))), None, None], ends: RowEnds([5, 10]), order: [RowOrder { rule: RuleNumber(0), line: FileLine(0), column: ColumnNumber(0), row: RowNumber(1) }, RowOrder { rule: RuleNumber(0), line: FileLine(0), column: ColumnNumber(0), row: RowNumber(0) }], total: RowTotal(0), keys: [] })|Some(Rendered { text: RenderedText(\"rule-1rule-1\"), cells: [Some(Text(TextRange { start: RowsTextAt(0), end: RowsTextAt(6) })), Some(Number(Integer(2))), Some(Number(Integer(0))), None, None, Some(Text(TextRange { start: RowsTextAt(6), end: RowsTextAt(12) })), Some(Number(Integer(1))), Some(Number(Integer(0))), None, None], ends: RowEnds([5, 10]), order: [RowOrder { rule: RuleNumber(1), line: FileLine(0), column: ColumnNumber(0), row: RowNumber(1) }, RowOrder { rule: RuleNumber(1), line: FileLine(0), column: ColumnNumber(0), row: RowNumber(0) }], total: RowTotal(0), keys: [] })|Some(Rendered { text: RenderedText(\"rulerule\"), cells: [Some(Text(TextRange { start: RowsTextAt(0), end: RowsTextAt(4) })), Some(Number(Integer(2))), Some(Number(Integer(0))), None, None, Some(Text(TextRange { start: RowsTextAt(4), end: RowsTextAt(8) })), Some(Number(Integer(1))), Some(Number(Integer(0))), None, None], ends: RowEnds([5, 10]), order: [RowOrder { rule: RuleNumber(0), line: FileLine(0), column: ColumnNumber(0), row: RowNumber(1) }, RowOrder { rule: RuleNumber(0), line: FileLine(0), column: ColumnNumber(0), row: RowNumber(0) }], total: RowTotal(0), keys: [] })|ReadinessCounts { preparations: ReadinessEvents(1), evaluations: ReadinessEvents(1), suspensions: ReadinessEvents(0), wakeups: ReadinessEvents(0), presence_absent: ReadinessEvents(0) }|1|2|3|true|true|true|1|1|1|2|1|3|Some(Rendered { text: RenderedText(\"rule-1rule-1\"), cells: [Some(Text(TextRange { start: RowsTextAt(0), end: RowsTextAt(6) })), Some(Number(Integer(2))), Some(Number(Integer(0))), None, None, Some(Text(TextRange { start: RowsTextAt(6), end: RowsTextAt(12) })), Some(Number(Integer(1))), Some(Number(Integer(0))), None, None], ends: RowEnds([5, 10]), order: [RowOrder { rule: RuleNumber(1), line: FileLine(0), column: ColumnNumber(0), row: RowNumber(1) }, RowOrder { rule: RuleNumber(1), line: FileLine(0), column: ColumnNumber(0), row: RowNumber(0) }], total: RowTotal(0), keys: [] })|Some(Rendered { text: RenderedText(\"rulerule\"), cells: [Some(Text(TextRange { start: RowsTextAt(0), end: RowsTextAt(4) })), Some(Number(Integer(2))), Some(Number(Integer(0))), None, None, Some(Text(TextRange { start: RowsTextAt(4), end: RowsTextAt(8) })), Some(Number(Integer(1))), Some(Number(Integer(0))), None, None], ends: RowEnds([5, 10]), order: [RowOrder { rule: RuleNumber(0), line: FileLine(0), column: ColumnNumber(0), row: RowNumber(1) }, RowOrder { rule: RuleNumber(0), line: FileLine(0), column: ColumnNumber(0), row: RowNumber(0) }], total: RowTotal(0), keys: [] })|Some(Rendered { text: RenderedText(\"rule-1rule-1\"), cells: [Some(Text(TextRange { start: RowsTextAt(0), end: RowsTextAt(6) })), Some(Number(Integer(2))), Some(Number(Integer(0))), None, None, Some(Text(TextRange { start: RowsTextAt(6), end: RowsTextAt(12) })), Some(Number(Integer(1))), Some(Number(Integer(0))), None, None], ends: RowEnds([5, 10]), order: [RowOrder { rule: RuleNumber(1), line: FileLine(0), column: ColumnNumber(0), row: RowNumber(1) }, RowOrder { rule: RuleNumber(1), line: FileLine(0), column: ColumnNumber(0), row: RowNumber(0) }], total: RowTotal(0), keys: [] })|ReadinessCounts { preparations: ReadinessEvents(1), evaluations: ReadinessEvents(1), suspensions: ReadinessEvents(0), wakeups: ReadinessEvents(0), presence_absent: ReadinessEvents(0) }|1|2",
        ),
    },
    Case {
        name: CaseName(
            "project_root_set_preserves_distinct_actual_population_policy_in_both_orders",
        ),
        word: Word::Succeeds,
        input: || Given::ProjectRootSetPreservesDistinctActualPopulationPolicyInBothOrders,
        expect: Expect::Value("2|true|true|2|2|2|3|2|true|true|2|2|2|3"),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::ProjectRootSetPreparesSharedPublicationsOnceBeforeOrderedDelivery => {
            project_root_set_prepares_shared_publications_once_before_ordered_delivery()?
        }
        Given::ProjectRootSetPreservesDistinctActualPopulationPolicyInBothOrders => {
            project_root_set_preserves_distinct_actual_population_policy_in_both_orders()?
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
