// The findings the rule of `operators/rank/input.ryoql` must give over the `function` fixture, in any order: the layers are `run`, then `inner` and `outer`, so the functions are in layers 0 and 1.

use super::{Finding, finding};
use ryo_language::inbound::use_cases::parse::fixtures::expected::{file, number};

/// The findings the operator's rule gives over the `function` fixture, as this fixture states them.
pub(super) fn expected() -> Vec<Finding> {
    vec![
        finding(file("function/input.rs"), vec![number(0)]),
        finding(file("function/input.rs"), vec![number(1)]),
    ]
}
