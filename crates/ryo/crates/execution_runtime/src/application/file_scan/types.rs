//! The types of the `file_scan` operation's harness.

use ryo_engine::domain::programs::types::Programs;
use ryo_relations::domain::relations::types::{Count, FileId};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// What the new engine's `load` made of the selected rules: the programs of the rules it accepts, and how many it refused, by
/// why.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::types::HarnessLoad;
///
/// assert!(HarnessLoad::default().accepted.is_empty());
/// ```
#[derive(Debug, Default)]
pub struct HarnessLoad {
    /// The programs of the accepted rules.
    pub programs: Programs,
    /// The accepted rules, in rule order.
    pub accepted: Vec<SelectedRuleOrdinal>,
    /// The rules whose plan the lowering cannot express.
    pub unlowerable: Count,
    /// The rules whose programs are over the lowering's budget.
    pub over_budget: Count,
}

/// One engine's rows of the compared rules: per rule and file, each row as its owned text, sorted.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::types::HarnessRows;
///
/// assert!(HarnessRows::default().0.is_empty());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HarnessRows(pub std::collections::BTreeMap<RuleFile, Vec<RowText>>);

/// A rule and a file.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::types::RuleFile;
/// use ryo_relations::domain::relations::types::FileId;
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
///
/// assert!(RuleFile { rule: SelectedRuleOrdinal(0), file: FileId(1) } < RuleFile { rule: SelectedRuleOrdinal(1), file: FileId(0) });
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuleFile {
    /// The rule.
    pub rule: SelectedRuleOrdinal,
    /// The file.
    pub file: FileId,
}

/// A row as owned text: its cells, each a word's text or a number, joined.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::types::RowText;
///
/// assert_eq!(&*RowText("F401 3 9".into()).0, "F401 3 9");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RowText(pub Box<str>);

/// The first place the two engines' rows differ.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::types::{HarnessDifference, RowText, RuleFile};
/// use ryo_relations::domain::relations::types::FileId;
/// use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
///
/// let at = RuleFile { rule: SelectedRuleOrdinal(0), file: FileId(1) };
/// let difference = HarnessDifference { at, old: vec![RowText("a".into())], new: Vec::new() };
/// assert_eq!(difference.old.len(), 1);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HarnessDifference {
    /// The rule and the file.
    pub at: RuleFile,
    /// The old engine's rows there.
    pub old: Vec<RowText>,
    /// The new engine's rows there.
    pub new: Vec<RowText>,
}
