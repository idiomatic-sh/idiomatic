//! The steps the `refusal_order` programs are made of: a hosted text, steps whose operands are refused, and their readers.
use crate::*;
use ryo_values::domain::plan_parts::types::{
    HostedNumber, JoinColumn, LiteralText, RepeatedRows, Side,
};

/// Step `at` of the plan.
pub(in crate::refusal_order) const fn s(at: u32) -> Input {
    Input::Step(StepId(at))
}

/// The hosted text.
pub(in crate::refusal_order) const fn h() -> Operation {
    Operation::Hosted {
        relation: HostedNumber(0),
    }
}

/// The words of column 99 of step `at`: a column past its input, refused by its kernel at the first row.
pub(in crate::refusal_order) const fn w99(at: u32) -> Operation {
    Operation::Words {
        from: s(at),
        text: ColumnIndex(99),
    }
}

/// A replace of step `at`'s column 0 by a pattern that is no regular expression: refused at the first row.
pub(in crate::refusal_order) fn bad(at: u32) -> Operation {
    Operation::Replace {
        from: s(at),
        text: ColumnIndex(0),
        pattern: LiteralText("(".into()),
        replacement: LiteralText("b".into()),
    }
}

/// A projection of step `at`'s `columns`.
pub(in crate::refusal_order) fn p(at: u32, columns: &[u32]) -> Operation {
    Operation::Project {
        from: s(at),
        columns: columns.iter().copied().map(ColumnIndex).collect(),
        distinct: RepeatedRows::Kept,
    }
}

/// The union of steps `left` and `right`.
pub(in crate::refusal_order) const fn u(left: u32, right: u32) -> Operation {
    Operation::Union {
        left: s(left),
        right: s(right),
    }
}

/// The difference of steps `left` and `right`.
pub(in crate::refusal_order) const fn d(left: u32, right: u32) -> Operation {
    Operation::Difference {
        left: s(left),
        right: s(right),
    }
}

/// A join of step `at` to the function relation, step column `key` against the function's node, keeping the step's column 0.
pub(in crate::refusal_order) fn join_functions(at: u32, key: u32) -> Operation {
    Operation::Join {
        left: s(at),
        right: Input::Relation(Relation::Function),
        left_on: vec![ColumnIndex(key)],
        right_on: vec![ColumnIndex(0)],
        output: vec![JoinColumn {
            side: Side::Left,
            column: ColumnIndex(0),
        }],
        filter: vec![],
        view: vec![],
        reach: None,
    }
}
