//! Reads the kernel release and container markers of this machine, and decides nothing.
use super::constants;
use crate::outbound::host::interface::HostPort;
use ryo_benchmark_contracts::domain::hosts::types::{
    HostFacts, HostFamily, HostIsolation, HostKernel,
};

/// The local host probe.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct HostLocalAdapter;

impl HostPort for HostLocalAdapter {
    fn processors(&self) -> crate::outbound::host::types::ProcessorCount {
        // The processors this process may run on: `taskset` bounds the affinity mask the standard library reads.
        crate::outbound::host::types::ProcessorCount(
            std::thread::available_parallelism().unwrap_or(std::num::NonZeroUsize::MIN),
        )
    }
    fn facts(&self) -> HostFacts {
        let family = if cfg!(unix) {
            HostFamily::Unix
        } else {
            HostFamily::Other
        };
        if std::env::consts::OS != constants::LINUX {
            return HostFacts {
                family,
                kernel: HostKernel::Native,
                isolation: HostIsolation::Bare,
            };
        }
        let release = std::fs::read_to_string(constants::PROC_SYS_KERNEL_OSRELEASE)
            .or_else(|_| std::fs::read_to_string(constants::PROC_VERSION));
        let kernel = if release
            .is_ok_and(|value| value.to_ascii_lowercase().contains(constants::MICROSOFT))
        {
            HostKernel::Microsoft
        } else {
            HostKernel::Native
        };
        let marked = std::fs::metadata(constants::DOCKERENV).is_ok()
            || std::fs::metadata(constants::RUN_CONTAINERENV).is_ok()
            || std::fs::read_to_string(constants::PROC_SELF_CGROUP).is_ok_and(|value| {
                let value = value.to_ascii_lowercase();
                constants::CONTAINER_MARKERS
                    .iter()
                    .any(|marker| value.contains(marker))
            });
        let isolation = if marked {
            HostIsolation::Container
        } else {
            HostIsolation::Bare
        };
        HostFacts {
            family,
            kernel,
            isolation,
        }
    }
}
