//! The one owner of retained-byte accounting: the ledger of the leases on physical backings, the footprint of each derived
//! payload it is told, and the backings of a relation's packed words.
pub mod footprint;
pub mod ledger;
pub(crate) mod packed;
