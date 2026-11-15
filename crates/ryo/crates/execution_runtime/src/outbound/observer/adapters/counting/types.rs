//! The types of the counting adapter: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::outbound::observer::types::FedTotals;

/// The totals the counting observer keeps across reports, behind the lock its reporting threads share; at most
/// `COUNTED_FED` rules of them.
#[derive(Debug, Default)]
pub(crate) struct CountedFed(pub std::sync::Mutex<FedTotals>);

/// The handovers the counting observer was told of, in the order they came, behind the lock its reporting threads share; at most
/// `COUNTED_HANDOVERS` of them.
#[derive(Debug, Default)]
pub(crate) struct CountedHandovers(
    pub std::sync::Mutex<Vec<crate::outbound::observer::types::FileHandover>>,
);
