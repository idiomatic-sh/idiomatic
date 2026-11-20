// The findings the rule of `operators/follow/input.ryoql` must give over the `function` fixture, in any order.

use super::{Finding, finding};
use ryo_language::inbound::use_cases::parse::fixtures::expected::node;

/// `outer` points to itself; `inner` points to `outer` and to the two functions named `run`, which point nowhere. So `inner`
/// reaches `outer`, `outer` answers itself, and the walks that run out of edges at a `run` have no row.
pub(super) fn expected() -> Vec<Finding> {
    vec![
        finding(node(1), vec![node(1)]),
        finding(node(14), vec![node(1)]),
    ]
}
