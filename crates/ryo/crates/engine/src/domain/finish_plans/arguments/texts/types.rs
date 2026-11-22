//! The types of the `texts` part: what the operators over spans and text: `span-bounds`, `span-between`, `replace`, `source-matches`, `capture`, `fits-rendered`, `render-syntax` are given. Plain data.

use crate::domain::finish_plans::arguments::types::StepNumber;
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::plan_parts::types::{LiteralText, WholeNumber};

/// `span-bounds`: every input row with the start and the end of one of its spans.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_engine::domain::finish_plans::arguments::texts::types::SpanBoundsArguments;
/// use ryo_values::domain::descriptions::types::ColumnName;
///
/// let name = |text: &str| ColumnName(text.into());
/// let bounds = SpanBoundsArguments { from: StepNumber(0), span: name("span"), start_output: name("start"), end_output: name("end") };
/// assert_eq!(&*bounds.span.0, "span");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SpanBoundsArguments {
    /// The rows.
    pub from: StepNumber,
    /// The span column.
    pub span: ColumnName,
    /// The column its start is written to.
    pub start_output: ColumnName,
    /// The column its end is written to.
    pub end_output: ColumnName,
}

/// `span-between`: every input row with the span from the start of one span to the end of another.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_engine::domain::finish_plans::arguments::texts::types::SpanBetweenArguments;
/// use ryo_values::domain::descriptions::types::ColumnName;
///
/// let name = |text: &str| ColumnName(text.into());
/// let between = SpanBetweenArguments { from: StepNumber(0), start: name("a"), end: name("b"), output: name("span") };
/// assert_eq!(&*between.output.0, "span");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SpanBetweenArguments {
    /// The rows.
    pub from: StepNumber,
    /// The span whose start begins it.
    pub start: ColumnName,
    /// The span whose end ends it.
    pub end: ColumnName,
    /// The column it is written to.
    pub output: ColumnName,
}

/// `replace`: every input row with a text column's matches of a pattern replaced.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_engine::domain::finish_plans::arguments::texts::types::ReplaceArguments;
/// use ryo_values::domain::descriptions::types::ColumnName;
/// use ryo_values::domain::plan_parts::types::LiteralText;
///
/// let replace = ReplaceArguments {
///     from: StepNumber(0), text: ColumnName("text".into()), pattern: LiteralText("_".into()),
///     replacement: LiteralText(String::new().into()), output: ColumnName("bare".into()),
/// };
/// assert_eq!(&*replace.pattern.0, "_");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ReplaceArguments {
    /// The rows.
    pub from: StepNumber,
    /// The text column.
    pub text: ColumnName,
    /// The pattern.
    pub pattern: LiteralText,
    /// What each match is replaced with.
    pub replacement: LiteralText,
    /// The column the result is written to.
    pub output: ColumnName,
}

/// `source-matches`: every match of a pattern in the source text of a row's span, with the columns the query asks for.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_engine::domain::finish_plans::arguments::texts::types::{OptionalColumn, SourceMatchesArguments};
/// use ryo_values::domain::descriptions::types::ColumnName;
/// use ryo_values::domain::plan_parts::types::LiteralText;
///
/// let matches = SourceMatchesArguments {
///     from: StepNumber(0), span: ColumnName("span".into()), base_line: OptionalColumn::Unwritten,
///     pattern: LiteralText("#".into()), output: ColumnName("match".into()), line_output: OptionalColumn::Unwritten,
///     capture: OptionalColumn::Unwritten, capture_output: OptionalColumn::Unwritten,
///     capture_text_output: OptionalColumn::Unwritten, capture_line_output: OptionalColumn::Unwritten,
/// };
/// assert_eq!(matches.capture, OptionalColumn::Unwritten);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SourceMatchesArguments {
    /// The rows.
    pub from: StepNumber,
    /// The span searched.
    pub span: ColumnName,
    /// The column of the span's first line.
    pub base_line: OptionalColumn,
    /// The pattern.
    pub pattern: LiteralText,
    /// The column a match's span is written to.
    pub output: ColumnName,
    /// The column a match's line is written to.
    pub line_output: OptionalColumn,
    /// The capture group read.
    pub capture: OptionalColumn,
    /// The column the capture's span is written to.
    pub capture_output: OptionalColumn,
    /// The column the capture's text is written to.
    pub capture_text_output: OptionalColumn,
    /// The column the capture's line is written to.
    pub capture_line_output: OptionalColumn,
}

/// A name an operator may be given or not: a column, or a capture group.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::texts::types::OptionalColumn;
/// use ryo_values::domain::descriptions::types::ColumnName;
///
/// assert_ne!(OptionalColumn::Written(ColumnName("line".into())), OptionalColumn::Unwritten);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum OptionalColumn {
    /// The query names none.
    Unwritten,
    /// The query names this one.
    Written(ColumnName),
}

/// `capture`: every input row whose text column a pattern matches, with its named captures as columns.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_engine::domain::finish_plans::arguments::texts::types::CaptureArguments;
/// use ryo_values::domain::descriptions::types::ColumnName;
/// use ryo_values::domain::plan_parts::types::LiteralText;
///
/// let capture = CaptureArguments {
///     from: StepNumber(0), path: ColumnName("path".into()), pattern: LiteralText("{name}".into()), captures: Vec::new(),
/// };
/// assert!(capture.captures.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CaptureArguments {
    /// The rows.
    pub from: StepNumber,
    /// The text column matched.
    pub path: ColumnName,
    /// The pattern, with its captures.
    pub pattern: LiteralText,
    /// The captures kept as columns.
    pub captures: Vec<ColumnName>,
}

/// `fits-rendered`: the rows whose texts, rendered with literals between them, fit on one line.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_engine::domain::finish_plans::arguments::texts::types::FitsRenderedArguments;
/// use ryo_values::domain::descriptions::types::ColumnName;
/// use ryo_values::domain::plan_parts::types::WholeNumber;
///
/// let fits = FitsRenderedArguments {
///     from: StepNumber(0), texts: Vec::new(), literals: Vec::new(), span: ColumnName("span".into()),
///     max_line: WholeNumber(88), tab_width: WholeNumber(8),
/// };
/// assert_eq!(fits.max_line, WholeNumber(88));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FitsRenderedArguments {
    /// The rows.
    pub from: StepNumber,
    /// The text columns rendered.
    pub texts: Vec<ColumnName>,
    /// The literals between them.
    pub literals: Vec<LiteralText>,
    /// The span whose line is measured.
    pub span: ColumnName,
    /// The longest line.
    pub max_line: WholeNumber,
    /// The width of a tab.
    pub tab_width: WholeNumber,
}

/// `render-syntax`: every input row with the source text of a node rendered for a context.
///
/// # Examples
///
/// ```
/// use ryo_engine::domain::finish_plans::arguments::types::StepNumber;
/// use ryo_engine::domain::finish_plans::arguments::texts::types::RenderSyntaxArguments;
/// use ryo_values::domain::descriptions::types::ColumnName;
/// use ryo_values::domain::plan_parts::types::LiteralText;
///
/// let render = RenderSyntaxArguments {
///     from: StepNumber(0), node: ColumnName("id".into()), context: LiteralText("positional-argument".into()),
///     output: ColumnName("text".into()),
/// };
/// assert_eq!(&*render.node.0, "id");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RenderSyntaxArguments {
    /// The rows.
    pub from: StepNumber,
    /// The node column.
    pub node: ColumnName,
    /// The context it is rendered for.
    pub context: LiteralText,
    /// The column the text is written to.
    pub output: ColumnName,
}
