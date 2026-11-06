//! Shared reference invocation identity; captures and offline replay use the same path encoding.
use super::discrepancy::json_digest;
use crate::application::run::constants;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::PreparedCase;

/// Normalize explicitly supplied invocation paths while preserving flags and environment overrides.
pub(super) fn invocation_settings(
    invocation: &ryo_benchmark_contracts::domain::measurements::types::Invocation,
    case: &PreparedCase,
    profile: &ryo_benchmark_contracts::domain::workloads::types::Tool,
) -> Result<ArtifactDigest, Error> {
    Ok(json_digest(&normalized_invocation(
        invocation, case, profile,
    )?))
}

pub(super) fn normalized_invocation(
    invocation: &ryo_benchmark_contracts::domain::measurements::types::Invocation,
    case: &PreparedCase,
    profile: &ryo_benchmark_contracts::domain::workloads::types::Tool,
) -> Result<serde_json::Value, Error> {
    let normalize = |value: &std::ffi::OsStr| -> Result<String, Error> {
        let value = value
            .to_str()
            .ok_or_else(|| Error::Invalid(constants::NON_UTF8_REFERENCE_INVOCATION.into()))?;
        Ok(value
            .replace(
                &case.request.state.to_string_lossy().to_string(),
                constants::STATE_2,
            )
            .replace(
                &case.request.root.to_string_lossy().to_string(),
                constants::ROOT,
            )
            .replace('\\', constants::SLASH))
    };
    let arguments = invocation
        .arguments
        .iter()
        .map(|value| normalize(value))
        .collect::<Result<Vec<_>, _>>()?;
    let environment = invocation
        .environment
        .iter()
        .map(|(key, value)| Ok((normalize(key)?, normalize(value)?)))
        .collect::<Result<std::collections::BTreeMap<_, _>, Error>>()?;
    let mut settings = serde_json::json!({
        constants::ARGUMENTS: arguments, constants::ENVIRONMENT: environment,
        constants::DIRECTORY: normalize(invocation.directory.as_os_str())?,
        constants::PARSER_CONFIGURATION: profile.parser_configuration,
        constants::PROJECT_CONTEXT_MODE: format!("{:?}", profile.project_context_mode),
    });
    settings[constants::PERFORMANCE] = serde_json::to_value(
        crate::outbound::worker::schemas::OptionsSchema::from(case.request.performance),
    )
    .map_err(|error| Error::Invalid(error.to_string()))?;
    if let Some(declaration) = &profile.rule_context {
        settings[constants::RULE_CONTEXT_DECLARATION] =
            crate::outbound::worker::schemas::declaration_value(declaration);
    }
    if let Some(context) = &case.request.rule_context {
        settings[constants::RULE_CONTEXT] =
            crate::outbound::worker::schemas::prepared_value(context);
    }
    if !profile.rule_observations.is_empty() {
        let declarations = crate::domain::suite::observations::project_profile(
            &profile.rule_observations,
            case.request
                .selected_rules
                .as_deref()
                .unwrap_or(&profile.defaults),
            profile.rule_observation_selection,
        );
        if !declarations.is_empty() {
            settings[constants::RULE_OBSERVATIONS] =
                crate::outbound::worker::schemas::observation_declarations_value(&declarations);
        }
    }
    if let Some(declarations) = &case.request.rule_observations {
        settings[constants::RULE_OBSERVATIONS] =
            crate::outbound::worker::schemas::observation_declarations_value(declarations);
    }
    if let Some(selected) = &case.request.selected_rules {
        let selected: std::collections::BTreeSet<_> =
            selected.iter().map(ToString::to_string).collect();
        settings[constants::SELECTED_RULES] = serde_json::json!(selected);
    }
    if let Some(language) = &profile.source_language {
        settings[constants::SOURCE_LANGUAGE] = serde_json::json!(language);
    }
    if let Some(roots) = &case.request.project_search_roots {
        settings[constants::PROJECT_SEARCH_ROOTS] =
            serde_json::json!(roots.iter().map(|root| root.0.as_ref()).collect::<Vec<_>>());
    }
    let inventory =
        crate::outbound::context::adapters::local::schemas::ProjectInventorySchema::from(
            case.request.given_project_inventory.clone(),
        );
    if !inventory.is_default() {
        settings[constants::GIVEN_PROJECT_INVENTORY] =
            serde_json::to_value(inventory).map_err(|error| Error::Json(Box::new(error)))?;
    }
    Ok(settings)
}

/// Reconstruct argv/environment/cwd from raw captured metadata, without trusting precomputed digests.
pub(super) fn reconstruct(invocation: &serde_json::Value) -> Result<serde_json::Value, Error> {
    use serde_json::Value;
    let mut settings = invocation[constants::NORMALIZED_SETTINGS].clone();
    if !settings.is_object() {
        return Err(Error::Invalid(
            constants::MISSING_NORMALIZED_INVOCATION_CONFIGURATION.into(),
        ));
    }
    let root = invocation[constants::ACTUAL_REQUEST_ROOT]
        .as_str()
        .filter(|root| !root.is_empty())
        .ok_or_else(|| Error::Invalid(constants::RAW_INVOCATION_ROOT_IS_MISSING.into()))?;
    if !std::path::Path::new(root).is_absolute() {
        return Err(Error::Invalid(
            constants::RAW_INVOCATION_ROOT_IS_NOT_ABSOLUTE.into(),
        ));
    }
    let state = invocation[constants::STATE]
        .as_str()
        .filter(|state| !state.is_empty())
        .ok_or_else(|| Error::Invalid(constants::RAW_INVOCATION_STATE_IS_MISSING.into()))?;
    let normalize = |value: &Value| -> Result<String, Error> {
        Ok(value
            .as_str()
            .ok_or_else(|| Error::Invalid(constants::NONUTF8_INVOCATION_VALUE.into()))?
            .replace(state, constants::STATE_2)
            .replace(root, constants::ROOT)
            .replace('\\', constants::SLASH))
    };
    settings[constants::ARGUMENTS] = Value::Array(
        invocation[constants::ARGUMENTS]
            .as_array()
            .ok_or_else(|| Error::Invalid(constants::RAW_ARGV_IS_MISSING.into()))?
            .iter()
            .map(|arg| normalize(arg).map(Value::String))
            .collect::<Result<_, _>>()?,
    );
    settings[constants::ENVIRONMENT] = Value::Object(
        invocation[constants::ENVIRONMENT]
            .as_object()
            .ok_or_else(|| Error::Invalid(constants::RAW_ENVIRONMENT_IS_MISSING.into()))?
            .iter()
            .map(|(key, value)| {
                Ok((
                    normalize(&Value::String(key.clone()))?,
                    Value::String(normalize(value)?),
                ))
            })
            .collect::<Result<_, Error>>()?,
    );
    settings[constants::DIRECTORY] = Value::String(normalize(&invocation[constants::DIRECTORY])?);
    Ok(settings)
}
