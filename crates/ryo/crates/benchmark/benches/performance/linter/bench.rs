//! benchmark's linter workload: the shared linter contract run through both constructions of the ryo adapter.
use ryo_benchmark::outbound::linter::{
    factory::{ryo, ryo_context},
    fixtures::contract::check,
};
use ryo_benchmark_contracts::domain::requests::types::{ParserSelection, ProviderSelection};
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy;

#[divan::bench]
/// Measure preparation and decoding with the isolated default construction.
fn isolated(bencher: divan::Bencher<'_, '_>) {
    let adapter = ryo(
        "worker".into(),
        "rules".into(),
        std::collections::BTreeMap::new(),
        std::collections::BTreeMap::new(),
        ParserSelection::Unspecified,
        vec![ryo::linter::RuleId::try_from("fixture").expect("fixture rule")],
        std::collections::BTreeMap::new().into(),
    );
    bencher.bench_local(|| check(&adapter));
}
#[divan::bench]
/// Measure preparation and decoding with an explicit project context mode and selection policy.
fn contextual(bencher: divan::Bencher<'_, '_>) {
    let adapter = ryo_context(
        "worker".into(),
        "rules".into(),
        std::collections::BTreeMap::new(),
        std::collections::BTreeMap::new(),
        ParserSelection::Unspecified,
        ProviderSelection::Unspecified,
        ProjectContextMode::Isolated,
        vec![ryo::linter::RuleId::try_from("fixture").expect("fixture rule")],
        ObservationSelectionPolicy::Declared,
        std::collections::BTreeMap::new().into(),
    );
    bencher.bench_local(|| check(&adapter));
}
