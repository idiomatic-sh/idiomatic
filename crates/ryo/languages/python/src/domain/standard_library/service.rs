//! Pure membership against the selected language release, not the host interpreter.
use super::constants;
use super::types::{ModulePath, StandardLibraryMembership, TargetVersion};
use std::{collections::HashSet, sync::OnceLock};

/// Whether the first component of an absolute module path is in the selected
/// Python release's standard library. Platform-specific modules remain members
/// on every host, matching the language catalog rather than local availability.
#[must_use]
pub(crate) fn contains(
    target: TargetVersion,
    absolute_module: ModulePath<'_>,
) -> StandardLibraryMembership {
    let Some(root) = absolute_module.0.split('.').next() else {
        return StandardLibraryMembership::Other;
    };
    if root.is_empty() {
        return StandardLibraryMembership::Other;
    }
    let versioned = match (target.major.0, target.minor.0) {
        (3, minor) => match minor {
            10 => constants::PY3_10,
            11 => constants::PY3_11,
            12 => constants::PY3_12,
            13 => constants::PY3_13,
            14 => constants::PY3_14,
            15 => constants::PY3_15,
            _ => return StandardLibraryMembership::UnverifiedRelease,
        },
        _ => return StandardLibraryMembership::UnverifiedRelease,
    };
    // The catalogues are immutable for each supported Python release.
    static COMMON: OnceLock<HashSet<&'static str>> = OnceLock::new();
    static RELEASE: [OnceLock<HashSet<&'static str>>; 6] = [const { OnceLock::new() }; 6];
    let extra = &RELEASE[(target.minor.0 - 10) as usize];
    if COMMON
        .get_or_init(|| constants::COMMON.split_terminator('|').collect())
        .contains(root)
        || extra
            .get_or_init(|| versioned.split_terminator('|').collect())
            .contains(root)
    {
        StandardLibraryMembership::Standard
    } else {
        StandardLibraryMembership::Other
    }
}
