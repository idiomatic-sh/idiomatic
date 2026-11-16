// Every function raises itself from state 0 to state 1 through its one edge, followed once in state 0.
use super::{Finding, finding};
use ryo_language::inbound::use_cases::parse::fixtures::expected::{Cell, node, number};
/// The worklist operator's findings over the standard function fixture.
pub(super) fn expected() -> Vec<Finding> {
    vec![
        finding(node(1), vec![Cell::from(number(0)), Cell::from(number(1))]),
        finding(node(14), vec![Cell::from(number(0)), Cell::from(number(1))]),
        finding(node(30), vec![Cell::from(number(0)), Cell::from(number(1))]),
        finding(node(44), vec![Cell::from(number(0)), Cell::from(number(1))]),
    ]
}
