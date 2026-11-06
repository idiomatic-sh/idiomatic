//! Pinned reference applicability belongs to its process edge, never Python facts.
use super::host_policy::Permissions;
#[cfg(unix)]
use super::host_policy::{Isolation, Kernel, supported};
use crate::constants::{
    CONTAINER, DOCKER, DOCKERENV, KUBEPODS, LINUX, LXC, MICROSOFT, PROC_SELF_CGROUP,
    PROC_SYS_KERNEL_OSRELEASE, PROC_VERSION, RUN_CONTAINERENV,
};

#[cfg(unix)]
pub(crate) fn permission_checks_supported() -> Permissions {
    if std::env::consts::OS != LINUX {
        return Permissions::Supported;
    }
    let kernel = std::fs::read_to_string(PROC_SYS_KERNEL_OSRELEASE)
        .or_else(|_| std::fs::read_to_string(PROC_VERSION));
    let kernel = if kernel.is_ok_and(|value| value.to_ascii_lowercase().contains(MICROSOFT)) {
        Kernel::Microsoft
    } else {
        Kernel::Native
    };
    let marked = std::fs::metadata(DOCKERENV).is_ok()
        || std::fs::metadata(RUN_CONTAINERENV).is_ok()
        || std::fs::read_to_string(PROC_SELF_CGROUP).is_ok_and(|value| {
            let value = value.to_ascii_lowercase();
            [DOCKER, CONTAINER, LXC, KUBEPODS]
                .iter()
                .any(|marker| value.contains(marker))
        });
    let isolation = if marked {
        Isolation::Container
    } else {
        Isolation::Bare
    };
    supported(kernel, isolation)
}
#[cfg(not(unix))]
pub(crate) fn permission_checks_supported() -> Permissions {
    Permissions::Unsupported
}
