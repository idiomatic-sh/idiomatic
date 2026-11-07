//! The `up` service: local free-function helpers, private to this use case.

use super::types::UpRequest;
use crate::domain::operations::types::{Folded, PerfRecord};
use crate::domain::reports::service::measured;
use crate::domain::reports::types::Report;
use crate::domain::seeding::service::seeding;

/// The perf records the last check's medians seed or raise, when the selectors leave perf to be chosen: they are
/// chosen by the tool selectors alone, as a rule, a path or a place names a finding.
pub(super) fn seeded(request: &UpRequest, folded: &Folded, report: &Report) -> Vec<PerfRecord> {
    let selectors = &request.selectors;
    if !(selectors.rules.is_empty() && selectors.paths.is_empty() && selectors.at.is_empty()) {
        return Vec::new();
    }
    seeding(
        &measured(&report.scanned),
        &folded.perf,
        &selectors.tools,
        &request.reason,
    )
}
