//! The shape of the data this entry does not control: each wire shape is a `<Name>Schema`, its checked conversion into the core's type is an `impl TryFrom<<Name>Schema>`, and each refusal of a conversion is a `<Name>SchemaError`.

use ryo_core::inbound::use_cases::lint::errors::LintError as CoreError;
use ryo_core::inbound::use_cases::lint::types::{
    LintReport as CoreReport, LintRequest as CoreRequest, ParameterCell as CoreCell,
    ParameterTable as CoreTable, ParameterValue as CoreValue, ProviderFault as CoreCode,
    Rule as CoreRule, RuleParameter,
};
use ryo_core::inbound::use_cases::open::errors::OpenError;
use ryo_execution::inbound::use_cases::execution::types::matching::MatchingError;
use ryo_execution::inbound::use_cases::execution::types::matching::MatchingPort as _;
use ryo_language::domain::files::types::FilePath;
use ryo_language::inbound::use_cases::project::types::Repository;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::files::types::Glob as CoreGlob;
use ryo_values::domain::hosted::types::{
    HostColumn as CoreHostColumn, HostColumnName as CoreHostColumnName, HostKind as CoreHostKind,
    HostName as CoreHostName, HostRelation as CoreHost, HostRow as CoreHostRow,
};
use ryo_values::domain::plan_parts::types::{
    FlagState as CoreFlag, LiteralText as CoreText, ParameterName, WholeNumber,
};
use ryo_values::domain::program::types::RefusedKind;
use ryo_values::domain::program::types::RefusedQuery;
use ryo_values::domain::records::types::Unreadable as CoreUnreadable;
use ryo_values::domain::row_texts::types::Cell as RowCell;
use ryo_values::domain::runs::types::{Budget, MemoryBytes, Session, WorkersAsked};

use super::errors::{LintError, ProviderCode};
use super::types::{
    Column, Corpus, Finding, FindingColumn, FindingKey, FlagState, Glob, HostRelation, HostRow,
    Line, LintReport, LintRequest, Message, ParameterValue, Place, Port, ProviderName, RowNumber,
    RuleApplicability, RuleId, SkipReason, Unknown,
};
use crate::domain::capacity::types::MemoryBudget;
use crate::domain::cell::types::{Cell, HostNumber, HostText, Slot};
use crate::domain::file::types::{ReadFailure, RelativePath, Unread, Unreadable};
use crate::domain::host::types::{HostColumn, HostKind, HostRelationName};
use crate::domain::query::errors::ValidateError;
use crate::domain::query::types::{ByteOffset, QuerySpan};
use crate::domain::relation::types::ColumnName;

/// Why a text is not a rule id.
#[expect(
    missing_copy_implementations,
    reason = "adding `Copy` breaks the SDK against its semver baseline (cargo-semver-checks `copy_impl_added`): it comes with the SDK's next version"
)]
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("a rule id is not blank")]
pub struct RuleIdSchemaError;

/// Why a text is not a glob.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("`{glob}` is not a glob: {reason}")]
pub struct GlobSchemaError {
    /// The text.
    pub glob: String,
    /// Why.
    pub reason: String,
}

/// Normalize only the project-context paths whose spelling becomes an observation key.
/// The empty path is the repository root, and is valid only as a search root.
fn project_path(path: &mut RelativePath, root: bool) -> Result<(), LintError> {
    let mut parts = Vec::new();
    let text = path.0.as_ref();
    if text.starts_with('/') || text.chars().any(|part| matches!(part, '\\' | ':' | '\0')) {
        return Err(LintError::ProjectPath { path: path.clone() });
    }
    for part in text.split('/') {
        match part {
            "" | "." => {}
            ".." => return Err(LintError::ProjectPath { path: path.clone() }),
            other => parts.push(other),
        }
    }
    if parts.is_empty() && !root {
        return Err(LintError::ProjectPath { path: path.clone() });
    }
    path.0 = parts.join("/").into();
    Ok(())
}

/// Validate and normalize project authority before either the Given adapter or core sees it.
pub(super) fn project_context(request: &mut LintRequest) -> Result<(), LintError> {
    if let super::types::ProjectSearchScope::Declared(roots) = &mut request.project_search_scope {
        for root in roots {
            project_path(root, true)?;
        }
    }
    for entry in &mut request.given_project_inventory.entries {
        project_path(&mut entry.path, false)?;
    }
    Ok(())
}

impl TryFrom<&str> for RuleId {
    type Error = RuleIdSchemaError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        if text.trim().is_empty() {
            return Err(RuleIdSchemaError);
        }
        Ok(Self(text.into()))
    }
}

impl TryFrom<&str> for Glob {
    type Error = GlobSchemaError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        // A glob is the text the matching reads: the schema keeps the text, and the run makes it ready among its rule's globs.
        match super::composition::build_matching().parse(&CoreGlob(text.into())) {
            Ok(()) => Ok(Self(text.into())),
            Err(MatchingError::BadGlob { glob, reason }) => Err(GlobSchemaError { glob, reason }),
        }
    }
}

/// A core failure of a lint, offered to the conversion.
struct LintErrorSchema(CoreError);

/// A core failure of opening, offered to the conversion.
struct OpenErrorSchema(OpenError);

/// A parameter value of the caller's, offered to the conversion into the core's.
struct ParameterValueSchema(ParameterValue);

/// A request, as the caller wrote it.
struct LintRequestSchema(LintRequest);

/// What a lint answered and what its findings were kept as.
struct LintReportSchema {
    /// Where the rows the run wrote were kept.
    collector: ryo_core::outbound::row_sink::types::Collector,
    /// The files nothing read.
    report: CoreReport,
}

pub(super) fn given_project_inventory(
    inventory: super::types::GivenProjectInventory,
) -> ryo_language::domain::project_context::types::GivenProjectInventory {
    ryo_language::domain::project_context::types::GivenProjectInventory {
            entries: inventory.entries.into_iter().map(|entry| ryo_language::domain::project_context::types::EntryObservation {
                path: FilePath(entry.path.0),
                state: match entry.state {
                    super::types::ProjectEntryState::File => ryo_language::domain::project_context::types::ObservedEntryState::File,
                    super::types::ProjectEntryState::Directory => ryo_language::domain::project_context::types::ObservedEntryState::Directory,
                    super::types::ProjectEntryState::Other => ryo_language::domain::project_context::types::ObservedEntryState::Other,
                    super::types::ProjectEntryState::Missing => ryo_language::domain::project_context::types::ObservedEntryState::Missing,
                    super::types::ProjectEntryState::Unknown => ryo_language::domain::project_context::types::ObservedEntryState::Unknown,
                },
            }).collect(),
            completeness: match inventory.completeness {
                super::types::ProjectInventoryCompleteness::Partial => ryo_language::domain::project_context::types::ProjectInventoryCompleteness::Partial,
                super::types::ProjectInventoryCompleteness::CompleteWithinDeclaredScope => ryo_language::domain::project_context::types::ProjectInventoryCompleteness::CompleteWithinDeclaredScope,
            },
        }
}

impl From<ParameterValueSchema> for CoreValue {
    fn from(schema: ParameterValueSchema) -> Self {
        match schema.0 {
            ParameterValue::Number(number) => Self::Number(WholeNumber(number.0)),
            ParameterValue::Flag(flag) => Self::Flag(match flag {
                FlagState::On => CoreFlag::Raised,
                FlagState::Off => CoreFlag::Lowered,
            }),
            ParameterValue::Text(text) => Self::Text(CoreText(text.0)),
            ParameterValue::Numbers(numbers) => {
                Self::Numbers(numbers.0.into_iter().map(WholeNumber).collect())
            }
            ParameterValue::Texts(texts) => {
                Self::Texts(texts.into_iter().map(|text| CoreText(text.0)).collect())
            }
            ParameterValue::TextSets(sets) => Self::TextSets(
                sets.into_iter()
                    .map(|set| set.into_iter().map(|text| CoreText(text.0)).collect())
                    .collect(),
            ),
            ParameterValue::Tables(tables) => Self::Tables(
                tables
                    .into_iter()
                    .map(|table| {
                        CoreTable(
                            table
                                .0
                                .into_iter()
                                .map(|cell| CoreCell {
                                    key: CoreText(cell.key.0),
                                    value: CoreText(cell.value.0),
                                })
                                .collect(),
                        )
                    })
                    .collect(),
            ),
        }
    }
}

impl TryFrom<LintRequestSchema> for CoreRequest {
    type Error = LintError;

    fn try_from(schema: LintRequestSchema) -> Result<Self, Self::Error> {
        let request = schema.0;
        let root = request
            .root
            .to_str()
            .ok_or_else(|| LintError::PathNotText {
                path: request.root.clone(),
            })?;
        let globs = |globs: Vec<Glob>| {
            globs
                .into_iter()
                .map(|glob| CoreGlob(glob.as_ref().into()))
                .collect::<Vec<_>>()
        };
        Ok(Self {
            source_language: match request.source_language {
                crate::domain::language::types::SourceLanguage::Recognized => None,
                crate::domain::language::types::SourceLanguage::Named(name) => Some(
                    ryo_values::domain::descriptions::types::LanguageName(name.0),
                ),
            },
            session: Session {
                root: RootPath(root.into()),
                repository: match request.corpus {
                    Corpus::Given(_) => Repository::Given,
                    Corpus::Walked | Corpus::Listed(_) => Repository::Disk,
                },
                search_scope: match request.project_search_scope {
                    super::types::ProjectSearchScope::Undeclared => {
                        ryo_language::domain::project_context::types::ProjectSearchScope::Undeclared
                    }
                    super::types::ProjectSearchScope::Declared(roots) => {
                        ryo_language::domain::project_context::types::ProjectSearchScope::Declared(
                            roots.into_iter().map(|root| FilePath(root.0)).collect(),
                        )
                    }
                },
                given_inventory: given_project_inventory(request.given_project_inventory),
            },
            files: request
                .files
                .into_iter()
                .map(|glob| FilePath(glob.as_ref().into()))
                .collect(),
            hosted: host_relations(request.hosted)?,
            answer: ryo_values::domain::runs::types::AnswerShape::Findings,
            rules: request
                .rules
                .into_iter()
                .map(|(id, rule)| CoreRule {
                    observations: observation_bindings(rule.observations),
                    id: ryo_values::domain::runs::types::RuleId(id.as_ref().into()),
                    query: rule.query.into_bundle(),
                    applicability: match rule.applicability {
                        RuleApplicability::Always => {
                            ryo_values::domain::runs::types::RuleApplicability::Always
                        }
                        RuleApplicability::AcceptedSyntax => {
                            ryo_values::domain::runs::types::RuleApplicability::AcceptedSyntax
                        }
                    },
                    parameters: rule
                        .parameters
                        .into_iter()
                        .map(|(name, value)| RuleParameter {
                            name: ParameterName(name.0),
                            value: ParameterValueSchema(value).into(),
                        })
                        .collect(),
                    include: globs(rule.include),
                    exclude: globs(rule.exclude),
                })
                .collect(),
        })
    }
}

impl TryFrom<LintReportSchema> for LintReport {
    type Error = LintError;

    fn try_from(schema: LintReportSchema) -> Result<Self, Self::Error> {
        let LintReportSchema { collector, report } = schema;
        let collected = std::mem::take(
            &mut *collector
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        );
        let findings = collected
            .rows
            .into_iter()
            .map(|row| finding_of(&row.0))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            findings,
            dependencies: report.dependencies.into_iter().map(|dependency| super::types::ProjectDependency {
                path: RelativePath(dependency.path.0),
                state: match dependency.state {
                    ryo_core::inbound::use_cases::lint::types::ProjectDependencyState::Present(digest) => super::types::ProjectDependencyState::Present(super::types::DependencyFingerprint(digest.0)),
                    ryo_core::inbound::use_cases::lint::types::ProjectDependencyState::Missing => super::types::ProjectDependencyState::Missing,
                },
            }).collect(),
            unread: report
                .unread
                .into_iter()
                .map(|each| Unread {
                    path: RelativePath(each.path.0),
                    reason: match each.reason {
                        CoreUnreadable::NoLanguage => Unreadable::NoLanguage,
                        CoreUnreadable::Failed(reason) => Unreadable::Failed(ReadFailure(reason.0)),
                    },
                })
                .collect(),
            unknowns: report
                .unknowns
                .into_iter()
                .map(|each| Unknown {
                    rule: RuleId(each.rule.0),
                    path: RelativePath(each.path.0),
                    message: Message(each.message.0),
                })
                .collect(),
            skipped: report
                .skipped
                .into_iter()
                .map(|each| super::types::Skipped {
                    rule: RuleId(each.rule.0),
                    path: RelativePath(each.path.0),
                    reason: match each.reason {
                        ryo_values::domain::runs::types::SkipReason::SyntaxRejected => SkipReason::SyntaxRejected,
                    },
                })
                .collect(),
        })
    }
}

impl From<LintErrorSchema> for LintError {
    fn from(schema: LintErrorSchema) -> Self {
        let io = |what: Port, reason: String| Self::Io { what, reason };
        match schema.0 {
            CoreError::Execution { reason } => Self::Execution { reason },

            CoreError::SourceLanguage { name } => Self::SourceLanguage { name },
            CoreError::ObservationBindings { reason } => Self::ObservationBindings { reason },
            CoreError::ObservationExecution { reason } => Self::ObservationExecution { reason },
            CoreError::Query { id, error } => Self::Rule {
                id: RuleId(id.into()),
                error: compile_error(error),
            },
            CoreError::Rule { id, reason } | CoreError::UnknownKind { id, reason } => {
                Self::Unrunnable {
                    id: RuleId(id.into()),
                    reason,
                }
            }
            CoreError::MissingParameter { id } => Self::MissingParameter {
                id: RuleId(id.into()),
            },
            CoreError::TooLarge { size, budget } => Self::TooLarge { size, budget },
            CoreError::MemoryBudgetExceeded {
                used,
                budget,
                plans,
            } => Self::MemoryBudgetExceeded {
                used,
                budget,
                plans: plans
                    .into_iter()
                    .map(|plan| RuleId(plan.into()))
                    .collect(),
            },
            CoreError::Provider { provider, code } => Self::Provider {
                provider: ProviderName(provider.into()),
                code: match code {
                    CoreCode::SpanOutsideFile => ProviderCode::SpanOutsideFile,
                    CoreCode::NodeOutsideTable => ProviderCode::NodeOutsideTable,
                    CoreCode::NodeOfAnotherFile => ProviderCode::NodeOfAnotherFile,
                    CoreCode::FileNotVisited => ProviderCode::FileNotVisited,
                },
            },
            CoreError::Corpus { reason } => io(Port::Corpus, reason),
            CoreError::Store { reason } => io(Port::Store, reason),
            CoreError::Facts { reason } => io(Port::Facts, reason),
            CoreError::Language { reason } => io(Port::Language, reason),
            CoreError::Memory { reason } => io(Port::Memory, reason),
            CoreError::Scheduler { reason } => io(Port::Scheduler, reason),
            CoreError::Output { reason } => io(Port::Output, reason),
            CoreError::OutputClosed => {
                io(Port::Output, "whoever reads the answer stopped".to_owned())
            }
            // Ids are unique and not blank by construction: these cannot happen, and say so if they do.
            other @ (CoreError::BlankRuleId | CoreError::DuplicateRuleId { .. }) => {
                Self::Unrunnable {
                    id: RuleId(Box::default()),
                    reason: other.to_string(),
                }
            }
        }
    }
}

impl From<OpenErrorSchema> for LintError {
    fn from(schema: OpenErrorSchema) -> Self {
        let (what, reason) = match schema.0 {
            OpenError::Language { reason } => (Port::Language, reason),
            OpenError::Memory { reason } => (Port::Memory, reason),
        };
        Self::Io { what, reason }
    }
}

/// Where a compiler refusal points, as the caller says it.
fn refusal_span(span: ryo_relations::domain::relations::types::Span) -> QuerySpan {
    QuerySpan {
        start: ByteOffset(usize::try_from(span.start.0).unwrap_or(usize::MAX)),
        end: ByteOffset(usize::try_from(span.end.0).unwrap_or(usize::MAX)),
    }
}

/// A rule's query that does not compile, as the caller's failure of validating.
fn compile_error(error: RefusedQuery) -> ValidateError {
    let origin = error.origin;
    let span = refusal_span(error.span);
    let message = error.message.0;
    let error = match error.kind {
        RefusedKind::Syntax => ValidateError::Syntax { span, message },
        RefusedKind::UnknownRelation => ValidateError::UnknownRelation {
            name: message,
            span,
        },
        RefusedKind::Type => ValidateError::Type { span, message },
        RefusedKind::Invalid => ValidateError::Invalid { span, message },
        RefusedKind::Catalog => ValidateError::Catalog { reason: message },
    };
    match origin {
        ryo_values::domain::query_tokens::types::Attribution::Located(origin) => {
            ValidateError::Located {
                origin: crate::domain::query::errors::QueryOrigin::from_internal(origin),
                error: Box::new(error),
            }
        }
        ryo_values::domain::query_tokens::types::Attribution::Unlocated => error,
    }
}

/// The core's request for the caller's.
///
/// # Errors
///
/// `PathNotText` for a root that is not text.
pub(super) fn request(request: LintRequest) -> Result<CoreRequest, LintError> {
    LintRequestSchema(request).try_into()
}

/// The relations the host declared as the core takes them, each row checked against the columns its relation declared.
fn host_relations(relations: Vec<HostRelation>) -> Result<Vec<CoreHost>, LintError> {
    let mut seen: Vec<HostRelationName> = Vec::new();
    let mut hosted = Vec::with_capacity(relations.len());
    for relation in relations {
        if seen.contains(&relation.name) {
            return Err(LintError::DuplicateHostRelation {
                name: relation.name,
            });
        }
        seen.push(relation.name.clone());
        let rows = relation
            .rows
            .into_iter()
            .enumerate()
            .map(|(place, row)| {
                host_row(&relation.name, &relation.columns, RowNumber(place + 1), row)
            })
            .collect::<Result<Vec<_>, _>>()?;
        hosted.push(CoreHost {
            name: CoreHostName(relation.name.0),
            columns: relation
                .columns
                .into_iter()
                .map(|column| CoreHostColumn {
                    name: CoreHostColumnName(column.name.0),
                    kind: match column.kind {
                        HostKind::File => CoreHostKind::File,
                        HostKind::Text => CoreHostKind::Text,
                        HostKind::Number => CoreHostKind::Number,
                    },
                })
                .collect(),
            rows,
        });
    }
    Ok(hosted)
}

/// One row of the relation `relation` the host declared as the core takes it, checked against the columns the relation declared: a
/// cell for every column, each of the column's kind.
fn host_row(
    relation: &HostRelationName,
    columns: &[HostColumn],
    number: RowNumber,
    row: HostRow,
) -> Result<CoreHostRow, LintError> {
    if row.0.len() != columns.len() {
        return Err(LintError::HostRowWidth {
            relation: relation.clone(),
            row: number,
            expected: columns.len(),
            given: row.0.len(),
        });
    }
    columns
        .iter()
        .zip(row.0)
        .map(|(column, cell)| {
            host_cell(column.kind, cell).ok_or_else(|| LintError::HostCellKind {
                relation: relation.clone(),
                row: number,
                column: column.name.clone(),
                expected: column.kind,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(CoreHostRow)
}

/// A cell of a row the host declared as the core takes it: none for a cell that is not of the kind of its column, a span included,
/// which no column the host declares holds.
fn host_cell(kind: HostKind, cell: Cell) -> Option<RowCell> {
    match (kind, cell) {
        (HostKind::File, Cell::File(path)) => Some(RowCell::File(path.0)),
        (HostKind::Text, Cell::Text(text)) => Some(RowCell::Text(text.0)),
        (HostKind::Number, Cell::Number(number)) => Some(RowCell::Number(Integer(number.0))),
        (
            HostKind::File | HostKind::Text | HostKind::Number,
            Cell::File(_) | Cell::Text(_) | Cell::Number(_) | Cell::Span(_),
        ) => None,
    }
}

/// The caller's report for the rows a lint wrote and what it answered.
///
/// # Errors
///
/// `Io` of the output when a row the run wrote is not a finding as the run writes one: a fault of the run, never dropped.
pub(super) fn report(
    collector: ryo_core::outbound::row_sink::types::Collector,
    report: CoreReport,
) -> Result<LintReport, LintError> {
    LintReport::try_from(LintReportSchema { collector, report })
}

/// The failure for a row the run wrote that is not a finding as the run writes one.
fn unreadable_row(what: &str) -> LintError {
    LintError::Io {
        what: Port::Output,
        reason: format!("a finding the run wrote has {what}"),
    }
}

/// The caller's cell for a typed value of a finding's row, as the core holds it: none for a value that reads as nothing.
fn cell_of(cell: Option<&RowCell>) -> Slot {
    cell.map_or(Slot::Nothing, |cell| {
        Slot::Is(match cell {
            RowCell::File(path) => Cell::File(RelativePath(path.clone())),
            RowCell::Text(text) => Cell::Text(HostText(text.clone())),
            RowCell::Number(number) => Cell::Number(HostNumber(number.0)),
            RowCell::Span(span) => Cell::Span(*span),
        })
    })
}

/// A line or a character of a finding, read from its cell: none for a cell with no value or a zero.
fn place_number(cell: Option<&RowCell>, what: &str) -> Result<Option<u32>, LintError> {
    match cell {
        None => Ok(None),
        Some(RowCell::Number(number)) => u32::try_from(number.0)
            .map(|number| (number != 0).then_some(number))
            .map_err(|_out_of_range| unreadable_row(what)),
        Some(RowCell::File(_) | RowCell::Text(_) | RowCell::Span(_)) => Err(unreadable_row(what)),
    }
}

/// A finding, read from the typed cells of the row the run wrote of it: its rule, its subject, how many key values and each, each
/// column as its heading and its value, then its line and character, none when the finding has no place. Every value stays typed.
fn finding_of(cells: &[Option<RowCell>]) -> Result<Finding, LintError> {
    let (column, rest) = cells
        .split_last()
        .ok_or_else(|| unreadable_row("no character"))?;
    let (line, rest) = rest.split_last().ok_or_else(|| unreadable_row("no line"))?;
    let [
        Some(RowCell::Text(rule)),
        subject,
        Some(RowCell::Number(count)),
        rest @ ..,
    ] = rest
    else {
        return Err(unreadable_row("no rule, subject and count of key values"));
    };
    let (key, columns) = rest
        .split_at_checked(usize::try_from(count.0).unwrap_or(usize::MAX))
        .ok_or_else(|| unreadable_row("fewer key values than it counts"))?;
    let columns = columns
        .chunks(2)
        .map(|pair| match pair {
            [Some(RowCell::Text(name)), value] => Ok(FindingColumn {
                name: ColumnName(name.clone()),
                value: cell_of(value.as_ref()),
            }),
            _ => Err(unreadable_row("a column with no heading")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let place = match (
        place_number(line.as_ref(), "a line that is no number")?,
        place_number(column.as_ref(), "a character that is no number")?,
    ) {
        (Some(line), Some(column)) => Place::At {
            line: Line(line),
            column: Column(column),
        },
        _ => Place::Missing,
    };
    Ok(Finding {
        rule: RuleId(rule.clone()),
        subject: cell_of(subject.as_ref()),
        key: FindingKey(key.iter().map(|cell| cell_of(cell.as_ref())).collect()),
        place,
        columns,
    })
}

/// The caller's failure for a core failure of a lint.
pub(super) fn error(error: CoreError) -> LintError {
    LintErrorSchema(error).into()
}

/// The caller's failure for a core failure of opening.
pub(super) fn open_error(error: OpenError) -> LintError {
    OpenErrorSchema(error).into()
}

/// The workers the caller asked for, as the scheduler factory takes them.
pub(super) const fn workers(workers: super::types::Workers) -> WorkersAsked {
    WorkersAsked(workers.0.get())
}

/// The most memory the caller lets the run use, as the memory factory takes it.
pub(super) const fn budget(memory: MemoryBudget) -> Budget {
    match memory {
        MemoryBudget::Bytes(bytes) => Budget::Bytes(MemoryBytes(bytes.get())),
        MemoryBudget::Machine => Budget::Machine,
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

/// Translate edge-validated rule identities without changing producer selection.
fn observation_bindings(
    bindings: Vec<super::types::ObservationBinding>,
) -> Vec<ryo_values::domain::observations::types::ObservationBinding> {
    use ryo_values::domain::observations::types::{ObservationBinding, ObservationProducer};
    bindings
        .into_iter()
        .map(|binding| ObservationBinding {
            name: ryo_values::domain::observations::types::ObservationBindingName(binding.name.0),
            columns: binding.columns.into_iter().map(Into::into).collect(),
            producers: binding
                .producers
                .into_iter()
                .map(|producer| match producer {
                    super::types::ObservationProducer::Output { rule, output } => {
                        ObservationProducer::Output {
                            rule: ryo_values::domain::runs::types::RuleId(rule.as_ref().into()),
                            output: ryo_values::domain::observations::types::ObservationName(
                                output.0,
                            ),
                        }
                    }
                    super::types::ObservationProducer::NoQuery { rule } => {
                        ObservationProducer::NoQuery {
                            rule: ryo_values::domain::runs::types::RuleId(rule.as_ref().into()),
                        }
                    }
                })
                .collect(),
        })
        .collect()
}
