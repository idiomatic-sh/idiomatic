//! Pure host applicability policy, shared by the executable and its test target.

/// What the kernel reports about the host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kernel {
    /// A Microsoft-built kernel, as under WSL.
    Microsoft,
    /// Any other kernel.
    Native,
}

/// Whether the process runs inside a container.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Isolation {
    /// A container runtime marker was found.
    Container,
    /// No container marker was found.
    Bare,
}

/// Whether the host honours the file permissions the reference observes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Permissions {
    /// File permission checks answer as the reference expects.
    Supported,
    /// File permission checks cannot be trusted on this host.
    Unsupported,
}

pub(crate) fn supported(kernel: Kernel, isolation: Isolation) -> Permissions {
    match (kernel, isolation) {
        (Kernel::Microsoft, Isolation::Bare) => Permissions::Unsupported,
        (Kernel::Microsoft, Isolation::Container) | (Kernel::Native, _) => Permissions::Supported,
    }
}

#[cfg(test)]
#[test]
fn wsl_reference_policy_does_not_misclassify_containers_or_rewrite_facts() {
    assert_eq!(
        supported(Kernel::Native, Isolation::Bare),
        Permissions::Supported
    );
    assert_eq!(
        supported(Kernel::Microsoft, Isolation::Bare),
        Permissions::Unsupported
    );
    assert_eq!(
        supported(Kernel::Microsoft, Isolation::Container),
        Permissions::Supported
    );
}
