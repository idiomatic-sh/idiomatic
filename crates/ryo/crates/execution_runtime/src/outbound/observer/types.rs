//! The types of the `observer` port: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_execution_domain::domain::operator::types::RowsFed;

/// The rows each step of each rule was fed, by rule then step: a rule or step the observer was told nothing of is absent.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FedTotals(pub Vec<Vec<RowsFed>>);
