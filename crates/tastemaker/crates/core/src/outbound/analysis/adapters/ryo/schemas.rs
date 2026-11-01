// ignore: idiomatic.arch/file-length - the adapter spec places one schemas.rs per adapter: every conversion between the core's types and the SDK's is in it, the request, the placed rows, the params, the report with its typed cells and the errors, each beside the others it shares the SDK's types with.
//! The shape of the data this adapter does not control: the core's request becomes the SDK's, and the SDK's report and
//! errors become the core's. Each refusal of a conversion is the port's own error.

use std::collections::BTreeMap;

use ryo::linter::{
    CaseSource, Cell, Corpus as Reads, FlagState, Glob, HostColumn, HostColumnName, HostKind,
    HostNumber, HostRelation, HostRelationName, HostRow, HostText, LintError, LintReport,
    LintRequest, ParameterCell, ParameterName, ParameterNumber, ParameterNumbers, ParameterTable,
    ParameterText, ParameterValue, RelativePath, Rule, RuleApplicability, RuleId, Slot, SourceText,
};

use super::constants::{PLACED, PLACED_COLUMNS, RYO_STORE, SPAN_TEXT};
use crate::domain::applications::types::{
    AnswerCell, CellText, CellValue, GlobText, LintAnswer, LintRuleId, LintUnknown, PlacedRow,
    PlacingSpec,
};
use crate::domain::catalog::types::{
    CatalogColumn, CatalogEntry, CatalogField, CatalogLanguage, CatalogName, CatalogText,
    OperatorPage, Reading, RelationPage, Requirement,
};
use crate::domain::comments::types::ByteSpan;
use crate::domain::definitions::types::ColumnName;
use crate::domain::entries::types::Value;
use crate::domain::findings::types::{ByteOffset, MessageText};
use crate::domain::paths::types::RepoPath;
use crate::domain::scalars::types::FlagValue;
use crate::outbound::analysis::errors::AnalysisError;
use crate::outbound::analysis::types::{AnalysisReport, AnalysisRequest, Corpus, Hosted};

/// The port's engine error carrying `reason`.
fn engine(reason: impl std::fmt::Display) -> AnalysisError {
    AnalysisError::Engine {
        reason: reason.to_string(),
    }
}

/// The declaration of `tastemaker.placed`: its name and columns, the first a file.
pub(super) fn placed_columns() -> (HostRelationName, Vec<HostColumn>) {
    let columns = PLACED_COLUMNS
        .iter()
        .enumerate()
        .map(|(index, name)| HostColumn {
            name: HostColumnName((*name).into()),
            kind: if index == 0 {
                HostKind::File
            } else {
                HostKind::Text
            },
        })
        .collect();
    (HostRelationName(PLACED.into()), columns)
}

/// The `tastemaker.placed` relation declared with `rows`.
fn placed(rows: &[PlacedRow]) -> HostRelation {
    let (name, columns) = placed_columns();
    HostRelation {
        name,
        columns,
        rows: rows
            .iter()
            .map(|row| {
                let text = |value: &str| Cell::Text(HostText(value.into()));
                HostRow(vec![
                    Cell::File(RelativePath(row.path.0.clone())),
                    text(&row.entry.0),
                    text(&row.package.0),
                    text(&row.template.0),
                    text(&row.version.0),
                    // ryo's cells hold no absence: a row of no spec carries the empty text in its `spec` column.
                    text(match &row.spec {
                        PlacingSpec::Workspace => "",
                        PlacingSpec::Instance(spec) => &spec.0,
                    }),
                ])
            })
            .collect(),
    }
}

/// A relation tm hosts named `name`, of the columns `columns` (each a name and its kind) and the rows `rows`.
fn hosted(name: &str, columns: &[(&str, HostKind)], rows: Vec<HostRow>) -> HostRelation {
    HostRelation {
        name: HostRelationName(name.into()),
        columns: columns
            .iter()
            .map(|(column, kind)| HostColumn {
                name: HostColumnName((*column).into()),
                kind: *kind,
            })
            .collect(),
        rows,
    }
}

/// A text cell.
fn text_cell(text: &str) -> Cell {
    Cell::Text(HostText(text.into()))
}

/// A number cell.
fn number_cell(number: u32) -> Cell {
    Cell::Number(HostNumber(i64::from(number)))
}

/// The relations of a verdict call: what the scenarios expected and found, what their `only` names, the rules they
/// prove and every rule and check their checks ran.
fn verdict(rows: &crate::domain::scenario_judging::types::VerdictRows) -> Vec<HostRelation> {
    use crate::domain::scenario_judging::constants::{
        EVALUATED, EXPECTED, FINDING, FINDING_PATH, FOUND, FULL, LINE, ONLY, ONLY_RULE, OWN, PLACE,
        RULE, SCENARIO,
    };
    let (number, text) = (HostKind::Number, HostKind::Text);
    let rules = |listed: &[crate::domain::scenario_judging::types::RuleRow]| {
        listed
            .iter()
            .map(|row| HostRow(vec![number_cell(row.scenario.0), text_cell(&row.rule.0)]))
            .collect::<Vec<_>>()
    };
    vec![
        hosted(
            EXPECTED,
            &[
                (SCENARIO, number),
                (FINDING, number),
                (RULE, text),
                (FINDING_PATH, text),
                (LINE, number),
            ],
            rows.expected
                .iter()
                .map(|row| {
                    HostRow(vec![
                        number_cell(row.scenario.0),
                        number_cell(row.finding.0),
                        text_cell(&row.rule.0),
                        text_cell(&row.path.0),
                        number_cell(row.line.0),
                    ])
                })
                .collect(),
        ),
        hosted(
            FOUND,
            &[
                (SCENARIO, number),
                (FINDING, number),
                (RULE, text),
                (FULL, text),
                (FINDING_PATH, text),
                (LINE, number),
            ],
            rows.found
                .iter()
                .map(|row| {
                    HostRow(vec![
                        number_cell(row.scenario.0),
                        number_cell(row.finding.0),
                        text_cell(&row.rule.0),
                        text_cell(&row.full.0),
                        text_cell(&row.path.0),
                        number_cell(row.line.0),
                    ])
                })
                .collect(),
        ),
        hosted(
            ONLY,
            &[(SCENARIO, number), (PLACE, number), (ONLY_RULE, text)],
            rows.only
                .iter()
                .map(|row| {
                    HostRow(vec![
                        number_cell(row.scenario.0),
                        number_cell(row.place.0),
                        text_cell(&row.only.0),
                    ])
                })
                .collect(),
        ),
        hosted(OWN, &[(SCENARIO, number), (RULE, text)], rules(&rows.own)),
        hosted(
            EVALUATED,
            &[(SCENARIO, number), (RULE, text)],
            rules(&rows.evaluated),
        ),
    ]
}

/// The relations of a count call: the counts and pairs a check left waiting, each with its bounds, and what each
/// condition they read answered; a flag or an answer as the number the count queries read.
fn counts(rows: &crate::domain::judging::types::CountRows) -> Vec<HostRelation> {
    use crate::domain::judging::constants::{
        CONDITION, CONDITION_ANSWER, ELSE_MAX, ELSE_MIN, ENTRY_NUMBER, FOUND, HOLDS, HOLDS_NO,
        HOLDS_UNANSWERED, HOLDS_YES, IDENTITY, KIND_COUNT, KIND_PAIR, MAX, MIN, PAIR_FLAG,
        SCOPE_INDEX, TEMPLATE_ENTRY, THEN_MAX, THEN_MIN,
    };
    use crate::domain::judging::types::EntryKind;
    use crate::domain::scopes::types::{Answer, ScopeIndex};
    let (number, text) = (HostKind::Number, HostKind::Text);
    let scope = |at: ScopeIndex| number_cell(u32::try_from(at.0).unwrap_or(u32::MAX));
    vec![
        hosted(
            TEMPLATE_ENTRY,
            &[
                (ENTRY_NUMBER, number),
                (SCOPE_INDEX, number),
                (IDENTITY, text),
                (PAIR_FLAG, number),
                (FOUND, number),
                (CONDITION, text),
                (MIN, number),
                (MAX, number),
                (THEN_MIN, number),
                (THEN_MAX, number),
                (ELSE_MIN, number),
                (ELSE_MAX, number),
            ],
            rows.entries
                .iter()
                .map(|row| {
                    HostRow(vec![
                        number_cell(row.entry.0),
                        scope(row.scope),
                        text_cell(&row.identity.0),
                        number_cell(match row.kind {
                            EntryKind::Count => KIND_COUNT,
                            EntryKind::Pair => KIND_PAIR,
                        }),
                        number_cell(row.found.0),
                        text_cell(&row.condition.0),
                        number_cell(row.bounds.min.0),
                        number_cell(row.bounds.max.0),
                        number_cell(row.then.min.0),
                        number_cell(row.then.max.0),
                        number_cell(row.otherwise.min.0),
                        number_cell(row.otherwise.max.0),
                    ])
                })
                .collect(),
        ),
        hosted(
            CONDITION_ANSWER,
            &[(SCOPE_INDEX, number), (CONDITION, text), (HOLDS, number)],
            rows.conditions
                .iter()
                .map(|row| {
                    HostRow(vec![
                        scope(row.scope),
                        text_cell(&row.condition.0),
                        number_cell(match row.holds {
                            Answer::Holds => HOLDS_YES,
                            Answer::Fails => HOLDS_NO,
                            Answer::Unanswered => HOLDS_UNANSWERED,
                        }),
                    ])
                })
                .collect(),
        ),
    ]
    .into_iter()
    .chain(closings(&rows.closings))
    .collect()
}

/// The relations of the closed query: the closed directories and the files beneath them, the files templates place and the
/// `tastemaker.toml` files, the instances nested in the directories and the directories their scopes placed files in.
fn closings(rows: &crate::domain::closing::types::Closings) -> Vec<HostRelation> {
    use crate::domain::closing::constants::{
        CLOSED_DIR, CLOSED_UNDER, DEPTH, GROUP, HOME, LAST, NESTED_DIR, OCCUPIED, OWN_FILE, OWNER,
        PATH, PLACED_FILE, PREFIX, STRAY,
    };
    let (number, text) = (HostKind::Number, HostKind::Text);
    let scope = |at: crate::domain::scopes::types::ScopeIndex| {
        number_cell(u32::try_from(at.0).unwrap_or(u32::MAX))
    };
    vec![
        hosted(
            CLOSED_DIR,
            &[(GROUP, number), (PREFIX, text), (LAST, number)],
            rows.directories
                .iter()
                .map(|row| {
                    HostRow(vec![
                        number_cell(row.group.0),
                        text_cell(&crate::domain::closing::service::prefix_of(&row.directory).0),
                        row.owners
                            .last()
                            .map_or_else(|| number_cell(0), |owner| scope(owner.scope)),
                    ])
                })
                .collect(),
        ),
        hosted(
            CLOSED_UNDER,
            &[(STRAY, number), (GROUP, number), (PATH, text), (HOME, text)],
            rows.candidates
                .iter()
                .map(|row| {
                    HostRow(vec![
                        number_cell(row.stray.0),
                        number_cell(row.group.0),
                        text_cell(&row.path.0),
                        text_cell(&row.home.0),
                    ])
                })
                .collect(),
        ),
        hosted(
            PLACED_FILE,
            &[(PATH, text)],
            rows.placed
                .iter()
                .map(|path| HostRow(vec![text_cell(&path.0)]))
                .collect(),
        ),
        hosted(
            OWN_FILE,
            &[(PATH, text)],
            rows.own
                .iter()
                .map(|path| HostRow(vec![text_cell(&path.0)]))
                .collect(),
        ),
        hosted(
            NESTED_DIR,
            &[(GROUP, number), (PREFIX, text)],
            rows.nested
                .iter()
                .map(|row| HostRow(vec![number_cell(row.group.0), text_cell(&row.prefix.0)]))
                .collect(),
        ),
        hosted(
            OCCUPIED,
            &[
                (GROUP, number),
                (OWNER, number),
                (PREFIX, text),
                (DEPTH, number),
            ],
            rows.occupied
                .iter()
                .map(|row| {
                    HostRow(vec![
                        number_cell(row.group.0),
                        scope(row.owner),
                        text_cell(&row.prefix.0),
                        number_cell(row.depth.0),
                    ])
                })
                .collect(),
        ),
    ]
}

/// A param's value as the SDK binds it: a list of whole numbers is numbers, a list of lists is sets of text, any other
/// list is text.
fn parameter(value: &Value) -> ParameterValue {
    match value {
        Value::Whole(number) => ParameterValue::Number(ParameterNumber(number.0)),
        Value::Flag(FlagValue::Yes) => ParameterValue::Flag(FlagState::On),
        Value::Flag(FlagValue::No) => ParameterValue::Flag(FlagState::Off),
        Value::List(items)
            if !items.is_empty() && items.iter().all(|item| matches!(item, Value::Whole(_))) =>
        {
            ParameterValue::Numbers(ParameterNumbers(
                items
                    .iter()
                    .filter_map(|item| {
                        if let Value::Whole(number) = item {
                            Some(number.0)
                        } else {
                            None
                        }
                    })
                    .collect(),
            ))
        }
        // A list of lists is a list of sets, in order: `[["app"], ["domain"]]`.
        Value::List(items)
            if !items.is_empty() && items.iter().all(|item| matches!(item, Value::List(_))) =>
        {
            ParameterValue::TextSets(
                items
                    .iter()
                    .filter_map(|item| {
                        if let Value::List(set) = item {
                            Some(set)
                        } else {
                            None
                        }
                    })
                    .map(|set| set.iter().map(parameter_text).collect())
                    .collect(),
            )
        }
        // A list of tables is a list of rows, each cell under its key: `[{ resource = "languages", path = "a::B" }]`.
        Value::List(items)
            if !items.is_empty() && items.iter().all(|item| matches!(item, Value::Table(_))) =>
        {
            ParameterValue::Tables(
                items
                    .iter()
                    .filter_map(|item| {
                        if let Value::Table(table) = item {
                            Some(table)
                        } else {
                            None
                        }
                    })
                    .map(|table| {
                        ParameterTable(
                            table
                                .iter()
                                .map(|(column, cell)| ParameterCell {
                                    key: ParameterText(column.0.clone()),
                                    value: parameter_text(cell),
                                })
                                .collect(),
                        )
                    })
                    .collect(),
            )
        }
        Value::List(items) => ParameterValue::Texts(items.iter().map(parameter_text).collect()),
        Value::Text(_) | Value::Float(_) | Value::Table(_) => {
            ParameterValue::Text(parameter_text(value))
        }
    }
}

/// A value as ryo reads a text parameter: text and a float as written, a whole number and a flag as TOML writes them,
/// and a list or a table as TOML writes it inline, the text a rule's entry wrote.
fn parameter_text(value: &Value) -> ParameterText {
    ParameterText(
        match value {
            Value::Text(text) => text.0.to_string(),
            Value::Float(text) => text.0.to_string(),
            Value::Whole(_) | Value::Flag(_) | Value::List(_) | Value::Table(_) => {
                inline(value).to_string()
            }
        }
        .into(),
    )
}

/// A value as a TOML value: a float keeps the text it was read as.
fn inline(value: &Value) -> toml_edit::Value {
    match value {
        Value::Text(text) => toml_edit::Value::from(&*text.0),
        Value::Whole(number) => toml_edit::Value::from(number.0),
        Value::Float(text) => text
            .0
            .parse::<toml_edit::Value>()
            .unwrap_or_else(|_unparsed| toml_edit::Value::from(&*text.0)),
        Value::Flag(FlagValue::Yes) => toml_edit::Value::from(true),
        Value::Flag(FlagValue::No) => toml_edit::Value::from(false),
        Value::List(items) => {
            toml_edit::Value::Array(items.iter().map(inline).collect::<toml_edit::Array>())
        }
        Value::Table(table) => toml_edit::Value::InlineTable(
            table
                .iter()
                .map(|(key, item)| (&*key.0, inline(item)))
                .collect::<toml_edit::InlineTable>(),
        ),
    }
}

/// The core's request, offered to the conversion into the SDK's.
pub(super) struct AnalysisRequestSchema(pub AnalysisRequest);

/// The SDK's report, offered to the conversion into the core's.
pub(super) struct LintReportSchema(pub LintReport);

/// The SDK's failure, offered to the conversion into the core's.
pub(super) struct LintErrorSchema(pub LintError);

impl TryFrom<AnalysisRequestSchema> for LintRequest {
    type Error = AnalysisError;

    fn try_from(schema: AnalysisRequestSchema) -> Result<Self, Self::Error> {
        convert_request(schema.0)
    }
}

impl From<LintReportSchema> for AnalysisReport {
    fn from(schema: LintReportSchema) -> Self {
        convert_report(schema.0)
    }
}

impl From<LintErrorSchema> for AnalysisError {
    fn from(schema: LintErrorSchema) -> Self {
        convert_error(&schema.0)
    }
}

/// The SDK's request for the core's.
///
/// # Errors
///
/// `Rule` for a rule whose id or glob the SDK refuses.
pub(super) fn request(request: AnalysisRequest) -> Result<LintRequest, AnalysisError> {
    AnalysisRequestSchema(request).try_into()
}

/// The core's report for the SDK's.
pub(super) fn report(report: LintReport) -> AnalysisReport {
    LintReportSchema(report).into()
}

/// The core's error for the SDK's: one rule's failure names the rule, so the caller can leave it out.
pub(super) fn error(error: LintError) -> AnalysisError {
    LintErrorSchema(error).into()
}

/// Where ryo keeps its work for `store`: in the store's own directory, or nowhere under `-n/--dry-run`.
pub(super) fn cache(store: &crate::domain::settings::types::Store) -> ryo::linter::Cache {
    match store.writing {
        crate::domain::writing::types::Writing::Write => {
            ryo::linter::Cache::At(std::path::PathBuf::from(&*store.directory.0).join(RYO_STORE))
        }
        crate::domain::writing::types::Writing::DryRun => ryo::linter::Cache::None,
    }
}

/// The SDK's source for a file the core gives with its text.
fn source(file: crate::outbound::analysis::types::GivenFile) -> CaseSource {
    CaseSource {
        path: RelativePath(file.path.0),
        text: SourceText(file.text.0),
    }
}

/// The SDK's request for the core's, refusing a rule that does not parse.
fn convert_request(request: AnalysisRequest) -> Result<LintRequest, AnalysisError> {
    let mut rules = BTreeMap::new();
    let mut parsed: BTreeMap<Box<str>, Glob> = BTreeMap::new();
    for rule in request.rules {
        let refused = |error: &dyn std::fmt::Display| AnalysisError::Rule {
            rule: rule.id.clone(),
            reason: error.to_string(),
        };
        let id = RuleId::try_from(&*rule.id.0).map_err(|error| refused(&error))?;
        let query =
            ryo::query::RyoQlQuery::try_from(&*rule.query.0).map_err(|error| refused(&error))?;
        // The rules of a request repeat the same globs many times over: each text is parsed once.
        let mut globs = |texts: &[GlobText]| {
            texts
                .iter()
                .map(|glob| {
                    if let Some(parsed) = parsed.get(&glob.0) {
                        return Ok(Glob::clone(parsed));
                    }
                    let made = Glob::try_from(&*glob.0).map_err(|error| refused(&error))?;
                    drop(parsed.insert(glob.0.clone(), made.clone()));
                    Ok(made)
                })
                .collect::<Result<Vec<_>, AnalysisError>>()
        };
        let include = globs(&rule.include)?;
        let exclude = globs(&rule.exclude)?;
        drop(
            rules.insert(
                id,
                Rule {
                    query,
                    parameters: rule
                        .parameters
                        .iter()
                        .map(|(name, value)| (ParameterName(name.0.clone()), parameter(value)))
                        .collect(),
                    include,
                    exclude,
                    applicability: RuleApplicability::Always,
                },
            ),
        );
    }
    Ok(LintRequest {
        root: std::path::PathBuf::from(&*request.root.0),
        rules,
        files: Vec::new(),
        hosted: std::iter::once(placed(&request.placed))
            .chain(match &request.hosted {
                Hosted::Nothing => Vec::new(),
                Hosted::Verdict(rows) => verdict(rows),
                Hosted::Counts(rows) => counts(rows),
            })
            .collect(),
        corpus: match request.corpus {
            Corpus::Walked => Reads::Walked,
            Corpus::Listed(paths) => {
                Reads::Listed(paths.into_iter().map(|path| RelativePath(path.0)).collect())
            }
            Corpus::Given(files) => Reads::Given(files.into_iter().map(source).collect()),
        },
        beside: request.beside.into_iter().map(source).collect(),
    })
}

/// A byte offset of a span the SDK hands back, as the core counts it.
fn offset(offset: u32) -> ByteOffset {
    ByteOffset(usize::try_from(offset).unwrap_or(usize::MAX))
}

/// The path a finding is of, read from its subject: a file as it is. A subject of another kind is the text it reads as, as before
/// the SDK typed its cells (a later step reads the finding's file from the SDK, not from its subject's text).
fn subject_path(subject: Slot) -> RepoPath {
    RepoPath(match subject {
        Slot::Is(Cell::File(path)) => path.0,
        Slot::Is(Cell::Text(text)) => text.0,
        Slot::Is(Cell::Number(number)) => number.0.to_string().into(),
        Slot::Is(Cell::Span(span)) => format!("{}{SPAN_TEXT}{}", span.start.0, span.end.0).into(),
        Slot::Is(_) | Slot::Nothing | _ => Box::default(),
    })
}

/// A column of the SDK's row as a cell, by the kind the SDK typed it: the row's own file, a span, or text; a number and any other
/// kind are the text they read as, and a value that reads as nothing is empty text.
fn cell(path: &RepoPath, name: ColumnName, value: Slot) -> AnswerCell {
    let value = match value {
        Slot::Is(Cell::File(file)) if file.0 == path.0 => CellValue::File(path.clone()),
        Slot::Is(Cell::File(file)) => CellValue::Text(CellText(file.0)),
        Slot::Is(Cell::Span(span)) => CellValue::Span(ByteSpan {
            start: offset(span.start.0),
            end: offset(span.end.0),
        }),
        Slot::Is(Cell::Text(text)) => CellValue::Text(CellText(text.0)),
        Slot::Is(Cell::Number(number)) => CellValue::Text(CellText(number.0.to_string().into())),
        Slot::Is(_) | Slot::Nothing | _ => CellValue::Text(CellText(Box::default())),
    };
    AnswerCell { name, value }
}

/// The core's report for the SDK's.
fn convert_report(report: LintReport) -> AnalysisReport {
    AnalysisReport {
        findings: report
            .findings
            .into_iter()
            .map(|finding| {
                let path = subject_path(finding.subject);
                let cells: Vec<AnswerCell> = finding
                    .columns
                    .into_iter()
                    .map(|column| cell(&path, ColumnName(column.name.0), column.value))
                    .collect();
                LintAnswer {
                    rule: LintRuleId(finding.rule.as_ref().into()),
                    path,
                    cells,
                }
            })
            .collect(),
        unknowns: report
            .unknowns
            .into_iter()
            .map(|unknown| LintUnknown {
                rule: LintRuleId(unknown.rule.as_ref().into()),
                path: RepoPath(unknown.path.0),
                message: MessageText(unknown.message.0),
            })
            .collect(),
        unread: report
            .unread
            .into_iter()
            .map(|unread| RepoPath(unread.path.0))
            .collect(),
    }
}

/// The core's error for the SDK's failure.
fn convert_error(error: &LintError) -> AnalysisError {
    match error {
        LintError::Rule { id, .. }
        | LintError::Unrunnable { id, .. }
        | LintError::MissingParameter { id } => AnalysisError::Rule {
            rule: LintRuleId(id.as_ref().into()),
            reason: error.to_string(),
        },
        LintError::TooLarge { .. }
        | LintError::Provider { .. }
        | LintError::MemoryBudgetExceeded { .. }
        | LintError::DuplicateHostRelation { .. }
        | LintError::HostRowWidth { .. }
        | LintError::HostCellKind { .. }
        | LintError::PathNotText { .. }
        | LintError::Io { .. }
        | _ => engine(error),
    }
}

/// Each item converted, in order.
fn each<T, U>(items: Vec<T>, convert: impl FnMut(T) -> U) -> Vec<U> {
    items.into_iter().map(convert).collect()
}

/// The core's page of a relation ryo describes.
pub(super) fn relation_page(relation: ryo::relation::RelationDescription) -> RelationPage {
    RelationPage {
        entry: CatalogEntry {
            name: CatalogName(relation.name.0),
            summary: CatalogText(relation.summary.0),
        },
        columns: each(relation.columns, |column| CatalogColumn {
            name: CatalogName(column.name.0),
            kind: CatalogName(column.kind.0),
            words: each(column.words, |word| CatalogName(word.0)),
        }),
        languages: each(relation.languages, |language| CatalogLanguage {
            name: CatalogName(language.name.0),
            reading: match language.reads {
                ryo::relation::Reading::Reads => Reading::Reads,
                ryo::relation::Reading::DoesNotRead => {
                    Reading::Absent(CatalogName(language.reason.0))
                }
            },
            unknowns: each(language.unknowns, |reason| CatalogName(reason.0)),
        }),
    }
}

/// The core's page of an operator ryo describes.
pub(super) fn operator_page(operator: ryo::operator::OperatorDescription) -> OperatorPage {
    OperatorPage {
        entry: CatalogEntry {
            name: CatalogName(operator.name.0),
            summary: CatalogText(operator.summary.0),
        },
        fields: each(operator.fields, |field| CatalogField {
            name: CatalogName(field.name.0),
            requirement: match field.required {
                ryo::operator::Requirement::Required => Requirement::Required,
                ryo::operator::Requirement::Optional => Requirement::Optional,
            },
            summary: CatalogText(field.summary.0),
        }),
        options: each(operator.options, |option| CatalogText(option.0)),
        example: CatalogText(operator.example.0),
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
