//! What the engine asks of the outside to keep its per-file diagnostic profile: one value for each thread, of the type the engine names.

/// What every `profile` adapter implements.
pub trait ProfilePort<State>: Send + Sync + std::fmt::Debug {
    /// Runs `visit` over the profile of the calling thread, which starts empty; a profile of another thread is never seen, and what `visit` leaves is kept.
    fn with_profile(&self, visit: &mut dyn FnMut(&mut Option<State>));
}
