// Independent expected names from the function source fixture.
use super::{Finding, finding};
use ryo_language::inbound::use_cases::parse::fixtures::expected::{file, name};

/// Every function has one ordered request followed by its reply.
pub(super) fn expected() -> Vec<Finding> {
    vec![
        finding(file("function/input.rs"), vec![name("inner")]),
        finding(file("function/input.rs"), vec![name("outer")]),
        finding(file("function/input.rs"), vec![name("run")]),
    ]
}
