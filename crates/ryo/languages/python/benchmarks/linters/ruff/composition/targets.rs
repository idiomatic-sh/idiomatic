//! One explicit shared parser target at the benchmark composition boundary.
use ryo_benchmark::domain::diagnostics::errors::DiagnosticsError as Error;
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
        Some("python-target=3.7") => (PythonTargetVersion::Py37, "py37"),
        Some("python-target=3.8") => (PythonTargetVersion::Py38, "py38"),
        Some("python-target=3.9") => (PythonTargetVersion::Py39, "py39"),
        Some("python-target=3.10") => (PythonTargetVersion::Py310, "py310"),
        Some("python-target=3.11") => (PythonTargetVersion::Py311, "py311"),
        Some("python-target=3.12") => (PythonTargetVersion::Py312, "py312"),
        Some("python-target=3.13") => (PythonTargetVersion::Py313, "py313"),
        Some("python-target=3.14") => (PythonTargetVersion::Py314, "py314"),
        Some("python-target=3.15") => (PythonTargetVersion::Py315, "py315"),
        None | Some(_) => {
            return Err(Error::Invalid(
                "unsupported or missing Ruff parser configuration".into(),
            ));
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
        let legacy = resolve(Some("python-target=3.7")).expect("explicit legacy target");
        assert_eq!(legacy.parser, PythonTargetVersion::Py37);
        assert_eq!(legacy.reference, "py37");
        let native =
            resolve(Some("python-target=3.14")).expect("explicit native annotation target");
        assert_eq!(native.parser, PythonTargetVersion::Py314);
        assert_eq!(native.reference, "py314");
    }

    #[test]
    fn missing_moving_unknown_and_noncanonical_targets_are_refused() {
        for value in [
            None,
            Some("python-target=latest"),
            Some("python-target=3.6"),
            Some("python-target=3.16"),
            Some("python-target=3.09"),
            Some("python-target=3.10 "),
        ] {
            assert!(resolve(value).is_err(), "{value:?}");
        }
    }
}
