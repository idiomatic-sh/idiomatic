//! Immutable settings that affect how the Python parser reads a source file.

mod constants;

use crate::domain::standard_library::types::{PythonMajor, PythonMinor, TargetVersion};
use constants::{
    IDENTITY_LATEST, IDENTITY_PY37, IDENTITY_PY38, IDENTITY_PY39, IDENTITY_PY310, IDENTITY_PY311,
    IDENTITY_PY312, IDENTITY_PY313, IDENTITY_PY314, IDENTITY_PY315,
};
use ryo::languages::ParseConfigurationIdentity;

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
    pub(crate) const fn components(self) -> TargetVersion {
        match self {
            Self::Py37 => TargetVersion {
                major: PythonMajor(3),
                minor: PythonMinor(7),
            },
            Self::Py38 => TargetVersion {
                major: PythonMajor(3),
                minor: PythonMinor(8),
            },
            Self::Py39 => TargetVersion {
                major: PythonMajor(3),
                minor: PythonMinor(9),
            },
            Self::Py310 => TargetVersion {
                major: PythonMajor(3),
                minor: PythonMinor(10),
            },
            Self::Py311 => TargetVersion {
                major: PythonMajor(3),
                minor: PythonMinor(11),
            },
            Self::Py312 => TargetVersion {
                major: PythonMajor(3),
                minor: PythonMinor(12),
            },
            Self::Py313 => TargetVersion {
                major: PythonMajor(3),
                minor: PythonMinor(13),
            },
            Self::Py314 => TargetVersion {
                major: PythonMajor(3),
                minor: PythonMinor(14),
            },
            Self::Py315 | Self::Latest => TargetVersion {
                major: PythonMajor(3),
                minor: PythonMinor(15),
            },
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
    pub(crate) const fn identity(self) -> ParseConfigurationIdentity<'static> {
        match self.target_version {
            PythonTargetVersion::Py37 => IDENTITY_PY37,
            PythonTargetVersion::Py38 => IDENTITY_PY38,
            PythonTargetVersion::Py39 => IDENTITY_PY39,
            PythonTargetVersion::Py310 => IDENTITY_PY310,
            PythonTargetVersion::Py311 => IDENTITY_PY311,
            PythonTargetVersion::Py312 => IDENTITY_PY312,
            PythonTargetVersion::Py313 => IDENTITY_PY313,
            PythonTargetVersion::Py314 => IDENTITY_PY314,
            PythonTargetVersion::Py315 => IDENTITY_PY315,
            PythonTargetVersion::Latest => IDENTITY_LATEST,
        }
    }
}
