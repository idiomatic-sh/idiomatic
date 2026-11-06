//! The words and files by which the host is recognised as a container or WSL.
/// Text `linux`.
pub(crate) const LINUX: &str = "linux";
/// Text `/proc/sys/kernel/osrelease`.
pub(crate) const PROC_SYS_KERNEL_OSRELEASE: &str = "/proc/sys/kernel/osrelease";
/// Text `/proc/version`.
pub(crate) const PROC_VERSION: &str = "/proc/version";
/// Text `microsoft`.
pub(crate) const MICROSOFT: &str = "microsoft";
/// Text `/.dockerenv`.
pub(crate) const DOCKERENV: &str = "/.dockerenv";
/// Text `/run/.containerenv`.
pub(crate) const RUN_CONTAINERENV: &str = "/run/.containerenv";
/// Text `/proc/self/cgroup`.
pub(crate) const PROC_SELF_CGROUP: &str = "/proc/self/cgroup";
/// Text `docker`.
pub(crate) const DOCKER: &str = "docker";
/// Text `/container`.
pub(crate) const CONTAINER: &str = "/container";
/// Text `/lxc`.
pub(crate) const LXC: &str = "/lxc";
/// Text `kubepods`.
pub(crate) const KUBEPODS: &str = "kubepods";
