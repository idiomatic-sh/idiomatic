//! Builds the observer adapters, one function each. Every dependency arrives as a
//! parameter from the edge that composes: settings, the connection or client the edge
//! opened, shared caches. A factory builds nothing else, chooses nothing, and is called
//! only by an edge.

/// The counting observer: it adds up what it is told.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::coverage::types::RuleIndex;
/// use ryo_execution_domain::domain::operator::types::{LogicalCounts, RowsFed};
/// use ryo_execution_runtime::outbound::observer::{factory::counting, interface::ObserverPort, types::FedTotals};
/// let observer = counting();
/// observer.fed(RuleIndex(0), LogicalCounts { fed: Box::new([RowsFed([3, 0, 0])]) })?;
/// assert_eq!(observer.totals()?, FedTotals(vec![vec![RowsFed([3, 0, 0])]]));
/// # Ok::<(), ryo_execution_domain::domain::operating::errors::OperatingError>(())
/// ```
#[must_use]
pub fn counting() -> impl crate::outbound::observer::interface::ObserverPort {
    super::adapters::counting::adapter::ObserverCountingAdapter::default()
}
