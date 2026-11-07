//! The profile port, implemented with a profile for each thread.

use super::constants::THREAD_PROFILES;
use super::types::ThreadProfiles;
use crate::outbound::profile::interface::ProfilePort;
use std::cell::RefCell;
use thread_local::ThreadLocal;

/// The thread adapter: each thread that visits it finds its own profile.
#[derive(Debug)]
pub(crate) struct ProfileThreadAdapter<State: Send> {
    profiles: ThreadProfiles<State>,
}

impl<State: Send> ProfileThreadAdapter<State> {
    /// An adapter no thread has visited.
    pub(crate) fn new() -> Self {
        Self {
            profiles: ThreadProfiles(ThreadLocal::with_capacity(THREAD_PROFILES)),
        }
    }
}

impl<State: Send + std::fmt::Debug> ProfilePort<State> for ProfileThreadAdapter<State> {
    fn with_profile(&self, visit: &mut dyn FnMut(&mut Option<State>)) {
        visit(&mut self.profiles.0.get_or(|| RefCell::new(None)).borrow_mut());
    }
}
