//! The `idioms` service: the pairs whose two functions have equal sets of texts, said as a comparison of
//! digests instead of two antijoins.

use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::terms::types::Vocabulary;

use crate::domain::lowering::constants::{Added, added};
use crate::domain::lowering::types::{
    Builder, PairFiltering, PlannedColumn, SameSetsJoin, SameSetsPush, Uniqueness, Widened,
};
use crate::domain::planning::service::{push, pushed};
use crate::domain::resolving::service::{column_of, step_of};
use crate::domain::rewriting::types::SameSets;
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plans::types as plans;
use ryo_values::domain::program::types::{self as syntax, QuerySpelling};

/// Says the pairs whose two functions have equal sets of texts as a comparison of digests: each
/// function's texts are reduced in its own file to one digest and one size, and what the files
/// hand over is those rows, never the texts.
pub(crate) fn push_same_sets(builder: &mut Builder<'_>, name: &syntax::StepName, idiom: SameSets) {
    if push_same_sets_joined(builder, name, idiom) == SameSetsPush::Joined {
        return;
    }
    let pair_fields = step_of(&builder.lowered, idiom.pairs).fields.clone();
    let digest = push_digests(builder, idiom);
    let Widened {
        output,
        fields: first_fields,
    } = widen(
        &pair_fields,
        &[
            QuerySpelling(added(Added::Digest)),
            QuerySpelling(added(Added::Size)),
        ],
    );
    let first = pushed(
        builder,
        None,
        plans::Operation::Join {
            left: parts::Input::Step(idiom.pairs),
            right: parts::Input::Step(digest),
            left_on: vec![idiom.function],
            right_on: vec![parts::ColumnIndex(0)],
            output,
            filter: Vec::new(),
            view: Vec::new(),

            reach: None,
        },
        first_fields.clone(),
    );
    let Widened {
        output: both_output,
        fields: both_fields,
    } = widen(
        &first_fields,
        &[
            QuerySpelling(added(Added::OtherDigest)),
            QuerySpelling(added(Added::OtherSize)),
        ],
    );
    let both = pushed(
        builder,
        None,
        plans::Operation::Join {
            left: parts::Input::Step(first),
            right: parts::Input::Step(digest),
            left_on: vec![idiom.other],
            right_on: vec![parts::ColumnIndex(0)],
            output: both_output,
            filter: Vec::new(),
            view: Vec::new(),

            reach: None,
        },
        both_fields.clone(),
    );
    let wide = u32::try_from(pair_fields.len()).unwrap_or(u32::MAX);
    let same = |left: parts::ColumnIndex, right: parts::ColumnIndex| parts::PlanComparison {
        test: parts::Test::Equal,
        left: parts::Operand::Column(left),
        right: parts::Operand::Column(right),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    let equal = pushed(
        builder,
        None,
        plans::Operation::Select {
            from: parts::Input::Step(both),
            comparisons: vec![
                same(parts::ColumnIndex(wide), parts::ColumnIndex(wide + 2)),
                same(parts::ColumnIndex(wide + 1), parts::ColumnIndex(wide + 3)),
            ],
            kept: Vec::new(),
        },
        both_fields,
    );
    let kept = (0..wide).map(parts::ColumnIndex).collect();
    push(
        builder,
        Some(name),
        plans::Operation::Project {
            from: parts::Input::Step(equal),
            columns: kept,
            distinct: parts::RepeatedRows::Dropped,
        },
        pair_fields,
    );
}

/// The pairs, when they are the functions of one step met on their name, said instead as the functions
/// met on their name and their digest: a name many functions share would otherwise pair them all,
/// each to be compared after. Answers whether it did.
pub(crate) fn push_same_sets_joined(
    builder: &mut Builder<'_>,
    name: &syntax::StepName,
    idiom: SameSets,
) -> SameSetsPush {
    let Some(found) = recognize_same_sets(builder, idiom) else {
        return SameSetsPush::Separate;
    };
    let SameSetsJoin {
        met,
        narrowing,
        left,
        first,
        left_on,
        right_on,
        output,
    } = found;
    let large_fields = step_of(&builder.lowered, left).fields.clone();
    let met_fields = step_of(&builder.lowered, met).fields.clone();
    let pair_fields = step_of(&builder.lowered, idiom.pairs).fields.clone();
    let digest = push_digests(builder, idiom);
    let width = u32::try_from(large_fields.len()).unwrap_or(u32::MAX);
    let Widened {
        output: columns,
        fields: widened,
    } = widen(
        &large_fields,
        &[
            QuerySpelling(added(Added::Digest)),
            QuerySpelling(added(Added::Size)),
        ],
    );
    let with_digests = pushed(
        builder,
        None,
        plans::Operation::Join {
            left: parts::Input::Step(left),
            right: parts::Input::Step(digest),
            left_on: vec![first],
            right_on: vec![parts::ColumnIndex(0)],
            output: columns,
            filter: Vec::new(),
            view: Vec::new(),

            reach: None,
        },
        widened,
    );
    let (mut by, mut against) = (left_on, right_on);
    by.extend([width, width + 1].map(parts::ColumnIndex));
    against.extend([width, width + 1].map(parts::ColumnIndex));
    let alike = pushed(
        builder,
        None,
        plans::Operation::Join {
            left: parts::Input::Step(with_digests),
            right: parts::Input::Step(with_digests),
            left_on: by,
            right_on: against,
            output,
            filter: Vec::new(),
            view: Vec::new(),

            reach: None,
        },
        met_fields,
    );
    let pairs = match narrowing {
        PairFiltering::Filtered(comparisons) => pushed(
            builder,
            None,
            plans::Operation::Select {
                from: parts::Input::Step(alike),
                comparisons,
                kept: Vec::new(),
            },
            pair_fields.clone(),
        ),
        PairFiltering::Unfiltered => alike,
    };
    let kept = (0..u32::try_from(pair_fields.len()).unwrap_or(u32::MAX))
        .map(parts::ColumnIndex)
        .collect();
    push(
        builder,
        Some(name),
        plans::Operation::Project {
            from: parts::Input::Step(pairs),
            columns: kept,
            distinct: parts::RepeatedRows::Dropped,
        },
        pair_fields,
    );
    SameSetsPush::Joined
}

/// A column of numbers the digest steps add, open and shared.
fn number(label: QuerySpelling<'_>) -> PlannedColumn {
    PlannedColumn {
        name: syntax::ColumnName(Box::from(label.0)),
        kind: CellKind::Number,
        unique: Uniqueness::Shared,
        words: Vocabulary::Open,
    }
}

/// Reduces each function's texts in its own file to one digest and one size: the step both sayings of the idiom join against.
fn push_digests(builder: &mut Builder<'_>, idiom: SameSets) -> parts::StepId {
    let owner_field =
        column_of(&step_of(&builder.lowered, idiom.texts).fields, idiom.owner).clone();
    pushed(
        builder,
        None,
        plans::Operation::GroupDigest {
            from: parts::Input::Step(idiom.texts),
            keys: vec![idiom.owner],
            over: idiom.text,
        },
        vec![
            owner_field,
            number(QuerySpelling(added(Added::Digest))),
            number(QuerySpelling(added(Added::Size))),
        ],
    )
}

/// The columns of a join that keeps every column of `fields` and adds the digest and the size of the right side, and the fields it
/// answers: `fields` and one number for each of `extra`.
fn widen(fields: &[PlannedColumn], extra: &[QuerySpelling<'_>]) -> Widened {
    let mut output: Vec<parts::JoinColumn> = (0..fields.len())
        .map(|place| parts::JoinColumn {
            side: parts::Side::Left,
            column: parts::ColumnIndex(u32::try_from(place).unwrap_or(u32::MAX)),
        })
        .collect();
    output.extend([1, 2].map(|place| parts::JoinColumn {
        side: parts::Side::Right,
        column: parts::ColumnIndex(place),
    }));
    let mut kept = fields.to_vec();
    kept.extend(extra.iter().map(|label| number(*label)));
    Widened {
        output,
        fields: kept,
    }
}

/// The join of the functions on their name that the pairs come from, when the pairs are that and nothing else.
pub(crate) fn recognize_same_sets(builder: &Builder<'_>, idiom: SameSets) -> Option<SameSetsJoin> {
    let index = |id: parts::StepId| usize::try_from(id.0).unwrap_or(usize::MAX);
    let place = |column: parts::ColumnIndex| usize::try_from(column.0).unwrap_or(usize::MAX);
    let (met, narrowing) = match builder
        .steps
        .get(index(idiom.pairs))
        .map(|step| &step.operation)
    {
        Some(plans::Operation::Select {
            from: parts::Input::Step(met),
            comparisons,
            kept,
        }) if kept.is_empty() => (*met, PairFiltering::Filtered(comparisons.clone())),
        Some(plans::Operation::Join { .. }) => (idiom.pairs, PairFiltering::Unfiltered),
        _ => return None,
    };
    let Some(plans::Operation::Join {
        left: parts::Input::Step(left),
        right: parts::Input::Step(right),
        left_on,
        right_on,
        output,
        filter,
        view,

        reach: None,
    }) = builder
        .steps
        .get(index(met))
        .map(|step| step.operation.clone())
    else {
        return None;
    };
    let (Some(first), Some(second)) = (
        output.get(place(idiom.function)),
        output.get(place(idiom.other)),
    ) else {
        return None;
    };
    let same_function = first.side == parts::Side::Left
        && second.side == parts::Side::Right
        && first.column == second.column;
    if left != right || !same_function || !filter.is_empty() || !view.is_empty() {
        return None;
    }
    Some(SameSetsJoin {
        met,
        narrowing,
        left,
        first: first.column,
        left_on,
        right_on,
        output,
    })
}
