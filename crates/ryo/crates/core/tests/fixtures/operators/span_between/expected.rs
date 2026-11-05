// Independent locations of the function fixture, including both repeated names.
use super::{Finding, finding};
use ryo_language::inbound::use_cases::parse::fixtures::expected::{file, span};

/// Each original function interval is retained by self composition.
pub(super) fn expected() -> Vec<Finding> {
    vec![
        finding(file("function/input.rs"), vec![span((1, 1), (1, 29))]),
        finding(file("function/input.rs"), vec![span((1, 14), (1, 27))]),
        finding(file("function/input.rs"), vec![span((3, 1), (3, 12))]),
        finding(file("function/input.rs"), vec![span((5, 1), (5, 12))]),
    ]
}
