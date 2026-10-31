// Independent expected names from the function source fixture.
use super::{Finding, finding};
use ryo_language::inbound::use_cases::parse::fixtures::expected::{file, name};

pub(super) fn expected() -> Vec<Finding> {
    vec![
        finding(file("function/input.rs"), vec![name("inneR")]),
        finding(file("function/input.rs"), vec![name("outeR")]),
        finding(file("function/input.rs"), vec![name("Run")]),
    ]
}
