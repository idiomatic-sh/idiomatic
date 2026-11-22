//! The types of the `written_sites` concept: where a step of a compiled plan was written. Plain data, public fields; the
//! one record a step carries is no part of what the step does, so it equals every other and hashes to nothing.

use crate::domain::query_tokens::types::{QuerySourceId, QueryTraceSite, StepSpelling};
use crate::domain::runs::types::RuleId;
use std::sync::Arc;

/// The statement a step was written as, as the compiler recorded it while it lowered that statement: a rewrite that makes
/// a step from another carries the other's. `Unrecorded` for a step no statement wrote (a plan built by hand).
///
/// It is no part of what the step does: two steps that do the same are equal and hash alike whatever their records, so a
/// plan's equality, its digests and every sharing they decide are as they would be without it.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::written_sites::types::{StepWritten, WrittenSite};
/// use ryo_values::domain::query_tokens::types::{QuerySourceId, StepSpelling};
/// use std::sync::Arc;
///
/// let site = WrittenSite {
///     document: QuerySourceId("library/summarize".into()),
///     statement: StepSpelling("grouped".into()),
///     trace: Arc::from([]),
/// };
/// assert_eq!(StepWritten::Written(Arc::new(site)), StepWritten::default());
/// ```
#[derive(Clone, Debug, Default)]
pub enum StepWritten {
    /// The statement it was written as.
    Written(Arc<WrittenSite>),
    /// No statement wrote it.
    #[default]
    Unrecorded,
}

impl PartialEq for StepWritten {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for StepWritten {}

impl std::hash::Hash for StepWritten {
    fn hash<H: std::hash::Hasher>(&self, _state: &mut H) {}
}

/// Where a statement is written: the document that writes it, the name its `let` gives there, and the import and call
/// sites from the rule's own query to that document (none for a statement of the rule's own query).
///
/// # Examples
///
/// ```
/// use ryo_values::domain::written_sites::types::WrittenSite;
/// use ryo_values::domain::query_tokens::types::{QuerySourceId, StepSpelling};
/// use std::sync::Arc;
///
/// let site = WrittenSite {
///     document: QuerySourceId("<query>".into()),
///     statement: StepSpelling("candidates".into()),
///     trace: Arc::from([]),
/// };
/// assert!(site.trace.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WrittenSite {
    /// The document that writes the statement: a module's, or the rule's own query's.
    pub document: QuerySourceId,
    /// The name the statement's `let` gives in that document.
    pub statement: StepSpelling,
    /// The import and call sites from the rule's own query to the document, outermost first.
    pub trace: Arc<[QueryTraceSite]>,
}

/// Where each step of each selected rule's plan was written, in rule order: what a measuring run's ledger names its units by.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::written_sites::types::RulesWritten;
///
/// assert!(RulesWritten::default().0.is_empty());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RulesWritten(pub Vec<RuleWritten>);

/// Where each step of one rule's plan, as the engine received it, was written, by step; with the rule's id and the identity
/// of its own query, by which a statement of the rule's own query is named after the rule.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::written_sites::types::RuleWritten;
/// use ryo_values::domain::query_tokens::types::QuerySourceId;
/// use ryo_values::domain::runs::types::RuleId;
///
/// let rule = RuleWritten { id: RuleId::default(), root: QuerySourceId("<query>".into()), steps: Vec::new() };
/// assert!(rule.steps.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleWritten {
    /// The rule.
    pub id: RuleId,
    /// The identity of the rule's own query.
    pub root: QuerySourceId,
    /// Where each step was written, by step.
    pub steps: Vec<StepWritten>,
}
