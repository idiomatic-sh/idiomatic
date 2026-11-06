//! Declared linter configuration data; matching remains with the tool/query and ordinary SDK scope rules.
use ryo_benchmark_contracts::domain::context::types::PreparedRuleContext;
use ryo_benchmark_contracts::domain::context::types::RuleConfiguration;
use ryo_benchmark_contracts::domain::context::types::RuleContextDeclaration;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;

/// Resolve an explicit override against the profile declaration without guessing table names or defaults.
pub fn resolve(
    profile: Option<&RuleContextDeclaration>,
    requested: Option<&RuleConfiguration>,
) -> Result<Option<PreparedRuleContext>, Error> {
    match (profile, requested) {
        (None, None) => Ok(None),
        (None, Some(_)) => Err(Error::Invalid(
            crate::domain::suite::constants::CASE_RULE_CONFIGURATION_REQUIRES_THE_PROFILE.into(),
        )),
        (Some(profile), requested) => {
            let names = [
                &profile.tables.selected,
                &profile.tables.disabled,
                &profile.tables.external,
            ];
            if names.iter().any(|name| name.trim().is_empty())
                || names
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != names.len()
            {
                return Err(Error::Invalid(
                    crate::domain::suite::constants::RULE_CONFIGURATION_TABLES_MUST_BE_DISTINCT
                        .into(),
                ));
            }
            Ok(Some(PreparedRuleContext {
                tables: profile.tables.clone(),
                configuration: requested.unwrap_or(&profile.defaults).clone(),
            }))
        }
    }
}

/// Validate declarative addresses against the actual request, without doing path-pattern matching.
pub fn check(
    prepared: &PreparedRuleContext,
    files: &[ryo::linter::RelativePath],
) -> Result<(), Error> {
    let names = [
        &prepared.tables.selected,
        &prepared.tables.disabled,
        &prepared.tables.external,
    ];
    if names.iter().any(|name| name.trim().is_empty())
        || names
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != names.len()
    {
        return Err(Error::Invalid(
            crate::domain::suite::constants::RULE_CONFIGURATION_TABLES_MUST_BE_DISTINCT.into(),
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
                crate::domain::suite::constants::DISABLED_RULE_LISTS_CONTAIN_EACH_RULE.into(),
            ));
        }
    }
    Ok(())
}

/// Disabled pairs refer to the same explicit selected cohort sent to both contestants.
pub fn check_selection(
    prepared: &PreparedRuleContext,
    selected: Option<&[ryo::linter::RuleId]>,
) -> Result<(), Error> {
    let selected = selected.ok_or_else(|| {
        Error::Invalid(
            crate::domain::suite::constants::RULE_CONTEXT_REQUIRES_AN_EXPLICIT_SELECTED.into(),
        )
    })?;
    if prepared
        .configuration
        .disabled_rules
        .values()
        .flatten()
        .any(|rule| !selected.contains(rule))
    {
        return Err(Error::Invalid(
            crate::domain::suite::constants::DISABLED_RULES_MUST_BELONG_TO_THE.into(),
        ));
    }
    Ok(())
}

/// Encode an exact path as a literal glob for existing tool/SDK scope interfaces; never match it here.
#[must_use]
pub fn literal_glob(text: &str) -> String {
    globset::escape(text)
}

/// Number of applicable selected pairs in an explicit exact-file configuration.
/// This is data accounting; actual rule governance stays in the SDK's existing matcher.
pub fn selected_pairs(
    prepared: &PreparedRuleContext,
    selected: &[ryo::linter::RuleId],
    files: &[ryo::linter::RelativePath],
) -> Result<usize, Error> {
    check(prepared, files)?;
    check_selection(prepared, Some(selected))?;
    let all = selected.len().checked_mul(files.len()).ok_or_else(|| {
        Error::Invalid(crate::domain::suite::constants::CONFIGURED_PAIR_COUNT_OVERFLOWS.into())
    })?;
    let disabled =
        prepared
            .configuration
            .disabled_rules
            .values()
            .try_fold(0_usize, |count, rules| {
                count.checked_add(rules.len()).ok_or_else(|| {
                    Error::Invalid(
                        crate::domain::suite::constants::CONFIGURED_PAIR_COUNT_OVERFLOWS.into(),
                    )
                })
            })?;
    all.checked_sub(disabled).ok_or_else(|| {
        Error::Invalid(
            crate::domain::suite::constants::DISABLED_PAIRS_EXCEED_THE_SELECTED_COHORT.into(),
        )
    })
}
