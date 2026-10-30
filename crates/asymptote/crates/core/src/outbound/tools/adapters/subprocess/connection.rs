//! The process groups of the tools running now, opened at the edge and shared with every adapter it binds, so the
//! edge can stop them all when a signal ends the command.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, PoisonError};

use crate::outbound::tools::types::GroupId;

/// The running tools' process groups, shared by the adapters bound to it and the edge that opened it.
#[derive(Clone, Debug, Default)]
pub struct ToolsConnection {
    /// The groups of the tools running now.
    running: Arc<Mutex<BTreeSet<GroupId>>>,
}

impl ToolsConnection {
    /// A connection with no tool running.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Notes that the tool leading `group` runs.
    pub(super) fn started(&self, group: GroupId) {
        let _: bool = self.groups().insert(group);
    }

    /// Notes that the tool leading `group` ended.
    pub(super) fn ended(&self, group: GroupId) {
        let _: bool = self.groups().remove(&group);
    }

    /// The first group running now, if any.
    #[cfg(test)]
    pub(super) fn first(&self) -> Option<GroupId> {
        self.groups().first().copied()
    }

    /// Kills every tool running now, with everything in its process group.
    #[cfg(unix)]
    pub fn stop(&self) {
        for group in self.groups().iter() {
            if let Ok(group) = i32::try_from(group.0) {
                // Best effort: a group that already ended cannot be killed.
                nix::sys::signal::killpg(
                    nix::unistd::Pid::from_raw(group),
                    nix::sys::signal::Signal::SIGKILL,
                )
                .unwrap_or_default();
            }
        }
    }

    /// Kills nothing where a tool's job object already ends with the command.
    #[cfg(not(unix))]
    pub fn stop(&self) {}

    /// The running groups, under their lock.
    fn groups(&self) -> std::sync::MutexGuard<'_, BTreeSet<GroupId>> {
        self.running.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
