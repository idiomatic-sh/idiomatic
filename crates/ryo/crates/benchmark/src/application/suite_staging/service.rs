//! Resolve executable dependencies before copying a selected suite snapshot.
use crate::domain::observations::service as observations;
use crate::outbound::suite::{interface::SuitePort, types::ArtifactPath};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::StagePurpose;
use std::collections::BTreeSet;
use std::path::Path;
/// Resolve executable observation dependencies before the outbound snapshot copies bytes.
pub(crate) fn stage(
    suite: &dyn SuitePort,
    directory: &Path,
    destination: &Path,
    authored: &[ryo::linter::RuleId],
    purpose: StagePurpose,
) -> Result<(), Error> {
    let tool = suite.profile(ArtifactPath(directory))?;
    suite.verify_query_modules(ArtifactPath(directory), &tool)?;
    let mut definitions: BTreeSet<_> = authored
        .iter()
        .cloned()
        .chain(tool.supporting_rules.iter().cloned())
        .collect();
    let defaults = observations::project_profile(
        &tool.rule_observations,
        authored,
        tool.rule_observation_selection,
    );
    definitions.extend(observations::closure(authored.iter().cloned(), |id| {
        defaults.get(id).map(Vec::as_slice)
    }));
    for owner in authored {
        let cases = suite.rule_cases(
            ArtifactPath(&directory.join(crate::domain::suite::constants::RULES)),
            owner,
        )?;
        for case in cases.cases {
            let selected = match &case.selected_rules {
                ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(rules) => rules.as_slice(),
                ryo_benchmark_contracts::domain::requests::types::CohortSelection::Frozen => authored,
            };
            let profile_declarations = observations::project_profile(
                &tool.rule_observations,
                selected,
                tool.rule_observation_selection,
            );
            let declarations = match &case.rule_observations {
                ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Declared(
                    declared,
                ) => declared,
                ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared => {
                    &profile_declarations
                }
            };
            definitions.extend(observations::closure(selected.iter().cloned(), |id| {
                declarations.get(id).map(Vec::as_slice)
            }));
        }
    }
    suite.stage(
        ArtifactPath(directory),
        ArtifactPath(destination),
        authored,
        &definitions,
        purpose,
    )
}
