//! The `project` service: free functions named by verb over what the rules read of the project, with no IO and no ports.

use super::types::PlanReads;
use ryo_relations::domain::relations::service::provenance_of;
use ryo_relations::domain::relations::types::Provenance;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::records::types::Freshness;
use ryo_values::domain::row_texts::types::PlacedRule;
use ryo_values::domain::run_metadata::types::{ProjectRead, RuleRunMetadata};
use ryo_values::domain::stores::types::MadeAgain;

/// Whether the cross-file steps of a plan read what the project writes - the manifests and how the files link to one another.
#[must_use]
pub(crate) fn reads_project(plan: &Plan) -> PlanReads {
    if plan
        .frontier
        .iter()
        .any(|input| matches!(input, Input::Relation(relation) if provenance_of(*relation) == Provenance::Manifests))
    {
        PlanReads::Reads
    } else {
        PlanReads::DoesNotRead
    }
}

/// Whether the cross-file steps of any of the plans read what the project writes.
#[must_use]
pub(crate) fn reads_any(metadata: &[RuleRunMetadata]) -> PlanReads {
    if metadata
        .iter()
        .any(|rule| rule.reads_project == ProjectRead::Reads)
    {
        PlanReads::Reads
    } else {
        PlanReads::DoesNotRead
    }
}

/// The rules that read what the project writes, by their place among `plans`, in order.
#[must_use]
pub(crate) fn project_rules(metadata: &[RuleRunMetadata]) -> Vec<PlacedRule> {
    metadata
        .iter()
        .enumerate()
        .filter(|(_, rule)| rule.reads_project == ProjectRead::Reads)
        .map(|(place, _)| PlacedRule(u32::try_from(place).unwrap_or(u32::MAX)))
        .collect()
}

/// The rules made again across files: the ones `made` names, and, when a file the project's lookup read beyond the manifests is gone
/// or changed (`consulted`), every rule that reads what the project writes, `project`: what the project comes to may change with
/// files no run lists (a configuration above the repository), and the store keeps their digests, so the project is read again only
/// then.
#[must_use]
pub(crate) fn with_consulted(
    made: MadeAgain,
    project: &[PlacedRule],
    consulted: Freshness,
) -> MadeAgain {
    match (made, consulted) {
        (made, Freshness::AsItWas) => made,
        (MadeAgain::All, Freshness::Changed) => MadeAgain::All,
        (MadeAgain::Nothing, Freshness::Changed) if project.is_empty() => MadeAgain::Nothing,
        (MadeAgain::Nothing, Freshness::Changed) => MadeAgain::These(project.to_vec()),
        (MadeAgain::These(mut rules), Freshness::Changed) => {
            rules.extend_from_slice(project);
            rules.sort_by_key(|rule| rule.0);
            rules.dedup();
            MadeAgain::These(rules)
        }
    }
}

/// A primary manifest read failure makes demanded project relations incomplete, without
/// attributing its disk error to the unrelated source paths those relations describe.
pub(crate) fn unread_manifest_partials(
    wanted: &[ryo_relations::domain::relations::types::Relation],
    path: &ryo_language::domain::files::types::FilePath,
    detail: &ryo_values::domain::files::types::InputReadFailure,
) -> Vec<ryo_execution::inbound::use_cases::metadata::interface::Partial> {
    wanted
        .iter()
        .filter(|relation| provenance_of(**relation) == Provenance::Manifests)
        .map(
            |relation| ryo_execution::inbound::use_cases::metadata::interface::Partial {
                relation: *relation,
                cause: ryo_values::domain::stored_facts::types::PartialCause(
                    format!("cannot read primary manifest `{}`: {}", path.0, detail.0).into(),
                ),
                lacking: ryo_execution::inbound::use_cases::metadata::interface::Lacking::Any,
                at: ryo_execution::inbound::use_cases::metadata::interface::PartialPlace::Repository,
            },
        )
        .collect()
}
