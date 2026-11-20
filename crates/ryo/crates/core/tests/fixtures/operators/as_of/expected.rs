// The findings the rule of `operators/as_of/input.ryoql` must give over the `function` fixture, in any order.

use super::{Finding, finding};
use ryo_language::inbound::use_cases::parse::fixtures::expected::{Cell, file, name, number};

/// Each function is paired with the function that starts latest before it, once for each time it is asked: the probes are
/// a join that holds every function twice (once for each `run`), so every pair is counted twice, and the first function,
/// with none before it, has no row. The two functions named `run` are one group of the count: two pairs each with `inner`
/// and with the first `run`.
pub(super) fn expected() -> Vec<Finding> {
    let paired = |function: &'static str, earlier: &'static str| -> Vec<Cell> {
        vec![
            name(function).into(),
            name(earlier).into(),
            number(2).into(),
        ]
    };
    vec![
        finding(file("function/input.rs"), paired("inner", "outer")),
        finding(file("function/input.rs"), paired("run", "inner")),
        finding(file("function/input.rs"), paired("run", "run")),
    ]
}
