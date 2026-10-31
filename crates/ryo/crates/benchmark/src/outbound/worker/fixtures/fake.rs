//! Stateful worker transport double records receive and response ordering evidence.
use crate::domain::diagnostics::{errors::DiagnosticsError as Error, types::RunReport};
use crate::domain::suite::types::WorkerRequest;
use crate::outbound::worker::interface::WorkerPort;
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

/// Controllable transport failures and observations for orchestration tests.
#[derive(Debug)]
#[expect(
    clippy::partial_pub_fields,
    reason = "tests configure failures publicly while observations remain controlled by the transport"
)]
pub struct WorkerFake {
    /// Immutable request returned by successful receive calls.
    request: WorkerRequest,
    /// Fail receiving before the engine can run.
    pub refuse_receive: bool,
    /// Fail writing instead of recording a published report.
    pub refuse_respond: bool,
    /// Total receive attempts across fixture threads.
    received: AtomicUsize,
    /// Total response attempts across fixture threads.
    responded: AtomicUsize,
    /// Successful publications retained in call order.
    published: Mutex<Vec<RunReport>>,
}
impl WorkerFake {
    /// Create a transport over one immutable fixture request.
    #[must_use]
    pub fn new(request: WorkerRequest) -> Self {
        Self {
            request,
            refuse_receive: false,
            refuse_respond: false,
            received: AtomicUsize::new(0),
            responded: AtomicUsize::new(0),
            published: Mutex::new(Vec::new()),
        }
    }
    /// Number of receive attempts, including failures.
    #[must_use]
    pub fn receive_calls(&self) -> usize {
        self.received.load(Ordering::SeqCst)
    }
    /// Number of response attempts, including failures.
    #[must_use]
    pub fn respond_calls(&self) -> usize {
        self.responded.load(Ordering::SeqCst)
    }
    /// Successfully published reports in order.
    /// # Panics
    /// Panics if another fixture thread poisoned the publication lock.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "a poisoned fixture lock is a test failure"
    )]
    pub fn reports(&self) -> Vec<RunReport> {
        self.published.lock().expect("fixture report lock").clone()
    }
}
impl WorkerPort for WorkerFake {
    fn receive(&self) -> Result<WorkerRequest, Error> {
        let _previous = self.received.fetch_add(1, Ordering::SeqCst);
        if self.refuse_receive {
            return Err(Error::Invalid("fixture receive failure".into()));
        }
        Ok(self.request.clone())
    }
    #[expect(
        clippy::expect_used,
        reason = "a poisoned fixture lock is a test failure"
    )]
    fn respond(&self, report: &RunReport) -> Result<(), Error> {
        let _previous = self.responded.fetch_add(1, Ordering::SeqCst);
        if self.refuse_respond {
            return Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe).into());
        }
        self.published
            .lock()
            .expect("fixture report lock")
            .push(report.clone());
        Ok(())
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
