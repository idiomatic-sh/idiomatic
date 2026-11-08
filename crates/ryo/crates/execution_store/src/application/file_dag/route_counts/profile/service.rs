//! The profile a file scope records into, reached only through the profile its parent hands down.
use super::super::types::{RouteProfile, RouteState};

/// Runs `visit` over the profile's state while it records; nothing when it does not.
pub(in super::super) fn with_state<R>(
    profile: &RouteProfile,
    visit: impl FnOnce(&mut RouteState) -> R,
) -> Option<R> {
    let state = profile.0.as_ref()?;
    let mut state = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Some(visit(&mut state))
}
