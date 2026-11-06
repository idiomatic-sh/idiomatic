//! One explicit shared parser target at the benchmark composition boundary.
use super::constants::{
    EXPLICIT_LEGACY_TARGET, EXPLICIT_NATIVE_ANNOTATION_TARGET, PY37, PY38, PY39, PY310, PY311,
    PY312, PY313, PY314, PY315, PYTHON_TARGET_3_6, PYTHON_TARGET_3_7, PYTHON_TARGET_3_8,
    PYTHON_TARGET_3_09, PYTHON_TARGET_3_9, PYTHON_TARGET_3_10, PYTHON_TARGET_3_10_2,
    PYTHON_TARGET_3_11, PYTHON_TARGET_3_12, PYTHON_TARGET_3_13, PYTHON_TARGET_3_14,
    PYTHON_TARGET_3_15, PYTHON_TARGET_3_16, PYTHON_TARGET_LATEST,
    UNSUPPORTED_OR_MISSING_RUFF_PARSER,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_python::settings::PythonTargetVersion;

/// The reference CLI spelling and linked parser mode for one canonical identity.
#[derive(Clone, Copy, Debug)]
pub(super) struct Target {
    /// The same version passed to the linked Python language.
    pub(super) parser: PythonTargetVersion,
    /// The pinned Ruff CLI spelling, without enabling preview or changing rules.
    pub(super) reference: &'static str,
}

/// Resolve an explicit supported version; moving latest and implicit defaults are refused.
pub(super) fn resolve(configuration: Option<&str>) -> Result<Target, Error> {
    let (parser, reference) = match configuration {
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
    Ok(Target { parser, reference })
}

#[cfg(test)]
mod tests {
    use super::resolve;
    use ryo_python::settings::PythonTargetVersion;

    #[test]
    fn legacy_and_native_annotation_boundaries_select_the_requested_parser() {
        let legacy = resolve(Some(PYTHON_TARGET_3_7)).expect(EXPLICIT_LEGACY_TARGET);
        assert_eq!(legacy.parser, PythonTargetVersion::Py37);
        assert_eq!(legacy.reference, "py37");
        let native = resolve(Some(PYTHON_TARGET_3_14)).expect(EXPLICIT_NATIVE_ANNOTATION_TARGET);
        assert_eq!(native.parser, PythonTargetVersion::Py314);
        assert_eq!(native.reference, "py314");
    }

    #[test]
    fn missing_moving_unknown_and_noncanonical_targets_are_refused() {
        for value in [
            None,
            Some(PYTHON_TARGET_LATEST),
            Some(PYTHON_TARGET_3_6),
            Some(PYTHON_TARGET_3_16),
            Some(PYTHON_TARGET_3_09),
            Some(PYTHON_TARGET_3_10_2),
        ] {
            assert!(resolve(value).is_err(), "{value:?}");
        }
    }
}
