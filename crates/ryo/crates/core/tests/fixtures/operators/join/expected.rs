// The findings the rule of `operators/join/input.ryoql` must give over the `function` fixture, in any order.

use super::{Finding, finding};
use ryo_language::inbound::use_cases::parse::fixtures::expected::{file, node};

/// The findings the operator's rule gives over the `function` fixture, as this fixture states them.
pub(super) fn expected() -> Vec<Finding> {
    vec![
        finding(file("function/input.rs"), vec![node(1)]),
        finding(file("function/input.rs"), vec![node(14)]),
        finding(file("function/input.rs"), vec![node(30)]),
        finding(file("function/input.rs"), vec![node(44)]),
    ]
}
