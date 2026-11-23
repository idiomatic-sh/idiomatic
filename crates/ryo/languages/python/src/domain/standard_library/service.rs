//! Pure membership against the selected language release, not the host interpreter.
use super::constants;
use super::types::{ModulePath, StandardLibraryMembership, TargetVersion};

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
    let (versioned, at) = match (target.major.0, target.minor.0) {
        (3, minor) => match minor {
            10 => (constants::PY3_10, 1),
            11 => (constants::PY3_11, 2),
            12 => (constants::PY3_12, 3),
            13 => (constants::PY3_13, 4),
            14 => (constants::PY3_14, 5),
            15 => (constants::PY3_15, 6),
            _ => return StandardLibraryMembership::UnverifiedRelease,
        },
        _ => return StandardLibraryMembership::UnverifiedRelease,
    };
    if member(constants::COMMON, 0, root) || member(versioned, at, root) {
        StandardLibraryMembership::Standard
    } else {
        StandardLibraryMembership::Other
    }
}

/// The module names of one `|`-separated list, sorted once a process (the lists are the catalog's constants).
static NAME_SETS: [std::sync::OnceLock<Vec<&'static str>>; 7] = [const { std::sync::OnceLock::new() }; 7];

/// Whether `root` is one of the names of `list` (the sorted names at `at` of [`NAME_SETS`], one a list).
fn member(list: &'static str, at: usize, root: &str) -> bool {
    NAME_SETS.get(at).is_some_and(|names| {
        names
            .get_or_init(|| {
                let mut sorted: Vec<&'static str> = list.split_terminator('|').collect();
                sorted.sort_unstable();
                sorted
            })
            .binary_search(&root)
            .is_ok()
    })
}
