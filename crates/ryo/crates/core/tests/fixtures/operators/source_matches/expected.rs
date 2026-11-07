// The nested occurrence of `fn` at 1:14..1:16 is found twice by the operator and is one row here: `emit a subject file key(match_span)`
// collapses rows of an identical rule, subject and key, the contract of the alpha (commit 1cd5de7fea); the call-count test compares the rows
// as the answer gives them, so the golden states the collapsed rows.
use super::{Finding, finding};
use ryo_language::inbound::use_cases::parse::fixtures::expected::{file, span};

pub(super) fn expected() -> Vec<Finding> {
    vec![
        finding(file("function/input.rs"), vec![span((1, 1), (1, 3))]),
        finding(file("function/input.rs"), vec![span((1, 14), (1, 16))]),
        finding(file("function/input.rs"), vec![span((3, 1), (3, 3))]),
        finding(file("function/input.rs"), vec![span((5, 1), (5, 3))]),
    ]
}
