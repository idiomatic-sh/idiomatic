//! Typed inputs and outcomes for release-specific module membership.

/// Selected Python release, independent of the host interpreter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TargetVersion {
    pub major: PythonMajor,
    pub minor: PythonMinor,
}

/// Python's major release number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PythonMajor(pub i64);

/// Python's minor release number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PythonMinor(pub i64);

/// Absolute imported path; the first component names the module to classify.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ModulePath<'a>(pub &'a str);

/// Definite catalog membership for a supported target release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StandardLibraryMembership {
    Standard,
    Other,
    /// CPython did not publish this catalog for the selected release.
    UnverifiedRelease,
}
