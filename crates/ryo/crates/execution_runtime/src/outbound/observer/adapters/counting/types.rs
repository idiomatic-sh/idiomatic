//! The types of the counting adapter: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::outbound::observer::types::FedTotals;

/// The totals the counting observer keeps across reports, behind the lock its reporting threads share; at most
/// `COUNTED_FED` rules of them.
#[derive(Debug, Default)]
pub(crate) struct CountedFed(pub(crate) std::sync::Mutex<FedTotals>);
