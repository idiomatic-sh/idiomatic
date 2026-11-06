//! Normalize explicitly supplied invocation paths while preserving flags and environment overrides.
use super::constants;
use super::errors::InvocationIdentityError;
use super::types::{Boundaries, InvocationText};
use crate::domain::suite::observations::project_profile;
use ryo_benchmark_contracts::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations;
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::workloads::types::{PreparedCase, Tool};
use std::ffi::OsStr;
use std::path::PathBuf;

/// One spelling with the state boundary, the request root and path separators replaced.
pub(crate) fn normalize_text(value: &InvocationText, boundaries: &Boundaries) -> InvocationText {
    InvocationText(
        value
            .0
            .replace(&boundaries.state.0, constants::STATE)
            .replace(&boundaries.root.0, constants::ROOT)
            .replace(constants::BACKSLASH, constants::SLASH),
    )
}

/// The state boundary and request root of a case.
pub(crate) fn boundaries(case: &PreparedCase) -> Boundaries {
    Boundaries {
        state: InvocationText(case.request.state.to_string_lossy().to_string()),
        root: InvocationText(case.request.root.to_string_lossy().to_string()),
    }
}

/// The invocation with every spelling normalized against the case's state and request root.
pub(crate) fn normalize(
    invocation: &Invocation,
    case: &PreparedCase,
) -> Result<Invocation, InvocationIdentityError> {
    let boundaries = boundaries(case);
    let spell = |value: &OsStr| -> Result<InvocationText, InvocationIdentityError> {
        let text = value.to_str().ok_or(InvocationIdentityError::NonUtf8)?;
        Ok(normalize_text(
            &InvocationText(text.to_owned()),
            &boundaries,
        ))
    };
    Ok(Invocation {
        executable: invocation.executable.clone(),
        arguments: invocation
            .arguments
            .iter()
            .map(|value| spell(value).map(|spelling| spelling.0.into()))
            .collect::<Result<_, _>>()?,
        environment: invocation
            .environment
            .iter()
            .map(|(key, value)| Ok((spell(key)?.0.into(), spell(value)?.0.into())))
            .collect::<Result<_, InvocationIdentityError>>()?,
        directory: PathBuf::from(spell(invocation.directory.as_os_str())?.0),
    })
}

/// The rule observations an invocation is bound to: the case's own, else the profile's projection.
pub(crate) fn effective_observations(
    case: &PreparedCase,
    profile: &Tool,
) -> Option<RuleObservationDeclarations> {
    if let Some(declarations) = &case.request.rule_observations {
        return Some(declarations.clone());
    }
    if profile.rule_observations.is_empty() {
        return None;
    }
    let declarations = project_profile(
        &profile.rule_observations,
        case.request
            .selected_rules
            .as_deref()
            .unwrap_or(&profile.defaults),
        profile.rule_observation_selection,
    );
    (!declarations.is_empty()).then_some(declarations)
}
