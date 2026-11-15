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

/// The rows one file handed the steps after the files of one rule, every input of it together, against the rows of the file's syntax
/// tree: how far the rule's per-file steps widen what the language read of the file.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::coverage::types::RuleIndex;
/// use ryo_execution_runtime::outbound::observer::types::FileHandover;
/// use ryo_values::domain::runs::types::{ParsedRows, RowsHanded};
/// use ryo_relations::domain::relations::types::FileId;
/// let handover = FileHandover { file: FileId(0), rule: RuleIndex(1), handed: RowsHanded(8), parsed: ParsedRows(4) };
/// assert_eq!(handover.handed, RowsHanded(8));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileHandover {
    /// The file.
    pub file: ryo_relations::domain::relations::types::FileId,
    /// The rule.
    pub rule: ryo_execution_domain::domain::coverage::types::RuleIndex,
    /// The rows the file handed the rule's steps after the files.
    pub handed: ryo_values::domain::runs::types::RowsHanded,
    /// The rows of the file's syntax tree.
    pub parsed: ryo_values::domain::runs::types::ParsedRows,
}
