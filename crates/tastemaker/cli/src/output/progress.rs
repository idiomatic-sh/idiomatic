//! One line of progress on stderr while a long command works (clig.dev, Robustness): each
//! event a service emits with the target `progress`
//! (`tracing::info!(target: "progress", "checking {phase}")`), drawn with indicatif, only
//! when stderr is a terminal and `--quiet` was not asked.

#[cfg(test)]
#[path = "progress_test.rs"]
mod tests;

/// The one line of progress a long command draws on stderr, while one is drawn: taken
/// down by [`settle`] and drawn afresh by the next progress event.
static PROGRESS: std::sync::Mutex<Option<indicatif::ProgressBar>> = std::sync::Mutex::new(None);

/// Whether the command line asked for `--quiet`: progress and planned changes are
/// dropped then, as the answer is.
pub(super) static QUIET: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The tracing layer that draws each `progress` event as the progress line: present only
/// when stderr is a terminal and `--quiet` was not asked, so a command whose stderr is a
/// file or a pipe installs nothing. The `timings` module adds it to the subscriber it
/// installs.
#[must_use]
pub fn layer<S>() -> Option<impl tracing_subscriber::Layer<S>>
where
    S: tracing::Subscriber + for<'span> tracing_subscriber::registry::LookupSpan<'span>,
{
    use std::io::IsTerminal as _;
    let drawn =
        std::io::stderr().is_terminal() && !QUIET.load(std::sync::atomic::Ordering::Relaxed);
    drawn.then(|| {
        tracing_subscriber::Layer::with_filter(
            Progress,
            tracing_subscriber::filter::Targets::new()
                .with_target("progress", tracing::Level::INFO),
        )
    })
}

/// Draws each `progress` event's message as the progress line.
struct Progress;

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Progress {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _context: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut message = Message::default();
        event.record(&mut message);
        PROGRESS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_or_insert_with(|| {
                let line = indicatif::ProgressBar::new_spinner();
                line.enable_steady_tick(std::time::Duration::from_millis(120));
                line
            })
            .set_message(message.0);
    }
}

/// An event's `message` field.
#[derive(Default)]
pub(super) struct Message(String);

impl Message {
    /// The message recorded.
    pub(super) fn into_text(self) -> String {
        self.0
    }
}

impl tracing::field::Visit for Message {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = format!("{value:?}");
        }
    }
}

/// Clears the progress line, so an answer, an error or a planned change starts on a
/// clean line; `resources::run` calls it once the command has ended.
pub fn settle() {
    let line = PROGRESS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take();
    if let Some(line) = line {
        line.finish_and_clear();
    }
}
