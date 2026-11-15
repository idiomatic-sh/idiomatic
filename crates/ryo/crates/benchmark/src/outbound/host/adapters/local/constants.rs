//! The files and words the local host probe reads.
/// The kernel release of a Linux host.
pub(super) const PROC_SYS_KERNEL_OSRELEASE: &str = "/proc/sys/kernel/osrelease";
/// The kernel version text of a Linux host, read when the release is unreadable.
pub(super) const PROC_VERSION: &str = "/proc/version";
/// What a Microsoft-built kernel names itself.
pub(super) const MICROSOFT: &str = "microsoft";
/// The marker file a Docker runtime leaves.
pub(super) const DOCKERENV: &str = "/.dockerenv";
/// The marker file a Podman runtime leaves.
pub(super) const RUN_CONTAINERENV: &str = "/run/.containerenv";
/// The control groups of this process.
pub(super) const PROC_SELF_CGROUP: &str = "/proc/self/cgroup";
/// The words a container's control groups contain.
pub(super) const CONTAINER_MARKERS: [&str; 4] = ["docker", "/container", "/lxc", "kubepods"];
/// The operating system name of a Linux host.
pub(super) const LINUX: &str = "linux";
