// The findings the rule of `operators/closure/input.ryoql` must give over the `function` fixture, in any order.

use super::{Finding, finding};
use ryo_language::inbound::use_cases::parse::fixtures::expected::node;

/// The findings the operator's rule gives over the `function` fixture, as this fixture states them.
pub(super) fn expected() -> Vec<Finding> {
    vec![
        finding(node(1), vec![node(1)]),
        finding(node(14), vec![node(14)]),
        finding(node(30), vec![node(30)]),
        finding(node(44), vec![node(44)]),
    ]
}
