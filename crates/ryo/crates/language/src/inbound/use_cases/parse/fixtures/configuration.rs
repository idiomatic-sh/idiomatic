//! Fixture-only selection of a parser and matching independent reference callbacks.
use super::super::types::ParseEngine;
use super::expected::{Oracle, Shape};
use super::findings::Findings;
use std::sync::Arc;

/// One configured fixture context; all three consumers must share its language settings.
pub struct ConfiguredFixture {
    /// Parser created by the language's fixture setup, never mutated by this harness.
    pub parse: Arc<dyn ParseEngine>,
    /// Independent tree reference for the same settings.
    pub shape: Shape,
    /// Independent row reference for the same settings.
    pub oracle: Oracle,
}

impl std::fmt::Debug for ConfiguredFixture {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ConfiguredFixture")
            .finish_non_exhaustive()
    }
}

/// A language-owned fixture selector; no metadata syntax is interpreted by the generic harness.
pub type Configuration<'a> = dyn Fn(&str) -> Result<Option<ConfiguredFixture>, String> + 'a;

/// Default parser and optional language-owned selection, held for one fixture run.
pub(super) struct Selection<'a> {
    /// Existing parser used when the source declares no override.
    pub default: &'a dyn ParseEngine,
    /// Validation and construction performed before any selected parser or reference callback.
    pub configure: &'a Configuration<'a>,
}

impl Selection<'_> {
    /// Validate one input once and retain the selected context for every operation on that input.
    pub(super) fn select(
        &self,
        text: &str,
        shown: &str,
        findings: &mut Findings,
    ) -> Result<Option<ConfiguredFixture>, ()> {
        (self.configure)(text).map_err(|error| {
            findings
                .0
                .push(format!("{shown}: invalid fixture configuration: {error}"));
        })
    }
}

#[cfg(test)]
#[path = "configuration_test.rs"]
mod tests;
