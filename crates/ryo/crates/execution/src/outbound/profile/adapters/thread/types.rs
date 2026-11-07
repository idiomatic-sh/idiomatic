//! The types of the thread adapter: what it keeps across calls.
use std::cell::RefCell;
use thread_local::ThreadLocal;

/// The profiles the threads that visited the adapter keep, one for each thread: a profile lives from the start of a file to its end on the thread that runs it.
#[derive(Debug)]
pub struct ThreadProfiles<State: Send>(pub ThreadLocal<RefCell<Option<State>>>);
