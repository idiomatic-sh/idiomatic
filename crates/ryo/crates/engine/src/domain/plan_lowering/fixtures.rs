//! The small plan the passes' cases run on: two relations, the select shapes each pass admits or refuses, and the axes.
//! Its JSON is the plan the Python passes (`pass_semi.py`, `pass_count.py`, `pass_axis.py`) were run on; every mark a
//! case expects is the mark they wrote.
//!
//! Steps: 0 `a`, 1 `b`; 2 a select keeping `l` read by a union (3); 4 a select keeping `r`, emitted; 5 a select reading
//! both sources; 6 a select with no test of both; 7 a select whose one reader is a group-count (8); 9 a select read by
//! a distinct group-count (10); 11 an ancestor axis read through a select (12) that keeps `id`; 13 a descendant axis
//! whose `related` a select (14) keeps; 15 a parent axis.

use ryo_values::domain::finish_plans::arguments::groups::types::{CountedRows, GroupCountArguments};
use ryo_values::domain::finish_plans::arguments::joins::types::UnionArguments;
use ryo_values::domain::finish_plans::arguments::types::StepNumber;
use ryo_values::domain::finish_plans::arguments::walks::types::AxisArguments;
use ryo_values::domain::finish_plans::expressions::types::{
    ColumnRead, Comparator, PlanConstant, PlanExpression, RowComparison, SourceAlias,
};
use ryo_values::domain::finish_plans::types::{
    AxisStep, CountMark, EmitSubject, ExistsMark, FinishPlan, FinishStep, PlanEmit, PlannedRule, ProbeMark,
    RowCondition, RuleEmit, SelectItem, SelectSource, SelectStep, SelectedColumns,
};
use ryo_values::domain::descriptions::types::{ColumnName, RelationName};
use ryo_values::domain::plan_parts::types::{Direction, LiteralText};
use ryo_values::domain::query_tokens::types::AliasName;
use ryo_values::domain::runs::types::RuleId;

fn name(text: &str) -> ColumnName {
    ColumnName(text.into())
}

fn alias(text: Option<&str>) -> SourceAlias {
    text.map_or(SourceAlias::Unwritten, |text| SourceAlias::Written(AliasName(text.into())))
}

fn column(column: &str, from: Option<&str>) -> PlanExpression {
    PlanExpression::Column(ColumnRead { column: name(column), alias: alias(from) })
}

fn equal(left: PlanExpression, right: PlanExpression) -> PlanExpression {
    PlanExpression::Compare(RowComparison {
        comparator: Comparator::Equal,
        left: Box::new(left),
        right: Box::new(right),
    })
}

/// A select of `sources` keeping `items` (name, column, alias) under `condition`, unmarked.
fn select(sources: &[(usize, Option<&str>)], items: &[(&str, &str, Option<&str>)], condition: Option<PlanExpression>) -> FinishStep {
    FinishStep::Select(SelectStep {
        sources: sources
            .iter()
            .map(|(step, written)| SelectSource { step: StepNumber(*step), alias: alias(*written) })
            .collect(),
        columns: SelectedColumns::Items(
            items
                .iter()
                .map(|(kept, read, from)| SelectItem { name: name(kept), value: column(read, *from) })
                .collect(),
        ),
        condition: condition.map_or(RowCondition::Every, RowCondition::Tested),
        probe: ProbeMark::Unmarked,
        count: CountMark::Unmarked,
    })
}

/// A select of `a` as `l` joined with `b` as `r`, keeping `id` of `kept`, tested on `l.<on> = r.<on>`.
fn joined(kept: &str, on: &str) -> FinishStep {
    select(
        &[(0, Some("l")), (1, Some("r"))],
        &[("id", "id", Some(kept))],
        Some(equal(column(on, Some("l")), column(on, Some("r")))),
    )
}

fn group_count(from: usize, distinct: CountedRows) -> FinishStep {
    FinishStep::GroupCount(GroupCountArguments { from: StepNumber(from), keys: vec![name("id")], count: name("n"), distinct })
}

fn axis(from: usize, relation: usize, direction: Direction, domain: usize) -> FinishStep {
    FinishStep::Axis(AxisStep {
        arguments: AxisArguments {
            from: StepNumber(from),
            relation: StepNumber(relation),
            direction,
            domain: StepNumber(domain),
        },
        exists: ExistsMark::Unmarked,
    })
}

/// The passes' small plan, before any pass.
pub(crate) fn small_plan() -> FinishPlan {
    let steps = vec![
        FinishStep::Base(RelationName("a".into())),
        FinishStep::Base(RelationName("b".into())),
        joined("l", "id"),
        FinishStep::Union(UnionArguments { left: StepNumber(2), right: StepNumber(2) }),
        joined("r", "id"),
        select(
            &[(0, Some("l")), (1, Some("r"))],
            &[("id", "id", Some("l")), ("other", "id", Some("r"))],
            Some(equal(column("id", Some("l")), column("id", Some("r")))),
        ),
        select(
            &[(0, Some("l")), (1, Some("r"))],
            &[("id", "id", Some("l"))],
            Some(equal(column("kind", Some("l")), PlanExpression::Literal(PlanConstant::Text(LiteralText("x".into()))))),
        ),
        joined("l", "file"),
        group_count(7, CountedRows::Every),
        joined("l", "span"),
        group_count(9, CountedRows::Distinct(name("id"))),
        axis(0, 1, Direction::Ancestor, 0),
        select(&[(11, None)], &[("id", "id", None)], None),
        axis(1, 1, Direction::Descendant, 0),
        select(&[(13, None)], &[("related", "related", None)], None),
        axis(0, 0, Direction::Parent, 1),
    ];
    let emitted = [(3, "id"), (4, "id"), (5, "id"), (6, "id"), (8, "id"), (10, "id"), (12, "id"), (14, "related"), (15, "id")];
    let rules = emitted
        .iter()
        .enumerate()
        .map(|(place, (step, key))| PlannedRule {
            rule: RuleId(format!("R{}", place + 1).into()),
            emit: RuleEmit::Written(PlanEmit { step: StepNumber(*step), subject: EmitSubject("file".into()), key: vec![name(key)] }),
        })
        .collect();
    FinishPlan { steps, rules, unplanned: Vec::new() }
}

/// Each select's marks, by step: the probed source (or none) and whether it is counted.
pub(crate) fn select_marks(plan: &FinishPlan) -> Vec<(usize, Option<usize>, bool)> {
    plan.steps
        .iter()
        .enumerate()
        .filter_map(|(place, step)| match step {
            FinishStep::Select(select) => Some((
                place,
                match select.probe {
                    ProbeMark::Unmarked => None,
                    ProbeMark::Probed(source) => Some(source.0),
                },
                select.count == CountMark::Counted,
            )),
            _ => None,
        })
        .collect()
}

/// The axes marked for existence, by step.
pub(crate) fn existence_marks(plan: &FinishPlan) -> Vec<usize> {
    plan.steps
        .iter()
        .enumerate()
        .filter_map(|(place, step)| matches!(step, FinishStep::Axis(axis) if axis.exists == ExistsMark::Marked).then_some(place))
        .collect()
}
