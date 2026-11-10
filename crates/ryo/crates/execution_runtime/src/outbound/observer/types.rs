//! The types of the `observer` port: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_execution_domain::domain::operator::types::RowsFed;

/// The rows each step of each rule was fed, by rule then step: a rule or step the observer was told nothing of is absent.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::coverage::types::RuleIndex;
/// use ryo_execution_domain::domain::operator::types::{LogicalCounts, RowsFed};
/// use ryo_execution_runtime::outbound::observer::{factory::counting, interface::ObserverPort, types::FedTotals};
/// let observer = counting();
/// observer.fed(RuleIndex(0), LogicalCounts { fed: Box::new([RowsFed([1, 0, 0])]) })?;
/// assert_eq!(observer.totals()?.0.len(), 1);
/// assert!(FedTotals::default().0.is_empty());
/// # Ok::<(), ryo_execution_domain::domain::operating::errors::OperatingError>(())
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FedTotals(pub Vec<Vec<RowsFed>>);
