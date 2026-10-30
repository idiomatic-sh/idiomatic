//! Proves when the progress line is drawn on stderr, and that `settle` takes it down.

use std::sync::{Arc, Mutex};

use tracing_subscriber::layer::SubscriberExt as _;

use super::{Message, PROGRESS, settle};

/// Keeps the message of every event it sees.
struct Seen(Arc<Mutex<Vec<String>>>);

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Seen {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _context: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut message = Message::default();
        event.record(&mut message);
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(message.0);
    }
}

/// The line draws the `message` of an event and nothing else of it.
#[test]
fn progress_succeeds_reading_the_message_of_an_event() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::registry().with(Seen(Arc::clone(&seen)));
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(target: "progress", files = 3_u32, "checking the files");
    });
    let messages = seen
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    assert_eq!(messages, vec!["checking the files".to_owned()]);
}

/// `settle` leaves no line drawn, whether one was drawn or not.
#[test]
fn settle_succeeds_leaving_no_line() {
    settle();
    settle();
    assert!(
        PROGRESS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_none()
    );
}
