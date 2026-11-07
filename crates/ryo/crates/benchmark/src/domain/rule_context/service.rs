//! Declared linter configuration data; matching remains with the tool/query and ordinary SDK scope rules.
use super::constants;
use super::types::LiteralGlob;
use ryo::linter::{RelativePath, RuleId};
use ryo_benchmark_contracts::domain::context::types::PreparedRuleContext;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::PairCount;
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, ConfigurationOverride, RuleContextPreparation,
};
use ryo_benchmark_contracts::domain::workloads::types::{
    RuleConfigurationSupport, RuleContextProfile,
};

/// Resolve an explicit override against the profile declaration without guessing table names or defaults.
///
/// # Errors
/// Refuses an override the profile does not declare configuration for, and unnamed or repeated table names.
pub fn resolve(
    profile: &RuleContextProfile,
    requested: &ConfigurationOverride,
) -> Result<RuleContextPreparation, Error> {
    match (profile, requested) {
        (RuleContextProfile::Undeclared, ConfigurationOverride::Inherited) => {
            Ok(RuleContextPreparation::Unconfigured)
        }
        (RuleContextProfile::Undeclared, ConfigurationOverride::Overridden(_)) => Err(
            Error::Invalid(constants::CASE_RULE_CONFIGURATION_REQUIRES_THE_PROFILE.into()),
        ),
        (RuleContextProfile::Declared(profile), requested) => {
            let names = [
                &profile.tables.selected,
                &profile.tables.disabled,
                &profile.tables.external,
            ];
            if names.iter().any(|name| name.0.trim().is_empty())
                || names
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != names.len()
            {
                return Err(Error::Invalid(
                    constants::RULE_CONFIGURATION_TABLES_MUST_BE_DISTINCT.into(),
                ));
            }
            Ok(RuleContextPreparation::Prepared(PreparedRuleContext {
                tables: profile.tables.clone(),
                configuration: match requested {
                    ConfigurationOverride::Inherited => profile.defaults.clone(),
                    ConfigurationOverride::Overridden(configuration) => configuration.clone(),
                },
            }))
        }
    }
}

/// Validate declarative addresses against the actual request, without doing path-pattern matching.
pub fn check(prepared: &PreparedRuleContext, files: &[RelativePath]) -> Result<(), Error> {
    let names = [
        &prepared.tables.selected,
        &prepared.tables.disabled,
        &prepared.tables.external,
    ];
    if names.iter().any(|name| name.0.trim().is_empty())
        || names
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != names.len()
    {
        return Err(Error::Invalid(
            constants::RULE_CONFIGURATION_TABLES_MUST_BE_DISTINCT.into(),
        ));
    }
    for (file, rules) in &prepared.configuration.disabled_rules {
        ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(std::path::Path::new(std::path::Path::new(file.0.as_ref())))?;
        if !files.contains(file) {
            return Err(Error::Invalid(format!(
                "rule configuration names an unrequested file: {}",
                file.0
            )));
        }
        if rules
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != rules.len()
        {
            return Err(Error::Invalid(
                constants::DISABLED_RULE_LISTS_CONTAIN_EACH_RULE.into(),
            ));
        }
    }
    Ok(())
}

/// Disabled pairs refer to the same explicit selected cohort sent to both contestants.
///
/// # Errors
/// Refuses a frozen cohort and a disabled rule outside the explicit cohort.
pub fn check_selection(
    prepared: &PreparedRuleContext,
    selected: &CohortSelection,
) -> Result<(), Error> {
    let CohortSelection::Explicit(selected) = selected else {
        return Err(Error::Invalid(
            constants::RULE_CONTEXT_REQUIRES_AN_EXPLICIT_SELECTED.into(),
        ));
    };
    if prepared
        .configuration
        .disabled_rules
        .values()
        .flatten()
        .any(|rule| !selected.contains(rule))
    {
        return Err(Error::Invalid(
            constants::DISABLED_RULES_MUST_BELONG_TO_THE.into(),
        ));
    }
    Ok(())
}

/// Encode an exact path as a literal glob for existing tool/SDK scope interfaces; never match it here.
#[must_use]
pub fn literal_glob(path: &RelativePath) -> LiteralGlob {
    LiteralGlob(globset::escape(path.0.as_ref()))
}

/// Number of applicable selected pairs in an explicit exact-file configuration.
/// This is data accounting; actual rule governance stays in the SDK's existing matcher.
///
/// # Errors
/// Refuses an invalid declaration, an overflowing count, or more disabled pairs than selected pairs.
pub fn selected_pairs(
    prepared: &PreparedRuleContext,
    selected: &[RuleId],
    files: &[RelativePath],
) -> Result<PairCount, Error> {
    check(prepared, files)?;
    check_selection(prepared, &CohortSelection::Explicit(selected.to_vec()))?;
    let all = selected
        .len()
        .checked_mul(files.len())
        .ok_or_else(|| Error::Invalid(constants::CONFIGURED_PAIR_COUNT_OVERFLOWS.into()))?;
    let disabled =
        prepared
            .configuration
            .disabled_rules
            .values()
            .try_fold(0_usize, |count, rules| {
                count.checked_add(rules.len()).ok_or_else(|| {
                    Error::Invalid(constants::CONFIGURED_PAIR_COUNT_OVERFLOWS.into())
                })
            })?;
    all.checked_sub(disabled)
        .map(PairCount)
        .ok_or_else(|| Error::Invalid(constants::DISABLED_PAIRS_EXCEED_THE_SELECTED_COHORT.into()))
}

/// Admit a request's declared rule configuration for one contestant: the declaration checks hold and the contestant implements the explicit contract.
///
/// # Errors
/// Refuses an invalid declaration, or a contestant that does not implement the declared configuration contract.
pub fn admit_configuration(
    request: &ryo_benchmark_contracts::domain::requests::types::RunRequest,
    support: RuleConfigurationSupport,
) -> Result<(), Error> {
    let RuleContextPreparation::Prepared(context) = &request.rule_context else {
        return Ok(());
    };
    check(context, &request.files)?;
    check_selection(context, &request.selected_rules)?;
    if support != RuleConfigurationSupport::Explicit {
        return Err(Error::Invalid(
            constants::CONTESTANT_DOES_NOT_IMPLEMENT_THE_DECLARED.into(),
        ));
    }
    Ok(())
}
