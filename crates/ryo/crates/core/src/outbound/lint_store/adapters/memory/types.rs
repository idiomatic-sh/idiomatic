//! The types of the memory store adapter: plain data, public fields and derives, with no constructor and no inherent impl.

use std::sync::{Arc, Mutex};

use ryo_values::domain::stores::types::Stored;

/// The store the adapter keeps: the last run's, which each keep replaces whole.
#[derive(Clone, Debug, Default)]
pub(crate) struct KeptStore(pub Arc<Mutex<Arc<Stored>>>);

/// The store as one run read it: what it held then, and the place the next keep goes to.
#[derive(Debug)]
pub(crate) struct MemoryRead {
    /// What the adapter keeps, shared with it.
    pub cell: KeptStore,
    /// What the store held when the run read it.
    pub was: Arc<Stored>,
}
