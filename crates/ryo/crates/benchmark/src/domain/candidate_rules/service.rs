//! Project declared configuration data into existing SDK hosted inputs and exact rule exclusions.
use super::types::CandidateConfiguration;
use crate::domain::candidate_rules::constants;
use ryo::linter::{
    Cell, Glob, HostColumn, HostColumnName, HostKind, HostRelation, HostRelationName, HostRow,
    HostText, Rule, RuleId,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::PairCount;
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, RuleContextPreparation, WorkerRequest,
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn project(
    request: &WorkerRequest,
    rules: &mut BTreeMap<RuleId, Rule>,
    selected_pairs: PairCount,
) -> Result<CandidateConfiguration, Error> {
    let RuleContextPreparation::Prepared(prepared) = &request.rule_context else {
        return Ok(CandidateConfiguration {
            hosted: Vec::new(),
            selected_pairs,
        });
    };
    crate::domain::rule_context::service::check(prepared, &request.files)?;
    let CohortSelection::Explicit(selected) = &request.selected_rules else {
        return Err(Error::Invalid(
            constants::RULE_CONTEXT_REQUIRES_THE_EXPLICIT_CONFIGURE.into(),
        ));
    };
    let context = &prepared.configuration;
    for (file, disabled) in &context.disabled_rules {
        let pattern = crate::domain::rule_context::service::literal_glob(file);
        for id in disabled {
            // Configuration can describe codes outside this execution cohort.
            let Some(rule) = rules.get_mut(id) else {
                continue;
            };
            rule.exclude.push(
                Glob::try_from(pattern.0.as_str())
                    .map_err(|error| Error::Invalid(error.to_string()))?,
            );
        }
    }
    let column = |name: HostColumnName, kind| HostColumn { name, kind };
    let table = |name: HostRelationName, columns, rows| HostRelation {
        name,
        columns,
        rows,
    };
    let selected_rows = selected
        .iter()
        .map(|id| HostRow(vec![Cell::Text(HostText(id.as_ref().into()))]))
        .collect();
    let disabled_rows = context
        .disabled_rules
        .iter()
        .flat_map(|(file, rules)| {
            rules.iter().map(move |id| {
                HostRow(vec![
                    Cell::File(file.clone()),
                    Cell::Text(HostText(id.as_ref().into())),
                ])
            })
        })
        .collect();
    let external_rows = context
        .external_prefixes
        .iter()
        .map(|prefix| HostRow(vec![Cell::Text(HostText(prefix.0.as_str().into()))]))
        .collect();
    Ok(CandidateConfiguration {
        selected_pairs: crate::domain::rule_context::service::selected_pairs(
            prepared,
            selected,
            &request.files,
        )?,
        hosted: vec![
            table(
                HostRelationName(prepared.tables.selected.0.as_str().into()),
                vec![column(
                    HostColumnName(constants::CODE.into()),
                    HostKind::Text,
                )],
                selected_rows,
            ),
            table(
                HostRelationName(prepared.tables.disabled.0.as_str().into()),
                vec![
                    column(
                        HostColumnName(constants::FILE_COLUMN.into()),
                        HostKind::File,
                    ),
                    column(HostColumnName(constants::CODE.into()), HostKind::Text),
                ],
                disabled_rows,
            ),
            table(
                HostRelationName(prepared.tables.external.0.as_str().into()),
                vec![column(
                    HostColumnName(constants::PREFIX.into()),
                    HostKind::Text,
                )],
                external_rows,
            ),
        ],
    })
}

/// Resolve diagnostic selection separately from explicit producer dependencies.
/// The SDK remains responsible for output/schema validation and cycle rejection.
pub(crate) fn retain_producers(
    declared: &[RuleId],
    frontend: Option<&RuleId>,
    rules: &mut BTreeMap<RuleId, Rule>,
) -> Result<BTreeSet<RuleId>, Error> {
    let selected: BTreeSet<_> = declared.iter().cloned().collect();
    let mut retained = selected.clone();
    if let Some(frontend) = frontend {
        retained.insert(frontend.clone());
    }
    retained = crate::domain::observations::service::closure(retained, |id| {
        rules.get(id).map(|rule| rule.observations.as_slice())
    });
    for id in &retained {
        if !rules.contains_key(id) {
            return Err(Error::Invalid(format!(
                "selected or required producer has no registered query: {id}"
            )));
        }
    }
    rules.retain(|id, _| retained.contains(id));
    let excluded =
        Glob::try_from(constants::TEXT).map_err(|error| Error::Invalid(error.to_string()))?;
    for (id, rule) in rules {
        if !selected.contains(id) && frontend != Some(id) {
            rule.exclude.push(excluded.clone());
        }
    }
    Ok(selected)
}

/// Restrict automatic syntax projection to literal declared missing paths.
pub(crate) fn exclude_missing(
    rules: &mut BTreeMap<RuleId, Rule>,
    frontend: &RuleId,
    missing: &BTreeSet<ryo::linter::RelativePath>,
) -> Result<(), Error> {
    let rule = rules
        .get_mut(frontend)
        .ok_or_else(|| Error::Invalid(constants::FRONTEND_PROJECTION_WAS_NOT_REGISTERED.into()))?;
    rule.exclude = frontend_exclusions(missing)?;
    Ok(())
}

/// Provider-reported configuration files do not receive source-syntax diagnostics.
pub(crate) fn exclude_configuration(
    rules: &mut BTreeMap<RuleId, Rule>,
    frontend: &RuleId,
    files: &[ryo::linter::RelativePath],
) -> Result<(), Error> {
    let rule = rules
        .get_mut(frontend)
        .ok_or_else(|| Error::Invalid(constants::FRONTEND_PROJECTION_WAS_NOT_REGISTERED.into()))?;
    for file in files {
        #[cfg(unix)]
        let literal = file.0.replace('\\', constants::TEXT_2);
        #[cfg(not(unix))]
        let literal = file.0.to_string();
        rule.exclude.push(
            Glob::try_from(globset::escape(&literal).as_str())
                .map_err(|error| Error::Invalid(error.to_string()))?,
        );
    }
    Ok(())
}

/// Escape exactly one literal path rather than admitting a caller glob.
pub(super) fn frontend_exclusions(
    missing: &BTreeSet<ryo::linter::RelativePath>,
) -> Result<Vec<Glob>, Error> {
    missing
        .iter()
        .map(|file| {
            #[cfg(unix)]
            let literal = file.0.replace('\\', constants::TEXT_2);
            #[cfg(not(unix))]
            let literal = file.0.to_string();
            Glob::try_from(globset::escape(&literal).as_str())
                .map_err(|error| Error::Invalid(error.to_string()))
        })
        .collect()
}
