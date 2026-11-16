//! Every evidence destination gathers costs, verification times and growth into one document, written once by `finish`.
use crate::outbound::evidence::interface::EvidencePort;

/// Run the evidence observations against one supplied destination.
/// # Panics
/// Panics if the destination violates the evidence contract.
pub fn check(adapter: &impl EvidencePort) {
    let report = super::sample::report();
    assert_eq!(
        adapter
            .comparison(
                &report,
                &report,
                ryo_benchmark_contracts::domain::reports::types::ReportComparison {
                    diagnostics: ryo_benchmark_contracts::domain::reports::types::Comparison::Exact,
                    execution_metadata:
                        ryo_benchmark_contracts::domain::reports::types::Comparison::Exact,
                },
                ryo_benchmark_contracts::domain::reports::types::ComparisonContract::Diagnostics,
            )
            .map_err(|_invalid_boundary_value| ()),
        Ok(()),
        "an exact comparison of two complete reports is written"
    );
    let language: ryo_benchmark_contracts::domain::workloads::types::suite_language::value::SuiteLanguage =
        "python".into();
    assert!(
        adapter
            .costs(
                &language,
                &ryo_benchmark_contracts::domain::costs::types::ProjectCosts(Vec::new())
            )
            .is_err(),
        "an empty set of costs is refused"
    );
    assert_eq!(
        adapter
            .costs(&language, &super::sample::costs())
            .map_err(|_invalid_boundary_value| ()),
        Ok(()),
        "a project's costs are added"
    );
    assert_eq!(
        adapter
            .verification(&language, &super::sample::timings())
            .map_err(|_invalid_boundary_value| ()),
        Ok(()),
        "a rule's verification times are added"
    );
    assert_eq!(
        adapter
            .growth(&language, &super::sample::growth())
            .map_err(|_invalid_boundary_value| ()),
        Ok(()),
        "a shape's growth is added"
    );
    assert_eq!(
        adapter
            .read_verification(&crate::outbound::evidence::types::VerificationTimesFiles::default())
            .map_err(|_invalid_boundary_value| ()),
        Ok(crate::outbound::evidence::types::RuleTimings::default()),
        "no verification file gives no times"
    );
    assert_eq!(
        adapter.finish().map_err(|_invalid_boundary_value| ()),
        Ok(()),
        "the document of what was added is written"
    );
}
