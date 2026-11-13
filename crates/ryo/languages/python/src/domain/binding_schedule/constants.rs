//! The names of the `binding_schedule` concept.
use crate::domain::standard_library::types::{PythonMajor, PythonMinor, TargetVersion};

/// The first target release whose annotations are evaluated lazily.
pub(crate) const LAZY_ANNOTATIONS_FROM: TargetVersion = TargetVersion {
    major: PythonMajor(3),
    minor: PythonMinor(14),
};
