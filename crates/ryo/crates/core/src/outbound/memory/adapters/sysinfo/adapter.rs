//! The memory port, implemented with sysinfo.

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

use crate::outbound::memory::errors::MemoryError;
use crate::outbound::memory::interface::MemoryPort;
use ryo_values::domain::runs::types::{Budget, MemoryBytes};

/// The sysinfo memory adapter, over the budget the edge asked for.
#[derive(Debug)]
pub(crate) struct MemorySysinfoAdapter {
    /// The most memory the run may use, as asked.
    asked: Budget,
}

impl MemorySysinfoAdapter {
    /// An adapter over the budget `asked`.
    pub(crate) const fn new(asked: Budget) -> Self {
        Self { asked }
    }
}

impl MemoryPort for MemorySysinfoAdapter {
    fn budget(&self) -> Result<MemoryBytes, MemoryError> {
        let mut system = System::new();
        system.refresh_memory();
        let available = match system.available_memory() {
            0 => return Err(MemoryError::NoneAvailable),
            bytes => MemoryBytes(bytes),
        };
        match self.asked {
            Budget::Machine => Ok(available),
            Budget::Bytes(asked) if asked > available => Err(MemoryError::MoreThanAvailable {
                asked: asked.0,
                available: available.0,
            }),
            Budget::Bytes(asked) => Ok(asked),
        }
    }

    fn resident_bytes(&self) -> Result<MemoryBytes, MemoryError> {
        let pid = sysinfo::get_current_pid().map_err(|reason| MemoryError::Unreadable {
            reason: reason.to_owned(),
        })?;
        let mut system = System::new();
        let refreshed = system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            true,
            ProcessRefreshKind::nothing().with_memory(),
        );
        // A process the system did not refresh holds nothing it can say.
        Ok(MemoryBytes(if refreshed == 0 {
            0
        } else {
            system.process(pid).map_or(0, sysinfo::Process::memory)
        }))
    }
}
