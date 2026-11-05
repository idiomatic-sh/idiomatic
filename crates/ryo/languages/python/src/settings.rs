//! Immutable settings that affect how the Python parser reads a source file.

/// A Python version Ruff may use to decide whether syntax is supported.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PythonTargetVersion {
    /// Python 3.7.
    Py37,
    /// Python 3.8.
    Py38,
    /// Python 3.9.
    Py39,
    /// Python 3.10.
    Py310,
    /// Python 3.11.
    Py311,
    /// Python 3.12.
    Py312,
    /// Python 3.13.
    Py313,
    /// Python 3.14.
    Py314,
    /// Python 3.15.
    Py315,
    /// The newest version supported by the linked parser library.
    #[default]
    Latest,
}

impl PythonTargetVersion {
    /// Numeric language version used by generic target-aware queries.
    #[must_use]
    pub(crate) const fn components(self) -> (i64, i64) {
        match self {
            Self::Py37 => (3, 7),
            Self::Py38 => (3, 8),
            Self::Py39 => (3, 9),
            Self::Py310 => (3, 10),
            Self::Py311 => (3, 11),
            Self::Py312 => (3, 12),
            Self::Py313 => (3, 13),
            Self::Py314 => (3, 14),
            Self::Py315 | Self::Latest => (3, 15),
        }
    }
}

/// Settings that determine version-related parser diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PythonParserSettings {
    /// Version used by the parser's syntax feature gates.
    target_version: PythonTargetVersion,
}

impl PythonParserSettings {
    /// Parse using `target_version`; `Latest` selects Ruff's latest supported version.
    #[must_use]
    pub const fn new(target_version: PythonTargetVersion) -> Self {
        Self { target_version }
    }

    /// The configured language version.
    #[must_use]
    pub const fn target_version(self) -> PythonTargetVersion {
        self.target_version
    }

    /// Canonical identity of the effective parser setting.
    #[must_use]
    pub(crate) const fn identity(self) -> &'static str {
        match self.target_version {
            PythonTargetVersion::Py37 => "python-target=3.7",
            PythonTargetVersion::Py38 => "python-target=3.8",
            PythonTargetVersion::Py39 => "python-target=3.9",
            PythonTargetVersion::Py310 => "python-target=3.10",
            PythonTargetVersion::Py311 => "python-target=3.11",
            PythonTargetVersion::Py312 => "python-target=3.12",
            PythonTargetVersion::Py313 => "python-target=3.13",
            PythonTargetVersion::Py314 => "python-target=3.14",
            PythonTargetVersion::Py315 => "python-target=3.15",
            PythonTargetVersion::Latest => "python-target=latest",
        }
    }
}
