//! The process groups of the tools running now, opened at the edge and shared with every adapter it binds, so the
//! edge can stop them all when a signal ends the command.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use crate::outbound::tools::types::GroupId;

/// The running tools' process groups, shared by the adapters bound to it and the edge that opened it.
#[derive(Clone, Debug, Default)]
pub struct ToolsConnection {
    /// The groups of the tools running now.
    running: Arc<Mutex<BTreeSet<GroupId>>>,
    /// Whether the command is stopping: a run looking at its child ends it.
    stopped: Arc<AtomicBool>,
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

    /// Kills every tool running now, with everything in its process group.
    #[cfg(unix)]
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
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

    /// Ends every tool running now and any started after, with every process each started: Windows has no process groups,
    /// so each tool's run, which holds its job object, ends the job when it next looks at its child.
    #[cfg(windows)]
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
    }

    /// Ends `child`, with everything it started (its group, or on Windows its job), when the command is stopping.
    pub(super) fn end_if_stopping(&self, child: &mut dyn process_wrap::std::ChildWrapper) {
        if self.stopped.load(Ordering::SeqCst) {
            // Best effort: a tool that already ended cannot be ended.
            drop(child.start_kill());
        }
    }

    /// Kills nothing where there is neither a process group nor a job object to end.
    #[cfg(not(any(unix, windows)))]
    pub fn stop(&self) {}

    /// The running groups, under their lock.
    fn groups(&self) -> std::sync::MutexGuard<'_, BTreeSet<GroupId>> {
        self.running.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
