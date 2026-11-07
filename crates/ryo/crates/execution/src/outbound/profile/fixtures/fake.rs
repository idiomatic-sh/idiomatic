//! The fake of the `profile` port: what the engine's tests stand in for it with.

use crate::outbound::profile::interface::ProfilePort;
use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use std::thread::ThreadId;

/// What the engine's tests stand in for `profile` with: one profile for each thread, kept in a map.
#[derive(Debug)]
pub struct ProfileFake<State>(Mutex<HashMap<ThreadId, Option<State>>>);

impl<State> Default for ProfileFake<State> {
    fn default() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

impl<State: Send + std::fmt::Debug> ProfilePort<State> for ProfileFake<State> {
    fn with_profile(&self, visit: &mut dyn FnMut(&mut Option<State>)) {
        let mut profiles = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        visit(profiles.entry(std::thread::current().id()).or_default());
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
