//! `query run` through the executor: the queries run as the rules of one request of the SDK's `Linter`, whose findings carry every column of
//! each emitted row; the rows are written to the same sink the old engine's answer went to. Built beside the old path (`command::answer`)
//! so that the two can be compared; the command switches to this one when they agree.

use ryo_core::inbound::use_cases::lint::errors::LintError;
use ryo_core::outbound::memory::interface::MemoryPort;
use ryo_core::outbound::row_sink::interface::RowSinkPort;
use ryo_core::outbound::row_sink::types::{AnswerCounts, LentCell};
use ryo_sdk::linter as sdk;
use ryo_values::domain::plan_parts::types::Heading;
use ryo_values::domain::records::types::{Unread as CoreUnread, Unreadable as CoreUnreadable};
use ryo_values::domain::row_texts::types::{Cell, CellText, RowTotal};
use std::collections::BTreeMap;
use std::sync::Arc;

/// The headings of the answer of one query: what its `emit` names, as the compiler of the query says, and the refusal of a query that
/// is not correct as the engine's query-language port says it (the compiler's message, without the module it came from).
fn headings_of(id: &str, text: &str) -> Result<Vec<Heading>, LintError> {
    use ryo_execution::outbound::query_language::errors::QueryLanguageError;
    use ryo_execution::outbound::query_language::interface::QueryLanguagePort as _;
    let compile = Arc::new(ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    ));
    let typecheck = Arc::new(ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl::new(
        Arc::new(ryo_query_language::outbound::syntax::factory::winnow()),
        Arc::new(ryo_query_language::outbound::catalog::factory::language()),
    ));
    let port = ryo_execution::outbound::query_language::factory::use_case(compile, typecheck);
    let bundle = ryo_values::domain::query_tokens::types::QueryBundle::from(ryo_values::domain::plan_parts::types::QueryText(text.into()));
    let plan = port
        .compile(&bundle, &[], &[])
        .map_err(|QueryLanguageError::Refused { error }| LintError::Query { id: id.into(), error })?;
    Ok(plan.emit.headings)
}

/// The cell a finding's column holds, as the sink takes it.
fn cell(slot: &sdk::Slot) -> LentCell<'_> {
    match slot {
        sdk::Slot::Is(sdk::Cell::File(path)) => LentCell::Valued(Cell::File(CellText(&path.0))),
        sdk::Slot::Is(sdk::Cell::Text(text)) => LentCell::Valued(Cell::Text(CellText(&text.0))),
        sdk::Slot::Is(sdk::Cell::Number(number)) => LentCell::Valued(Cell::Number(ryo_relations::domain::relations::types::Integer(number.0))),
        sdk::Slot::Is(sdk::Cell::Span(span)) => LentCell::Valued(Cell::Span(*span)),
        _ => LentCell::Empty,
    }
}

/// The bytes of the files below `root` the repository does not ignore, as the old engine's walk finds them: what the memory budget is
/// held against before any file is read. The SDK walks the corpus for the run itself; this walk only measures.
fn repository_size(root: &str, skipped: &ryo_core::outbound::corpus::types::SkippedDirectories) -> Result<u64, LintError> {
    use ryo_core::outbound::corpus::interface::CorpusPort as _;
    let walk = ryo_core::outbound::corpus::factory::ignore(
        ryo_language::inbound::use_cases::project::types::RootPath(root.into()),
        ryo_values::domain::files::types::CorpusListing::Walked,
        Vec::new(),
        skipped.clone(),
    );
    let inventory = walk.inventory().map_err(|error| LintError::Corpus { reason: error.to_string() })?;
    let mut size = 0_u64;
    for input in inventory.0 {
        if let ryo_values::domain::files::types::CorpusInput::Available(entry) = input {
            size += entry.size.0;
        }
    }
    Ok(size)
}

/// Runs `texts` as the queries of one lint request and writes the rows to `sink`: at most `limit` of them, every one counted.
pub(super) fn answer(
    texts: &[String],
    limit: u64,
    root: &str,
    budget: ryo_values::domain::runs::types::Budget,
    workers: std::num::NonZeroUsize,
    skipped: &ryo_core::outbound::corpus::types::SkippedDirectories,
    sink: &dyn RowSinkPort,
) -> Result<(), LintError> {
    let several = texts.len() > 1;
    let mut rules = BTreeMap::new();
    let mut headings: Vec<Vec<Heading>> = Vec::new();
    for (place, text) in texts.iter().enumerate() {
        headings.push(headings_of(&(place + 1).to_string(), text)?);
        let query = ryo_sdk::query::RyoQlQuery::try_from(text.as_str())
            .map_err(|error| LintError::Rule { id: (place + 1).to_string(), reason: error.to_string() })?;
        let id = sdk::RuleId::try_from((place + 1).to_string().as_str()).map_err(|error| LintError::Execution { reason: error.to_string() })?;
        let _prior = rules.insert(
            id,
            sdk::Rule {
                observations: Vec::new(),
                query,
                applicability: sdk::RuleApplicability::Always,
                parameters: BTreeMap::new(),
                include: Vec::new(),
                exclude: Vec::new(),
            },
        );
    }
    let linter = sdk::Linter::new(crate::languages::linked_sdk(), sdk::Workers(workers).into(), sdk::Cache::None);
    let memory = ryo_core::outbound::memory::factory::sysinfo(budget);
    let may_use = memory.budget().map_err(|error| LintError::Memory { reason: error.to_string() })?.0;
    let ids: Vec<String> = (1..=texts.len()).map(|place| place.to_string()).collect();
    let within = |memory: &dyn MemoryPort| -> Result<(), LintError> {
        let used = memory.resident_bytes().map_err(|error| LintError::Memory { reason: error.to_string() })?.0;
        if used > may_use {
            return Err(LintError::MemoryBudgetExceeded { used, budget: may_use, plans: ids.clone() });
        }
        Ok(())
    };
    let size = repository_size(root, skipped)?;
    if size > may_use {
        return Err(LintError::TooLarge { size, budget: may_use });
    }
    within(&memory)?;
    let request = sdk::LintRequest {
        source_language: sdk::SourceLanguage::Recognized,
        root: root.into(),
        project_search_scope: Default::default(),
        given_project_inventory: Default::default(),
        rules,
        files: Vec::new(),
        hosted: Vec::new(),
        corpus: sdk::Corpus::Walked,
        beside: Vec::new(),
    };
    let report = linter
        .lint_rows(request, usize::try_from(limit).unwrap_or(usize::MAX))
        .map_err(|error| LintError::Execution { reason: error.to_string() })?;
    // The columns of the answer: with several queries, the rule's place first and then every query's columns, each once.
    let mut columns: Vec<Heading> = Vec::new();
    if several {
        columns.push(Heading("rule".into()));
    }
    for each in headings.iter().flatten() {
        if !columns.contains(each) {
            columns.push(each.clone());
        }
    }
    columns.push(Heading("line".into()));
    columns.push(Heading("column".into()));
    sink.begin(&columns);
    let total = u64::try_from(report.rows_total).unwrap_or(u64::MAX);
    let mut shown = 0_u64;
    for finding in &report.findings {
        let id = finding.rule.to_string();
        let by_name: BTreeMap<&str, &sdk::Slot> = finding.columns.iter().map(|column| (column.name.0.as_ref(), &column.value)).collect();
        let mut cells: Vec<LentCell<'_>> = Vec::with_capacity(columns.len());
        let (line, column) = match finding.place {
            sdk::Place::At { line, column } => (
                LentCell::Valued(Cell::Number(ryo_relations::domain::relations::types::Integer(i64::from(line.0)))),
                LentCell::Valued(Cell::Number(ryo_relations::domain::relations::types::Integer(i64::from(column.0)))),
            ),
            _ => (LentCell::Empty, LentCell::Empty),
        };
        for (at, heading) in columns.iter().enumerate() {
            if heading.0.as_ref() == "line" && at + 2 == columns.len() {
                cells.push(line);
                continue;
            }
            if heading.0.as_ref() == "column" && at + 1 == columns.len() {
                cells.push(column);
                continue;
            }
            if several && at == 0 {
                cells.push(LentCell::Valued(Cell::Text(CellText(&id))));
                continue;
            }
            cells.push(by_name.get(heading.0.as_ref()).map_or(LentCell::Empty, |slot| cell(slot)));
        }
        sink.row(&mut cells.into_iter());
        shown += 1;
    }
    let unread: Vec<CoreUnread> = report
        .unread
        .iter()
        .map(|each| CoreUnread {
            path: ryo_language::domain::files::types::FilePath(each.path.0.clone()),
            reason: match &each.reason {
                sdk::Unreadable::NoLanguage => CoreUnreadable::NoLanguage,
                sdk::Unreadable::Failed(reason) => CoreUnreadable::Failed(ryo_values::domain::stored_facts::types::ReadFailure(reason.0.clone())),
                _ => CoreUnreadable::NoLanguage,
            },
        })
        .collect();
    sink.end(AnswerCounts { total: RowTotal(total), shown: RowTotal(shown) }, &unread)
        .map_err(|error| LintError::Output { reason: error.to_string() })
}
