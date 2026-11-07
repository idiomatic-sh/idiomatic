//! The plain data of one certification.
use crate::outbound::inventory::types::Registration;
use std::path::PathBuf;

/// Where the cases of a certification come from.
pub enum PreparedCases {
    /// The cases the audited profile authors.
    Profile,
    /// The cases a registered context pins, run into an output directory.
    Registered {
        /// The loaded registration.
        registration: Registration,
        /// Where the inventory result is published.
        output: PathBuf,
    },
}

/// A loaded registration and every artifact path it pins.
pub struct RegisteredContext {
    /// The loaded registration.
    pub registration: Registration,
    /// Every pinned artifact path, sorted and without duplicates.
    pub paths: Vec<PathBuf>,
}
