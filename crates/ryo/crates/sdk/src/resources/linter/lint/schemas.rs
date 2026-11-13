// ignore: idiomatic.arch/file-length - the one validation file the entry spec admits: its wire shapes and their conversions stay in the single schemas.rs
//! The shape of the data this entry does not control: each wire shape is a `<Name>Schema`, its checked conversion into the core's type is an `impl TryFrom<<Name>Schema>`, and each refusal of a conversion is a `<Name>SchemaError`.

use ryo_core::inbound::use_cases::lint::types::{
    LintReport as CoreReport, LintRequest as CoreRequest, Rule as CoreRule, RuleParameter,
};
use ryo_execution::outbound::matching::interface::MatchingPort as _;
use ryo_language::domain::files::types::FilePath;
use ryo_language::inbound::use_cases::project::types::Repository;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_matching::inbound::use_cases::matching::errors::MatchingError;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::files::types::Glob as CoreGlob;
use ryo_values::domain::hosted::types::{
    HostName as CoreHostName, HostRelation as CoreHost, HostRow as CoreHostRow,
};
use ryo_values::domain::plan_parts::types::ParameterName;
use ryo_values::domain::row_texts::types::Cell as RowCell;
use ryo_values::domain::runs::types::{Session, WorkersAsked};

use super::errors::LintError;
use super::types::{
    Corpus, Finding, FindingColumn, FindingKey, Glob, HostRelation, HostRow, LintReport,
    LintRequest, Message, Port, RowNumber, RuleApplicability, RuleId, SkipReason, Unknown,
};
use crate::domain::cell::types::{Cell, Slot};
use crate::domain::file::types::{RelativePath, Unread};
use crate::domain::host::types::{HostColumn, HostKind, HostRelationName};
use crate::domain::place::types::{Column, Line, Place};
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
    if let crate::domain::project::types::ProjectSearchScope::Declared(roots) =
        &mut request.project_search_scope
    {
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
            Err(MatchingError::InvalidRegex { reason }) => Err(GlobSchemaError {
                glob: text.into(),
                reason,
            }),
        }
    }
}

/// A request, as the caller wrote it.
struct LintRequestSchema(LintRequest);

/// What a lint answered and what its findings were kept as.
struct LintReportSchema {
    /// Where the rows the run wrote were kept.
    collected: ryo_core::outbound::row_sink::types::Collected,
    /// The files nothing read.
    report: CoreReport,
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
            source_language: request.source_language.into_engine(),
            session: Session {
                root: RootPath(root.into()),
                repository: match request.corpus {
                    Corpus::Given(_) => Repository::Given,
                    Corpus::Walked | Corpus::Listed(_) => Repository::Disk,
                },
                search_scope: request.project_search_scope.into_engine(),
                given_inventory: request.given_project_inventory.into_engine(),
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
                            value: value.into_engine(),
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
        let LintReportSchema { collected, report } = schema;
        let findings = collected
            .rows
            .into_iter()
            .map(|row| finding_of(&row.0))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            findings,
            dependencies: report
                .dependencies
                .into_iter()
                .map(crate::domain::project::types::ProjectDependency::from_engine)
                .collect(),
            unread: report.unread.into_iter().map(Unread::from_engine).collect(),
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
                        ryo_values::domain::runs::types::SkipReason::SyntaxRejected => {
                            SkipReason::SyntaxRejected
                        }
                    },
                })
                .collect(),
        })
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
                .map(HostColumn::into_engine)
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
    collected: ryo_core::outbound::row_sink::types::Collected,
    report: CoreReport,
) -> Result<LintReport, LintError> {
    LintReport::try_from(LintReportSchema { collected, report })
}

/// The failure for a row the run wrote that is not a finding as the run writes one.
fn unreadable_row(what: &str) -> LintError {
    LintError::Io {
        what: Port::Output,
        reason: format!("a finding the run wrote has {what}"),
    }
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
                value: Slot::from_engine(value.as_ref()),
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
        subject: Slot::from_engine(subject.as_ref()),
        key: FindingKey(
            key.iter()
                .map(|cell| Slot::from_engine(cell.as_ref()))
                .collect(),
        ),
        place,
        columns,
    })
}

/// The workers the caller asked for, as the scheduler factory takes them.
pub(super) const fn workers(workers: super::types::Workers) -> WorkersAsked {
    WorkersAsked(workers.0.get())
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
            columns: binding
                .columns
                .into_iter()
                .map(crate::domain::observation::types::ObservationColumn::into_engine)
                .collect(),
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
