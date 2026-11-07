//! The per-file profile of the thread that runs a file: opened by the file's scope, closed when it ends.
use super::super::types::RouteState;
use std::cell::RefCell;

thread_local! {
    /// The profile of the file the calling thread runs; none outside a file scope.
    static STATE: RefCell<Option<Box<RouteState>>> = const { RefCell::new(None) };
}

/// Runs `visit` over the calling thread's profile and keeps what it leaves.
pub(in super::super) fn with_borrow_mut<R>(
    visit: impl FnOnce(&mut Option<Box<RouteState>>) -> R,
) -> R {
    STATE.with_borrow_mut(visit)
}

/// Runs `visit` over the calling thread's profile.
#[cfg(any(test, feature = "diagnostic-row-progress"))]
pub(in super::super) fn with_borrow<R>(visit: impl FnOnce(&Option<Box<RouteState>>) -> R) -> R {
    STATE.with_borrow(visit)
}
