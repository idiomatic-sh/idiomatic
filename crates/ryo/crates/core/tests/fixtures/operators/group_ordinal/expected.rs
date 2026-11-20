// The findings the rule of `operators/group_ordinal/input.ryoql` must give over the `function` fixture, in any order.

use super::{Finding, finding};
use ryo_language::inbound::use_cases::parse::fixtures::expected::{Cell, file, name, number};

/// Each function's place in the file by where it starts, from 0. The input is a join that holds every function four times: a repeated row
/// is one row, so the places are 0 to 3 and not multiples of four.
pub(super) fn expected() -> Vec<Finding> {
    let placed = |function: &'static str, place: i64| -> Vec<Cell> {
        vec![name(function).into(), number(place).into()]
    };
    vec![
        finding(file("function/input.rs"), placed("outer", 0)),
        finding(file("function/input.rs"), placed("inner", 1)),
        finding(file("function/input.rs"), placed("run", 2)),
        finding(file("function/input.rs"), placed("run", 3)),
    ]
}
