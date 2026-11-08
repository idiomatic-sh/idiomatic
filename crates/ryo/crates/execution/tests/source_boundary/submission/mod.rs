//! Per-rule metadata is reachable only through its counted accessor, and the frontier preparation only
//! through the request-owned cache.
use super::fixtures::{Compilation, Expectation, Probe};
const PROGRAM: &str = "src/application/file_dag/program.rs";
const REQUEST: &str = "src/inbound/use_cases/execution/use_case.rs";
pub(super) fn controls() -> Vec<Probe> {
    vec![
        Probe {
            compilation: Compilation::Library,
            owner: PROGRAM,
            source: r#"
mod counted_metadata {
    use super::preparation::types::PreparedProgram;
    use ryo_execution_domain::domain::{coverage::types::RuleIndex, operating::errors::OperatingError};
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
        // The provider read is reachable only through the request-owned cache.
        Probe {
            compilation: Compilation::Library,
            owner: REQUEST,
            source: "mod cached_frontiers { const _: () = { let _ = crate::application::file_dag::frontiers::types::FrontierCache::prepared; }; }".into(),
            expected: Expectation::Accept,
        },
        Probe {
            compilation: Compilation::Library,
            owner: PROGRAM,
            source: "mod uncached_frontiers { const _: () = { let _ = crate::application::file_dag::frontiers::service::prepare; }; }".into(),
            expected: Expectation::Refuse {
                code: "E0603",
                first: "prepare",
                second: "private",
            },
        },
        Probe {
            compilation: Compilation::Library,
            owner: REQUEST,
            source: "mod uncached_frontiers_from_request { const _: () = { let _ = crate::application::file_dag::frontiers::service::prepare; }; }".into(),
            expected: Expectation::Refuse {
                code: "E0603",
                first: "prepare",
                second: "private",
            },
        },
    ]
}
