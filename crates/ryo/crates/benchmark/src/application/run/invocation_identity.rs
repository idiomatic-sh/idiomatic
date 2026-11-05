//! Shared reference invocation identity; captures and offline replay use the same path encoding.
use super::discrepancy::json_digest;
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
            .ok_or_else(|| Error::Invalid("non-UTF8 reference invocation".into()))?;
        Ok(value
            .replace(&case.request.state.to_string_lossy().to_string(), "$STATE")
            .replace(&case.request.root.to_string_lossy().to_string(), "$ROOT")
            .replace('\\', "/"))
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
        "arguments": arguments, "environment": environment,
        "directory": normalize(invocation.directory.as_os_str())?,
        "parser_configuration": profile.parser_configuration,
        "project_context_mode": format!("{:?}", profile.project_context_mode),
    });
    settings["performance"] = serde_json::to_value(
        crate::outbound::worker::schemas::OptionsSchema::from(case.request.performance),
    )
    .map_err(|error| Error::Invalid(error.to_string()))?;
    if let Some(declaration) = &profile.rule_context {
        settings["rule_context_declaration"] =
            crate::outbound::worker::schemas::declaration_value(declaration);
    }
    if let Some(context) = &case.request.rule_context {
        settings["rule_context"] = crate::outbound::worker::schemas::prepared_value(context);
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
            settings["rule_observations"] =
                crate::outbound::worker::schemas::observation_declarations_value(&declarations);
        }
    }
    if let Some(declarations) = &case.request.rule_observations {
        settings["rule_observations"] =
            crate::outbound::worker::schemas::observation_declarations_value(declarations);
    }
    if let Some(selected) = &case.request.selected_rules {
        let selected: std::collections::BTreeSet<_> =
            selected.iter().map(ToString::to_string).collect();
        settings["selected_rules"] = serde_json::json!(selected);
    }
    if let Some(language) = &profile.source_language {
        settings["source_language"] = serde_json::json!(language);
    }
    if let Some(roots) = &case.request.project_search_roots {
        settings["project_search_roots"] =
            serde_json::json!(roots.iter().map(|root| root.0.as_ref()).collect::<Vec<_>>());
    }
    let inventory =
        crate::outbound::context::adapters::local::schemas::ProjectInventorySchema::from(
            case.request.given_project_inventory.clone(),
        );
    if !inventory.is_default() {
        settings["given_project_inventory"] =
            serde_json::to_value(inventory).map_err(|error| Error::Json(Box::new(error)))?;
    }
    Ok(settings)
}

/// Reconstruct argv/environment/cwd from raw captured metadata, without trusting precomputed digests.
pub(super) fn reconstruct(invocation: &serde_json::Value) -> Result<serde_json::Value, Error> {
    use serde_json::Value;
    let mut settings = invocation["normalized_settings"].clone();
    if !settings.is_object() {
        return Err(Error::Invalid(
            "missing normalized invocation configuration".into(),
        ));
    }
    let root = invocation["actual_request_root"]
        .as_str()
        .filter(|root| !root.is_empty())
        .ok_or_else(|| Error::Invalid("raw invocation root is missing".into()))?;
    if !std::path::Path::new(root).is_absolute() {
        return Err(Error::Invalid("raw invocation root is not absolute".into()));
    }
    let state = invocation["state"]
        .as_str()
        .filter(|state| !state.is_empty())
        .ok_or_else(|| Error::Invalid("raw invocation state is missing".into()))?;
    let normalize = |value: &Value| -> Result<String, Error> {
        Ok(value
            .as_str()
            .ok_or_else(|| Error::Invalid("nonUTF8 invocation value".into()))?
            .replace(state, "$STATE")
            .replace(root, "$ROOT")
            .replace('\\', "/"))
    };
    settings["arguments"] = Value::Array(
        invocation["arguments"]
            .as_array()
            .ok_or_else(|| Error::Invalid("raw argv is missing".into()))?
            .iter()
            .map(|arg| normalize(arg).map(Value::String))
            .collect::<Result<_, _>>()?,
    );
    settings["environment"] = Value::Object(
        invocation["environment"]
            .as_object()
            .ok_or_else(|| Error::Invalid("raw environment is missing".into()))?
            .iter()
            .map(|(key, value)| {
                Ok((
                    normalize(&Value::String(key.clone()))?,
                    Value::String(normalize(value)?),
                ))
            })
            .collect::<Result<_, Error>>()?,
    );
    settings["directory"] = Value::String(normalize(&invocation["directory"])?);
    Ok(settings)
}
