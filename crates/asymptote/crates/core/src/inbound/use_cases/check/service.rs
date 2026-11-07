//! The `check` use case's own helper: the runs the next report keeps.

use crate::domain::judgement::types::Status;
use crate::domain::manifest::types::Tool;
use crate::domain::reports::service::{replaced, retained};
use crate::domain::reports::types::Scanned;
use crate::domain::violations::types::ToolId;
use crate::outbound::check_store::types::KeptReport;

/// The runs the next report keeps: `scanned`, the completed runs of the configured tools not among `selected` that the
/// report before it held, so a check of some groups forgets nothing of the others, and the completed run a failed
/// tool's run replaces, so the failure never costs the tool its reuse.
pub(super) fn keeping(
    previous: &KeptReport,
    configured: &[Tool],
    selected: &[Tool],
    scanned: Vec<Scanned>,
) -> Vec<Scanned> {
    let ids = |tools: &[Tool]| {
        tools
            .iter()
            .map(|tool| tool.id.clone())
            .collect::<Vec<ToolId>>()
    };
    let failed: Vec<ToolId> = scanned
        .iter()
        .filter(|scan| scan.run.status != Status::Completed)
        .map(|scan| scan.run.tool.clone())
        .collect();
    let mut kept = scanned;
    if let KeptReport::Kept(report) = previous {
        kept.extend(retained(&report.scanned, &ids(configured), &ids(selected)));
        kept.extend(replaced(&report.scanned, &failed));
    }
    kept
}

/// The current scans as command answers, preserving captured failures independently of stored history.
pub(super) fn answered(
    scanned: &[Scanned],
    reuses: Vec<super::types::Reuse>,
) -> Vec<super::types::ToolRun> {
    scanned
        .iter()
        .zip(reuses)
        .map(|(scan, reuse)| super::types::ToolRun {
            failure_output: scan.failure_output.clone(),
            run: scan.run.clone(),
            reuse,
        })
        .collect()
}
