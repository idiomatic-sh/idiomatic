//! Builds the analysis adapters, one function each. A factory builds nothing else, chooses nothing, and is called only by
//! an edge, which links the languages and hands them over.

/// The ryo adapter: ryo's SDK, `query::validate` and `linter::lint`, over the languages the edge linked, keeping nothing
/// between its lint calls.
#[must_use]
pub fn ryo(
    languages: ryo::linter::Languages,
    settings: super::types::AnalysisSettings,
) -> impl crate::outbound::analysis::interface::AnalysisPort {
    super::adapters::ryo::adapter::AnalysisRyoAdapter::new(
        languages,
        settings,
        ryo::linter::Cache::None,
    )
}

/// The ryo adapter as `ryo` builds it, keeping what a lint call leaves for the next in ryo's own directory of `store`,
/// under the build that runs; under `-n/--dry-run` it keeps nothing.
#[must_use]
pub fn ryo_kept(
    languages: ryo::linter::Languages,
    settings: super::types::AnalysisSettings,
    store: &crate::domain::settings::types::Store,
) -> impl crate::outbound::analysis::interface::AnalysisPort + use<> {
    super::adapters::ryo::adapter::AnalysisRyoAdapter::kept(languages, settings, store)
}
