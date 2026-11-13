// Every function's explicit self-edge is visited, even though the root is marked seen.
use super::{Finding, finding};
use ryo_language::inbound::use_cases::parse::fixtures::expected::{Cell, node, number};
/// The ordered DFS operator's findings over the standard function fixture.
pub(super) fn expected() -> Vec<Finding> {
    vec![
        finding(node(1), vec![Cell::from(node(1)), Cell::from(number(0))]),
        finding(node(14), vec![Cell::from(node(14)), Cell::from(number(0))]),
        finding(node(30), vec![Cell::from(node(30)), Cell::from(number(0))]),
        finding(node(44), vec![Cell::from(node(44)), Cell::from(number(0))]),
    ]
}
