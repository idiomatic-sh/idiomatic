//! The types of the `warning` part: what a warning names besides the engine's counts, and the warning as it is printed.

use ryo_language::domain::files::types::SourcePath;
use ryo_values::domain::query_tokens::types::{
    QueryBundle, QuerySourceId, QuerySpan, StepSpelling,
};
use ryo_values::domain::runs::types::RuleId;

/// Where a statement of a rule's query is written, as the query language says it of a step of the rule's plan: the document
/// that writes it, the name its `let` gives, its bytes in that document, and the names of what it reads, in its order (the
/// two sides of a join). The line a warning shows is counted in the document's text, up to where the bytes start.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::warning::types::WrittenStatement;
/// use ryo_values::domain::query_tokens::types::{ByteOffset, QuerySourceId, QuerySpan, StepSpelling};
///
/// let written = WrittenStatement {
///     document: QuerySourceId("rule.ryo".into()),
///     statement: StepSpelling("pairs".into()),
///     span: QuerySpan { start: ByteOffset(40), end: ByteOffset(72) },
///     inputs: vec![StepSpelling("words".into()), StepSpelling("words".into())],
/// };
/// assert_eq!(&*written.statement, "pairs");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrittenStatement {
    /// The document that writes the statement: a module's, or the query's own.
    pub document: QuerySourceId,
    /// The name the statement's `let` gives.
    pub statement: StepSpelling,
    /// Where it is written in its document, in bytes.
    pub span: QuerySpan,
    /// The names of what it reads, in its order.
    pub inputs: Vec<StepSpelling>,
}

/// The statement a warning names: the one the query language says the noticed step was written as, with the rule's query
/// (whose documents' text the statement's line is counted in); or, when it says none, the step of the rule's plan the
/// engine's notice names.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::warning::types::NoticedStatement;
///
/// assert!(matches!(NoticedStatement::PlanStep, NoticedStatement::PlanStep));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticedStatement<'a> {
    /// The statement, where it is written, and the rule's query: its own document and its modules.
    Written(&'a WrittenStatement, &'a QueryBundle),
    /// No statement is said: the warning names the plan step.
    PlanStep,
}

/// What a warning names besides the engine's counts: the rule, the file and the statement.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::warning::types::{NoticedStatement, SlowRule};
/// use ryo_language::domain::files::types::SourcePath;
/// use ryo_values::domain::runs::types::RuleId;
///
/// let rule = RuleId("rule".into());
/// let slow = SlowRule { rule: &rule, file: SourcePath("b.walk"), statement: NoticedStatement::PlanStep };
/// assert_eq!(&*slow.file, "b.walk");
/// ```
#[derive(Clone, Copy, Debug)]
pub struct SlowRule<'a> {
    /// The rule's id.
    pub rule: &'a RuleId,
    /// The file's path.
    pub file: SourcePath<'a>,
    /// The statement.
    pub statement: NoticedStatement<'a>,
}

/// A warning as it is printed and listed: its lines, the first naming the rule and the file, the others indented under it.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::warning::types::WarningText;
///
/// assert!(WarningText("warning: rule `rule` is slow in `b.walk`".into()).0.starts_with("warning: "));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WarningText(pub Box<str>);

/// A form of a warning: a line, or a piece of one, with its named places.
#[derive(Clone, Copy, Debug)]
pub(crate) struct WarningForm<'a>(pub(crate) &'a str);

/// What fills a named place of a form.
#[derive(Clone, Copy)]
pub(crate) struct PlaceValue<'a> {
    /// The place, as a form writes it.
    pub(crate) place: WarningForm<'a>,
    /// What is written there.
    pub(crate) value: &'a dyn std::fmt::Display,
}
