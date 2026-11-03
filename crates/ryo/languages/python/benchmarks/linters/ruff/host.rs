//! Pinned reference applicability belongs to its process edge, never Python facts.
#[cfg(unix)]
pub(crate) fn permission_checks_supported() -> bool {
    if std::env::consts::OS != "linux" {
        return true;
    }
    let kernel = std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .or_else(|_| std::fs::read_to_string("/proc/version"));
    let microsoft = kernel.is_ok_and(|value| value.to_ascii_lowercase().contains("microsoft"));
    let container = std::fs::metadata("/.dockerenv").is_ok()
        || std::fs::metadata("/run/.containerenv").is_ok()
        || std::fs::read_to_string("/proc/self/cgroup").is_ok_and(|value| {
            let value = value.to_ascii_lowercase();
            ["docker", "/container", "/lxc", "kubepods"]
                .iter()
                .any(|marker| value.contains(marker))
        });
    supported(microsoft, container)
}
#[cfg(not(unix))]
pub(crate) fn permission_checks_supported() -> bool {
    false
}

#[cfg(unix)]
fn supported(microsoft: bool, container: bool) -> bool {
    !microsoft || container
}

#[cfg(all(test, unix))]
#[test]
fn wsl_reference_policy_does_not_misclassify_containers_or_rewrite_facts() {
    assert!(supported(false, false));
    assert!(!supported(true, false));
    assert!(supported(true, true));
}
