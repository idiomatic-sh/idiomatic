use super::{Finding, finding};
use ryo_language::inbound::use_cases::parse::fixtures::expected::{file, span};

pub(super) fn expected() -> Vec<Finding> {
    vec![
        finding(file("function/input.rs"), vec![span((1, 1), (1, 3))]),
        finding(file("function/input.rs"), vec![span((1, 14), (1, 16))]),
        finding(file("function/input.rs"), vec![span((1, 14), (1, 16))]),
        finding(file("function/input.rs"), vec![span((3, 1), (3, 3))]),
        finding(file("function/input.rs"), vec![span((5, 1), (5, 3))]),
    ]
}
