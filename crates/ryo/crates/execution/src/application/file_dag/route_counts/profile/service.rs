//! The per-file profile, kept by the injected profile port.
use super::super::types::RouteState;
use crate::outbound::profile::interface::ProfilePort;
use std::sync::{Arc, OnceLock};

/// The profile port the edge composed; without one no profile is recorded.
static PORT: OnceLock<Arc<dyn ProfilePort<RouteProfile>>> = OnceLock::new();

/// Hands the engine the port that keeps its per-file profile; the first port installed is kept.
pub fn install(port: Arc<dyn ProfilePort<RouteProfile>>) {
    let _first = PORT.set(port);
}

/// The profile of one file as the profile port keeps it; only the engine opens it.
#[derive(Debug)]
pub struct RouteProfile(pub(in super::super) Box<RouteState>);

/// Runs `visit` over the calling thread's profile and keeps what it leaves; with no port, over a profile that is dropped.
pub(in super::super) fn with_borrow_mut<R>(
    visit: impl FnOnce(&mut Option<Box<RouteState>>) -> R,
) -> R {
    let mut visit = Some(visit);
    let mut answer = None;
    if let Some(port) = PORT.get() {
        port.with_profile(&mut |profile| {
            if let Some(visit) = visit.take() {
                let mut state = profile.take().map(|kept| kept.0);
                answer = Some(visit(&mut state));
                *profile = state.map(RouteProfile);
            }
        });
    }
    match (answer, visit) {
        (Some(answer), _) => answer,
        (None, Some(visit)) => visit(&mut None),
        (None, None) => unreachable!(),
    }
}

/// Runs `visit` over the calling thread's profile.
pub(in super::super) fn with_borrow<R>(visit: impl FnOnce(&Option<Box<RouteState>>) -> R) -> R {
    with_borrow_mut(|state| visit(state))
}
