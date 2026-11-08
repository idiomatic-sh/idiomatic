//! Actual-source privacy and same-input dense activation falsifiers.
use super::fixtures::{Compilation, Expectation, Probe};
mod fixtures;
mod test;
pub(super) const SETUP: &str = "src/application/file_dag/derived/owner/setup.rs";
pub(super) const ANCHOR: &str = "let mut wanted = Vec::new();";
pub(super) fn controls() -> Vec<Probe> {
    let mut controls = vec![Probe {
        compilation: Compilation::Library,
        owner: SETUP,
        source: r#"
mod targeted_activation {
    use crate::application::file_dag::derived::prepared::RequestDerived;
    use ryo_execution_domain::domain::{coverage::types::RuleIndex, operating::errors::OperatingError};
    fn read(value: &RequestDerived, rule: RuleIndex) -> Result<(), OperatingError> {
        let _ = value.relations_for(rule)?;
        let _ = value.graph_for(rule)?;
        Ok(())
    }
    const _: fn(&RequestDerived, RuleIndex) -> Result<(), OperatingError> = read;
}
"#.into(),
        expected: Expectation::Accept,
    }, Probe {
        compilation: Compilation::WantedMutation,
        owner: SETUP,
        source: "for (index, relations) in self.relations.iter().enumerate() { if !demanded.contains(&RuleIndex(index)) { continue; } std::hint::black_box(relations); }".into(),
        expected: Expectation::Refuse { code: "E0616", first: "relations", second: "private" },
    }];
    for field in ["relations", "graph_rules"] {
        controls.push(Probe {
            compilation: Compilation::Library,
            owner: SETUP,
            source: format!("mod raw_{field} {{ use crate::application::file_dag::derived::prepared::RequestDerived as Metadata; fn read(value: &Metadata) {{ let _ = &value.{field}; }} const _: fn(&Metadata) = read; }}"),
            expected: Expectation::Refuse { code: "E0616", first: field, second: "private" },
        });
    }
    controls
}
