// The findings the rule of `operators/intersection/input.ryoql` must give over the `function` fixture, in any order.

use super::{Finding, finding};
use ryo_language::inbound::use_cases::parse::fixtures::expected::{file, span};

/// The findings the operator's rule gives over the `function` fixture, as this fixture states them.
pub(super) fn expected() -> Vec<Finding> {
    vec![
        finding(file("function/input.rs"), vec![span((3, 1), (3, 12))]),
        finding(file("function/input.rs"), vec![span((5, 1), (5, 12))]),
    ]
}
