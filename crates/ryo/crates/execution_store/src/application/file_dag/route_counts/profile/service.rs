//! The profile a file scope records into, reached only through the profile its parent hands down.
use std::sync::{Arc, Mutex};

use super::super::types::{RouteOrigin, RouteState};

/// Runs `visit` over the profile's state while it records; nothing when it does not.
pub(crate) fn with_state<R>(
    profile: &RouteProfile,
    visit: impl FnOnce(&mut RouteState) -> R,
) -> Option<R> {
    let state = profile.0.as_ref()?;
    let mut state = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Some(visit(&mut state))
}

/// The profile of one file, opened by its scope and handed down to every part that records
/// into it; it holds no state when the scope does not record.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::route_counts::profile::service::RouteProfile;
/// let profile = RouteProfile::default();
/// let _shared = profile.clone();
/// ```
#[derive(Clone, Debug, Default)]
pub struct RouteProfile(pub(in super::super) Option<Arc<Mutex<RouteState>>>);

/// The profile of one file while its scope is open; it reports the file's counts when it closes.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::route_counts::profile::service::FileScope;
/// let scope = FileScope::begin(0);
/// let _profile = scope.profile();
/// ```
#[derive(Debug)]
pub struct FileScope {
    pub(in super::super) profile: RouteProfile,
}

/// The origin a profile records under until this scope ends, when the previous one is restored.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::route_counts::profile::service::OriginScope;
/// fn restore(scope: OriginScope) {
///     drop(scope);
/// }
/// ```
#[derive(Debug)]
pub struct OriginScope {
    pub(in super::super) profile: RouteProfile,
    pub(in super::super) previous: Option<RouteOrigin>,
}

/// The origin a profile had when an invocation began, with the profile it belongs to.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::route_counts::profile::service::CapturedOrigin;
/// fn captured(origin: &CapturedOrigin) -> CapturedOrigin {
///     origin.clone()
/// }
/// ```
#[cfg(feature = "diagnostic-row-progress")]
#[derive(Clone, Debug)]
pub struct CapturedOrigin {
    pub(in super::super) profile: RouteProfile,
    pub(in super::super) origin: Option<RouteOrigin>,
}
