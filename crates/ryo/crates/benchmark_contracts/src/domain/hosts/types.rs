//! The facts of the host that decide whether a file permission can be trusted.

/// Whether the host has POSIX file permissions at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostFamily {
    /// A Unix host: files carry an execute bit.
    Unix,
    /// Any other host.
    Other,
}

/// What the kernel reports about the host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostKernel {
    /// A Microsoft-built kernel, as under WSL.
    Microsoft,
    /// Any other kernel.
    Native,
}

/// Whether the process runs inside a container.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostIsolation {
    /// A container runtime marker was found.
    Container,
    /// No container marker was found.
    Bare,
}

/// What a host port observed about the machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostFacts {
    /// Whether files carry POSIX permissions.
    pub family: HostFamily,
    /// The kernel the host reports.
    pub kernel: HostKernel,
    /// Whether a container runtime isolates the process.
    pub isolation: HostIsolation,
}

/// Whether the host honours the file permissions a reference observes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilePermissions {
    /// File permission checks answer as the reference expects.
    Honoured,
    /// File permission checks cannot be trusted on this host.
    Unreliable,
}
