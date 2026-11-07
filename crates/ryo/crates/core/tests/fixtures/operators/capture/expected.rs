// The findings the rule of `operators/capture/input.ryoql` must give over the `function` fixture, in any order.

use super::{Finding, finding};
use ryo_language::inbound::use_cases::parse::fixtures::expected::{file, name};

/// The findings the operator's rule gives over the `function` fixture, as this fixture states them.
pub(super) fn expected() -> Vec<Finding> {
    vec![
        finding(file("function/input.rs"), vec![name("inner")]),
        finding(file("function/input.rs"), vec![name("outer")]),
        finding(file("function/input.rs"), vec![name("run")]),
    ]
}
