//! The `explain` service: local free-function helpers, private to this use case.

use super::errors::ExplainError;
use tastemaker_values::domain::applications::types::Plan;
use tastemaker_values::domain::definitions::types::{ErrorFile, LoadedWorkspace};
use tastemaker_values::domain::explanations::constants::{AUTHORING_TITLE, TOPICS_TITLE};
use tastemaker_values::domain::explanations::service::workspace_sections;
use tastemaker_values::domain::explanations::types::{
    Section, SectionLine, SectionTitle, Tally, Totals,
};
use tastemaker_values::domain::files::types::SkippedDirectory;
use tastemaker_values::domain::placement::types::Placement;
use tastemaker_values::domain::topics::constants::{OPERATOR_WORD, RELATION_WORD};
use tastemaker_values::domain::topics::service::{authoring, names};

/// What `explain` says asked nothing: the workspace's totals and scopes, every topic, and how each kind of asset is
/// written.
pub(super) fn overview(
    workspace: &LoadedWorkspace,
    placement: &Placement,
    planned: &Plan,
    skipped: &[SkippedDirectory],
) -> Vec<Section> {
    let mut sections = workspace_sections(
        &workspace.section.name,
        Totals {
            governed: Tally(workspace.files.len()),
            placed: Tally(placement.placed.len()),
            rules: Tally(planned.rules.len()),
        },
        &placement.scopes,
        skipped,
    );
    let kinds = [RELATION_WORD, OPERATOR_WORD]
        .into_iter()
        .map(|kind| SectionLine(format!("- tm explain {kind}/<name>, tm list {kind}s").into()));
    sections.push(Section {
        title: SectionTitle(TOPICS_TITLE.into()),
        lines: names()
            .into_iter()
            .map(|name| SectionLine(format!("- tm explain {}", name.0).into()))
            .chain(kinds)
            .collect(),
    });
    sections.push(Section {
        title: SectionTitle(AUTHORING_TITLE.into()),
        lines: authoring()
            .into_iter()
            .map(|line| SectionLine(line.0.into()))
            .collect(),
    });
    sections
}

/// Turns a manifest failure into the error that names `file`.
pub(super) fn manifest_error(
    file: ErrorFile<'_>,
) -> impl Fn(crate::outbound::manifest::errors::ManifestError) -> ExplainError + '_ {
    move |source| ExplainError::Manifest {
        file: file.0.to_owned(),
        source,
    }
}
