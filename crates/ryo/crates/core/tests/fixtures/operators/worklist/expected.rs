// Every function raises itself from state 0 to state 1 through its one edge, followed once in state 0.
use super::{Finding, finding};
use ryo_language::inbound::use_cases::parse::fixtures::expected::{Cell, node, number};
/// The worklist operator's findings over the standard function fixture.
pub(super) fn expected() -> Vec<Finding> {
    vec![]
}
