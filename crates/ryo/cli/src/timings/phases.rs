//! Diagnostic-only outer command phase observation.
//!
//! The phases of a command are the spans the command opens with no span above them, in the order they ended. Each is
//! reported with the wall time at which it was entered and at which it was left, counted from when observation began.
//! Re-entry of a phase that is already entered, entries on a thread other than the one that created the span, and a
//! phase that was entered and never left are not reported.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;
use std::time::{Duration, Instant};

use tracing_subscriber::layer::SubscriberExt as _;

#[cfg(test)]
#[path = "phases_test.rs"]
mod tests;

/// One outer phase: its name and the wall time at which it was entered and at which it was left.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Phase {
    /// The span's name.
    name: String,
    /// When it was first entered, since observation began.
    entered: Duration,
    /// When its last entry was left, since observation began.
    left: Duration,
}

/// A phase being observed: entered at `entered`, `depth` entries deep on the thread that created it.
struct Open {
    /// The span's name.
    name: String,
    /// The thread that created the span, the only one whose entries count.
    creator: ThreadId,
    /// When its first entry opened, since observation began.
    entered: Duration,
    /// How many of its entries on the creator's thread are open.
    depth: usize,
}

/// The phases seen so far.
#[derive(Default)]
struct Log {
    /// The phases created and not yet closed, by span id.
    open: BTreeMap<u64, Open>,
    /// The phases that ended, in the order they ended.
    done: Vec<Phase>,
}

impl Log {
    /// A span with no span above it begins to be observed.
    fn create(&mut self, id: u64, name: &str, creator: ThreadId) {
        // A span id is live once: a phase it replaces is one tracing has already closed and recycled.
        self.open.extend([(
            id,
            Open {
                name: name.to_owned(),
                creator,
                entered: Duration::ZERO,
                depth: 0,
            },
        )]);
    }

    /// A span is entered on `thread` at `now`; only the first entry on its creator's thread opens the phase.
    fn enter(&mut self, id: u64, thread: ThreadId, now: Duration) {
        let Some(open) = self.open.get_mut(&id) else {
            return;
        };
        if open.creator != thread {
            return;
        }
        if open.depth == 0 {
            open.entered = now;
        }
        open.depth += 1;
    }

    /// A span is left on `thread` at `now`; the last exit on its creator's thread ends the phase.
    fn exit(&mut self, id: u64, thread: ThreadId, now: Duration) {
        let Some(open) = self.open.get_mut(&id) else {
            return;
        };
        if open.creator != thread || open.depth == 0 {
            return;
        }
        open.depth -= 1;
        if open.depth == 0 {
            self.done.push(Phase {
                name: open.name.clone(),
                entered: open.entered,
                left: now,
            });
        }
    }

    /// A span is closed: whatever was left unfinished of it is dropped.
    fn close(&mut self, id: u64) {
        drop(self.open.remove(&id));
    }
}

/// The layer that observes the phases and the table that reports them.
#[derive(Clone)]
struct Phases {
    /// The phases seen, shared by the layer and the report that prints them.
    log: Arc<Mutex<Log>>,
    /// The instant the phases' times count from.
    origin: Instant,
}

impl Phases {
    /// An observer with no phase seen, counting from now.
    fn new() -> Self {
        Self {
            log: Arc::default(),
            origin: Instant::now(),
        }
    }

    /// Writes the phases, one row each, once and whole; a writer that refuses them has no one left to tell, as the
    /// command is ending.
    fn print(&self, out: &mut impl std::io::Write) {
        let Ok(log) = self.log.lock() else {
            return;
        };
        let mut rows = vec![format!(
            "{:<24} {:>12} {:>12}",
            "phase", "entered ms", "left ms"
        )];
        rows.extend(log.done.iter().map(|phase| {
            format!(
                "{:<24} {:>12.3} {:>12.3}",
                phase.name,
                phase.entered.as_secs_f64() * 1000.0_f64,
                phase.left.as_secs_f64() * 1000.0_f64
            )
        }));
        rows.push(String::new());
        // The write is std's; a refused one leaves nothing to report it to.
        drop(out.write_all(rows.join("\n").as_bytes()));
    }
}

impl<S> tracing_subscriber::Layer<S> for Phases
where
    S: tracing::Subscriber + for<'span> tracing_subscriber::registry::LookupSpan<'span>,
{
    fn on_new_span(
        &self,
        _attributes: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let Some(span) = ctx.span(id) else { return };
        if span.parent().is_some() {
            return;
        }
        if let Ok(mut log) = self.log.lock() {
            log.create(id.into_u64(), span.name(), std::thread::current().id());
        }
    }

    fn on_enter(
        &self,
        id: &tracing::span::Id,
        _context: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if let Ok(mut log) = self.log.lock() {
            log.enter(
                id.into_u64(),
                std::thread::current().id(),
                self.origin.elapsed(),
            );
        }
    }

    fn on_exit(&self, id: &tracing::span::Id, _context: tracing_subscriber::layer::Context<'_, S>) {
        if let Ok(mut log) = self.log.lock() {
            log.exit(
                id.into_u64(),
                std::thread::current().id(),
                self.origin.elapsed(),
            );
        }
    }

    fn on_close(&self, id: tracing::span::Id, _context: tracing_subscriber::layer::Context<'_, S>) {
        if let Ok(mut log) = self.log.lock() {
            log.close(id.into_u64());
        }
    }
}

/// Prints the phases on stderr when the command ends, with the wall time from when observation began.
struct Report {
    /// The observer whose phases are printed.
    phases: Phases,
}

impl Drop for Report {
    fn drop(&mut self) {
        self.phases.print(&mut std::io::stderr());
    }
}

/// Installs the observer as the global subscriber and answers the report the parent keeps in its guard; the
/// phases are printed on stderr when the report is dropped.
///
/// # Errors
///
/// Returns an error if another global subscriber is already installed.
pub(super) fn install()
-> Result<Box<dyn std::any::Any + Send>, tracing::subscriber::SetGlobalDefaultError> {
    let phases = Phases::new();
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(tracing_subscriber::filter::LevelFilter::INFO)
            .with(phases.clone()),
    )?;
    Ok(Box::new(Report { phases }))
}
