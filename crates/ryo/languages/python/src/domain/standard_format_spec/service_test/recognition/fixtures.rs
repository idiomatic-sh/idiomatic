//! The support of the group's tests.
use crate::domain::standard_format_spec::types::StandardFormatTarget;
use crate::domain::standard_library::types::{PythonMajor, PythonMinor, TargetVersion};

/// Python 3 at the given minor release, as the recognizer is asked about it.
pub(super) fn target(minor: i64) -> StandardFormatTarget {
    TargetVersion {
        major: PythonMajor(3),
        minor: PythonMinor(minor),
    }
    .into()
}
