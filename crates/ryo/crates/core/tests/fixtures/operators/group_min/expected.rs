// The findings the rule of `operators/group_min/input.ryoql` must give over the `function` fixture, in any order.

use super::{Finding, finding};
use ryo_language::inbound::use_cases::parse::fixtures::expected::{file, span};

/// The first function of each name: of the two named `run`, the one that starts first.
pub(super) fn expected() -> Vec<Finding> {
    vec![
        finding(file("function/input.rs"), vec![span((1, 1), (1, 29))]),
        finding(file("function/input.rs"), vec![span((1, 14), (1, 27))]),
        finding(file("function/input.rs"), vec![span((3, 1), (3, 12))]),
    ]
}
