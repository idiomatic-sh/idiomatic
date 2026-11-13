//! A settings profile projected for one registered rule: the selected rule, its supporting rules, its parser
//! configuration, its modules and their applicability; every other key is kept and `deferred` and `pending` dropped.
use super::super::constants;
use crate::outbound::inventory::types::ProfileProjection;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::ParserSelection;

/// A list of texts.
fn texts<'a>(items: impl Iterator<Item = &'a String>) -> toml::Value {
    toml::Value::Array(items.cloned().map(toml::Value::String).collect())
}

/// The profile `source` holds, projected; `None` when the projection leaves it unchanged, so its bytes are staged.
pub(in super::super) fn projected(source: &str, projection: &ProfileProjection) -> Result<Option<String>, Error> {
    let original: toml::Table = toml::from_str(source)?;
    let mut profile = original.clone();
    for key in [constants::DEFERRED_KEY, constants::PENDING_KEY, constants::QUERY_MODULES, constants::APPLICABILITY_KEY] {
        let _dropped = profile.remove(key);
    }
    let _defaults = profile.insert(
        constants::DEFAULTS_KEY.into(),
        texts(projection.defaults.iter().map(|rule| &rule.0)),
    );
    if projection.supporting_rules.is_empty() {
        let _dropped = profile.remove(constants::SUPPORTING_KEY);
    } else {
        let _supporting = profile.insert(
            constants::SUPPORTING_KEY.into(),
            texts(projection.supporting_rules.iter().map(|rule| &rule.0)),
        );
    }
    match &projection.parser_configuration {
        ParserSelection::Named(name) => {
            let _configuration = profile.insert(constants::PARSER_CONFIGURATION.into(), toml::Value::String(name.0.clone()));
        }
        ParserSelection::Unspecified => {}
    }
    if !projection.query_modules.is_empty() {
        let _modules = profile.insert(
            constants::QUERY_MODULES.into(),
            toml::Value::Table(
                projection
                    .query_modules
                    .iter()
                    .map(|(name, path)| (name.0.clone(), toml::Value::String(path.to_string_lossy().into_owned())))
                    .collect(),
            ),
        );
    }
    if !projection.rule_applicability.is_empty() {
        let _applicability = profile.insert(
            constants::APPLICABILITY_KEY.into(),
            toml::Value::Table(
                projection
                    .rule_applicability
                    .iter()
                    .map(|(rule, applicability)| (rule.0.clone(), toml::Value::String(applicability.0.clone())))
                    .collect(),
            ),
        );
    }
    if profile == original {
        return Ok(None);
    }
    toml::to_string(&profile)
        .map(Some)
        .map_err(|error| Error::Invalid(error.to_string()))
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
