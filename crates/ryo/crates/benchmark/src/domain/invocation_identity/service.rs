//! Normalize explicitly supplied invocation paths while preserving flags and environment overrides.
use super::constants;
use super::errors::InvocationIdentityError;
use super::types::{Boundaries, InvocationText, Variable};
use benchmark_contracts::domain::measurements::constants::{JOBS_PIN, THREAD_PIN, THREADS_FLAG};
use benchmark_contracts::domain::measurements::types::Invocation;
use benchmark_contracts::domain::workloads::types::PreparedCase;
use std::ffi::OsStr;
use std::path::PathBuf;

/// One spelling with the state boundary, the request root and path separators replaced.
pub(crate) fn normalize_text(value: &InvocationText, boundaries: &Boundaries) -> InvocationText {
    InvocationText(
        value
            .0
            .replace(&boundaries.state.0, constants::STATE)
            .replace(&boundaries.root.0, constants::ROOT)
            .replace(constants::BACKSLASH, constants::NORMALIZED_SEPARATOR),
    )
}

/// One environment variable as an identity spells it: name and value normalized, and a worker count (the thread pool's or
/// cargo's jobs) spelled as one worker, for the workers a run takes are a measurement setting that never enters the identity.
pub(crate) fn normalize_variable(variable: &Variable, boundaries: &Boundaries) -> Variable {
    Variable {
        name: normalize_text(&variable.name, boundaries),
        value: if [THREAD_PIN, JOBS_PIN].contains(&variable.name.0.as_str()) {
            InvocationText(constants::IDENTITY_WORKERS.to_owned())
        } else {
            normalize_text(&variable.value, boundaries)
        },
    }
}

/// The arguments as an identity spells them: each normalized, and the value after a thread-count flag spelled as one worker,
/// as `normalize_variable` spells a worker count.
pub(crate) fn normalize_arguments(
    arguments: &[InvocationText],
    boundaries: &Boundaries,
) -> Vec<InvocationText> {
    let mut previous: Option<&InvocationText> = None;
    arguments
        .iter()
        .map(|argument| {
            let spelled = if previous.is_some_and(|flag| flag.0 == THREADS_FLAG) {
                InvocationText(constants::IDENTITY_WORKERS.to_owned())
            } else {
                normalize_text(argument, boundaries)
            };
            previous = Some(argument);
            spelled
        })
        .collect()
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
        arguments: normalize_arguments(
            &invocation
                .arguments
                .iter()
                .map(|value| {
                    value
                        .to_str()
                        .map(|text| InvocationText(text.to_owned()))
                        .ok_or(InvocationIdentityError::NonUtf8)
                })
                .collect::<Result<Vec<_>, _>>()?,
            &boundaries,
        )
        .into_iter()
        .map(|spelling| spelling.0.into())
        .collect(),
        environment: invocation
            .environment
            .iter()
            .map(|(key, value)| {
                let text = |value: &OsStr| {
                    value
                        .to_str()
                        .map(|text| InvocationText(text.to_owned()))
                        .ok_or(InvocationIdentityError::NonUtf8)
                };
                let variable = Variable {
                    name: text(key)?,
                    value: text(value)?,
                };
                let spelled = normalize_variable(&variable, &boundaries);
                Ok((spelled.name.0.into(), spelled.value.0.into()))
            })
            .collect::<Result<_, InvocationIdentityError>>()?,
        directory: PathBuf::from(spell(invocation.directory.as_os_str())?.0),
    })
}
