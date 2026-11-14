//! Host observation at the linter process edge: the facts the permission applicability decision is given.
/// Whether this host honours the file permissions the pinned reference observes: never off Unix, where files carry no
/// execute bit.
pub(crate) fn permission_checks_supported() -> crate::host_policy::Permissions {
    if !cfg!(unix) {
        return crate::host_policy::Permissions::Unsupported;
    }
    if std::env::consts::OS != crate::composition::constants::LINUX {
        return crate::host_policy::Permissions::Supported;
    }
    let kernel = std::fs::read_to_string(crate::composition::constants::PROC_SYS_KERNEL_OSRELEASE)
        .or_else(|_| std::fs::read_to_string(crate::composition::constants::PROC_VERSION));
    let kernel = if kernel.is_ok_and(|value| {
        value
            .to_ascii_lowercase()
            .contains(crate::composition::constants::MICROSOFT)
    }) {
        crate::host_policy::Kernel::Microsoft
    } else {
        crate::host_policy::Kernel::Native
    };
    let marked = std::fs::metadata(crate::composition::constants::DOCKERENV).is_ok()
        || std::fs::metadata(crate::composition::constants::RUN_CONTAINERENV).is_ok()
        || std::fs::read_to_string(crate::composition::constants::PROC_SELF_CGROUP).is_ok_and(
            |value| {
                let value = value.to_ascii_lowercase();
                [
                    crate::composition::constants::DOCKER,
                    crate::composition::constants::CONTAINER,
                    crate::composition::constants::LXC,
                    crate::composition::constants::KUBEPODS,
                ]
                .iter()
                .any(|marker| value.contains(marker))
            },
        );
    let isolation = if marked {
        crate::host_policy::Isolation::Container
    } else {
        crate::host_policy::Isolation::Bare
    };
    crate::host_policy::supported(kernel, isolation)
}