//! One explicit shared parser target at the benchmark composition boundary.
use crate::constants::{
    PY37, PY38, PY39, PY310, PY311, PY312, PY313, PY314, PY315, PYTHON_TARGET_3_7,
    PYTHON_TARGET_3_8, PYTHON_TARGET_3_9, PYTHON_TARGET_3_10, PYTHON_TARGET_3_11,
    PYTHON_TARGET_3_12, PYTHON_TARGET_3_13, PYTHON_TARGET_3_14, PYTHON_TARGET_3_15,
    UNSUPPORTED_OR_MISSING_RUFF_PARSER,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_python::settings::PythonTargetVersion;

/// The explicit target version text of a request or command line.
#[derive(Clone, Copy, Debug)]
pub(super) struct TargetConfiguration<'a>(pub(super) &'a str);

/// The pinned Ruff CLI spelling of one target version.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ReferenceSpelling(pub(super) &'static str);

/// The reference CLI spelling and linked parser mode for one canonical identity.
#[derive(Clone, Copy, Debug)]
pub(super) struct Target {
    /// The same version passed to the linked Python language.
    pub(super) parser: PythonTargetVersion,
    /// The pinned Ruff CLI spelling, without enabling preview or changing rules.
    pub(super) reference: ReferenceSpelling,
}

/// Resolve an explicit supported version; moving latest and implicit defaults are refused.
pub(super) fn resolve(configuration: Option<TargetConfiguration<'_>>) -> Result<Target, Error> {
    let (parser, reference) = match configuration.map(|configuration| configuration.0) {
        Some(PYTHON_TARGET_3_7) => (PythonTargetVersion::Py37, PY37),
        Some(PYTHON_TARGET_3_8) => (PythonTargetVersion::Py38, PY38),
        Some(PYTHON_TARGET_3_9) => (PythonTargetVersion::Py39, PY39),
        Some(PYTHON_TARGET_3_10) => (PythonTargetVersion::Py310, PY310),
        Some(PYTHON_TARGET_3_11) => (PythonTargetVersion::Py311, PY311),
        Some(PYTHON_TARGET_3_12) => (PythonTargetVersion::Py312, PY312),
        Some(PYTHON_TARGET_3_13) => (PythonTargetVersion::Py313, PY313),
        Some(PYTHON_TARGET_3_14) => (PythonTargetVersion::Py314, PY314),
        Some(PYTHON_TARGET_3_15) => (PythonTargetVersion::Py315, PY315),
        None | Some(_) => {
            return Err(Error::Invalid(UNSUPPORTED_OR_MISSING_RUFF_PARSER.into()));
        }
    };
    Ok(Target {
        parser,
        reference: ReferenceSpelling(reference),
    })
}
