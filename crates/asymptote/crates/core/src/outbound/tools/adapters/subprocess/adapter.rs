//! The tools port, implemented with `std::process` and `process-wrap`: a tool leads a process group of its own (a
//! job object on Windows), so its timeout kills everything it spawned; both pipes drain on threads of their own, so
//! neither fills and blocks it. Its group is out of reach of the terminal's Ctrl-C, so it is noted in the connection
//! the edge opened while it runs, for the edge to stop when a signal ends the command.

use std::io::Read as _;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use process_wrap::std::CommandWrap;
#[cfg(windows)]
use process_wrap::std::JobObject;
#[cfg(unix)]
use process_wrap::std::{ProcessGroup, ResetSigmask};

use super::connection::ToolsConnection;
use super::constants::{EMPTY, MAX_OUTPUT, POLL, REMOVED, SET};
use crate::outbound::tools::interface::ToolsPort;
use crate::outbound::tools::types::{
    Captured, Completion, Ending, ExitCode, GroupId, Invocation, StartProblem,
};

/// The subprocess tools adapter.
#[derive(Debug)]
pub(crate) struct ToolsSubprocessAdapter {
    /// Where the running tools' groups are noted, for the edge that opened it.
    pub connection: ToolsConnection,
}

/// A completion that never ran.
fn unstartable(reason: impl std::fmt::Display) -> Completion {
    Completion {
        ending: Ending::Unstartable(StartProblem(reason.to_string().into())),
        stdout: Captured::default(),
        stderr: Captured::default(),
    }
}

impl ToolsPort for ToolsSubprocessAdapter {
    fn run(&self, invocation: &Invocation) -> Completion {
        let Some((program, arguments)) = invocation.argv.0.split_first() else {
            return unstartable(EMPTY);
        };
        let mut command = Command::new(&*program.0);
        // The builder hands back the command it configures; only its effect on `command` is wanted.
        let _: &mut Command = REMOVED
            .iter()
            .fold(
                command
                    .args(arguments.iter().map(|argument| &*argument.0))
                    .current_dir(&*invocation.directory.0)
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped()),
                |command, key| command.env_remove(key),
            )
            .envs(SET);
        // The tool starts with no signal blocked, whatever the command blocks to take its own.
        #[cfg(unix)]
        let spawned = CommandWrap::from(command)
            .wrap(ProcessGroup::leader())
            .wrap(ResetSigmask)
            .spawn();
        #[cfg(windows)]
        let spawned = CommandWrap::from(command).wrap(JobObject).spawn();
        #[cfg(not(any(unix, windows)))]
        let spawned = CommandWrap::from(command).spawn();
        let mut child = match spawned {
            Ok(child) => child,
            Err(error) => return unstartable(format!("{}: {error}", program.0)),
        };
        // The group leader's id is the group's.
        let group = GroupId(child.id());
        self.connection.started(group);
        let stdout = Arc::new(Mutex::new(Captured::default()));
        let stderr = Arc::new(Mutex::new(Captured::default()));
        let readers = [
            drain(child.stdout().take(), Arc::clone(&stdout)),
            drain(child.stderr().take(), Arc::clone(&stderr)),
        ];
        let timeout = Duration::from_secs(invocation.timeout.0);
        let started = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) => {}
                Err(error) => {
                    self.connection.ended(group);
                    return unstartable(error);
                }
            }
            if started.elapsed() >= timeout {
                // Best effort: a group that already ended cannot be killed, and the run is a timeout either way.
                drop(child.kill());
                drop(child.wait());
                break None;
            }
            std::thread::sleep(POLL);
        };
        // Its output is complete when every process holding its pipes has ended too; one it left in its group is
        // still its work, held to the same deadline. A reader blocked on a process outside the killed group is
        // abandoned, since waiting there would defeat the timeout.
        let status = status.and_then(|status| {
            while !readers
                .iter()
                .flatten()
                .all(std::thread::JoinHandle::is_finished)
            {
                if started.elapsed() >= timeout {
                    // Best effort, as above: the run is a timeout whether or not the kill lands.
                    drop(child.start_kill());
                    return None;
                }
                std::thread::sleep(POLL);
            }
            Some(status)
        });
        if status.is_some() {
            for reader in readers.into_iter().flatten() {
                // A reader that panicked leaves its capture as it was; the run's ending stands.
                drop(reader.join());
            }
        }
        self.connection.ended(group);
        let ending = match status {
            None => Ending::TimedOut,
            Some(status) => status
                .code()
                .map_or(Ending::Signalled, |code| Ending::Exited(ExitCode(code))),
        };
        Completion {
            ending,
            stdout: take(&stdout),
            stderr: take(&stderr),
        }
    }
}

/// Drains a pipe into `into` on a thread of its own.
fn drain(
    pipe: Option<impl std::io::Read + Send + 'static>,
    into: Arc<Mutex<Captured>>,
) -> Option<std::thread::JoinHandle<()>> {
    let pipe = pipe?;
    Some(std::thread::spawn(move || {
        let mut buffer = Vec::new();
        // A pipe that fails mid-read keeps what it gave: the capture is what the tool was seen to write.
        drop(
            pipe.take(MAX_OUTPUT.saturating_add(1))
                .read_to_end(&mut buffer),
        );
        *into.lock().unwrap_or_else(PoisonError::into_inner) =
            Captured(String::from_utf8_lossy(&buffer).into());
    }))
}

/// What a drained pipe captured, copied out from under its lock.
fn take(buffer: &Arc<Mutex<Captured>>) -> Captured {
    buffer
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}
