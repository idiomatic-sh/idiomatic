//! Every public submission route, enumerated from its source inventory, refuses an
//! incomplete or extra root set; per-rule metadata is reachable only through its counted accessor.
use super::fixtures::{Compilation, Expectation, Probe};
mod controls;
mod inventory;
mod routes;
mod witness;
const PROGRAM: &str = "src/application/file_dag/program.rs";
pub(super) fn controls() -> Vec<Probe> {
    vec![
        Probe {
            compilation: Compilation::Library,
            owner: PROGRAM,
            source: r#"
mod counted_metadata {
    use super::preparation::types::PreparedProgram;
    use crate::domain::{coverage::types::RuleIndex, operating::errors::OperatingError};
    fn read(value: &PreparedProgram, rule: RuleIndex) -> Result<(), OperatingError> {
        let _ = value.rule_metadata(rule)?;
        Ok(())
    }
    const _: fn(&PreparedProgram, RuleIndex) -> Result<(), OperatingError> = read;
}
"#
            .into(),
            expected: Expectation::Accept,
        },
        Probe {
            compilation: Compilation::Library,
            owner: PROGRAM,
            source: "mod raw_rules { use super::preparation::types::PreparedProgram; fn read(value: &PreparedProgram) { let _ = &value.rules; } const _: fn(&PreparedProgram) = read; }".into(),
            expected: Expectation::Refuse {
                code: "E0616",
                first: "rules",
                second: "private",
            },
        },
    ]
}
