//! Placement and bounded source rendering of the cross-file answer over the injected corpus.
use crate::application::cross_file::types::ProjectCrossing;
use crate::domain::answering::types::Todo;
use crate::domain::crossing::service::placeable;
use crate::domain::crossing::types::Placeability;
use crate::domain::finish::errors::FinishError;
use crate::domain::finish::service::column_cell;
use crate::domain::finish::service::founds;
use crate::domain::finish::service::line_cell;
use crate::domain::finish::service::place_all;
use crate::domain::finish::service::shown_cell;
use crate::domain::finish::service::source_of;
use crate::domain::finish::types::Arenas;
use crate::domain::finish::types::FoundRow;
use crate::domain::finish::types::HeldFile;
use crate::domain::finish::types::Loaded;
use crate::domain::finish::types::Resolving;
use crate::domain::ordering::service::order_alike;
use crate::domain::ordering::service::sort_findings;
use crate::domain::render::service::end_row;
use crate::domain::render::service::key_count;
use crate::domain::render::service::next_row;
use crate::domain::render::types::AnswerCell;
use crate::domain::render::types::AnswerColumns;
use crate::domain::render::types::Answering;
use crate::domain::render::types::KeyCount;
use crate::domain::render::types::Rendered;
use crate::domain::render::types::RowNumber;
use crate::domain::render::types::RowOrder;
use crate::domain::reporting::service::order_found;
use crate::outbound::corpus::interface::CorpusPort;
use ryo_execution::application::file_dag::program::prepared_execution::interface::PreparedExecution;
use ryo_execution::inbound::use_cases::metadata::interface::Governing;
use ryo_execution::inbound::use_cases::metadata::interface::Mask;
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_language::domain::symbols::service::file_in;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::files::types::FileBytes;
use ryo_values::domain::files::types::Hits;
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::records::service::keep;
use ryo_values::domain::row_texts::types::Cell;
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::row_texts::types::RenderedText;
use ryo_values::domain::row_texts::types::RowTotal;
use ryo_values::domain::row_texts::types::TextRange;
use ryo_values::domain::rows::service::cell;
use ryo_values::domain::rule_requests::errors::RuleRequestsError;
use ryo_values::domain::rule_requests::service::failed;
use ryo_values::domain::runs::types::RuleNumber;
use std::sync::Arc;

/// Renders the answer of a cross-file rule, reading back the files its findings name.
pub(crate) struct Rendering {
    /// Reads back the files the findings name.
    corpus: Arc<dyn CorpusPort>,
}

impl Rendering {
    /// Bind source access once at composition.
    #[must_use]
    pub(crate) fn new(corpus: Arc<dyn CorpusPort>) -> Self {
        Self { corpus }
    }
    /// Place and render the actual diagnostic root; named-root production is independent.
    pub(crate) fn render_emit(
        &self,
        crossing: &ProjectCrossing<'_>,
        todo: &Todo,
        emit: &ryo_values::domain::plans::types::Emit,
        table: Option<ryo_values::domain::rows::types::Table>,
        cross: &mut Symbols,
    ) -> Result<Option<Rendered>, RuleRequestsError> {
        let refuse = |reason: &dyn std::fmt::Display| failed(&todo.id, reason);
        let rule_number = RuleNumber(u32::try_from(todo.rule.0).unwrap_or(u32::MAX));
        Ok(if let Some(table) = table {
            let mut found = founds(rule_number, &table, emit, FileId(u32::MAX))
                .map_err(|error| refuse(&error))?;
            // Observation demand can execute a rule on a file outside its
            // diagnostic scope. Apply the same compiled placement decision to
            // each actual source file before reporting its cross-file Emit.
            let mut visible = std::collections::HashMap::new();
            let (mut mask, mut hits) = (Mask::default(), Hits::default());
            found.retain(|each| {
                let Ok(index) = usize::try_from(each.file.0) else {
                    // Unlisted manifests and unplaceable project rows were
                    // never visited by per-file placement. Preserve their
                    // separate project rendering contract.
                    return true;
                };
                let Some(path) = crossing
                    .paths
                    .get(index)
                    .filter(|_| index < crossing.files.len())
                else {
                    // Unlisted manifests and unplaceable project rows were
                    // never visited by per-file placement. Preserve their
                    // separate project rendering contract.
                    return true;
                };
                *visible.entry(each.file.0).or_insert_with(|| {
                    MetadataUseCaseImpl.place(crossing.placed, path, &mut mask, &mut hits);
                    MetadataUseCaseImpl.governs(&mask, rule_number) == Governing::Governs
                })
            });
            found.sort_by_key(|each| each.file);
            let mut bytes = FileBytes::default();
            if placeable(emit, &table) == Placeability::Placeable {
                for group in found.chunk_by_mut(|left, right| left.file == right.file) {
                    let Some(CorpusInput::Available(entry)) = group
                        .first()
                        .and_then(|first| file_in(crossing.files, first.file))
                    else {
                        continue;
                    };
                    if self.corpus.read(entry, &mut bytes).is_ok() {
                        place_all(group, emit, SourceView(&bytes.0), cross);
                    }
                }
            }
            let mut rendered = Rendered::default();
            match crossing.answering {
                Answering::Findings => self.render_cross(crossing, cross, &found, &mut rendered),
                Answering::Rows(_) => {
                    order_found(&mut found, crossing.paths);
                    self.render_rows(crossing, cross, &found, &mut rendered);
                    Ok(())
                }
            }
            .map_err(|error| refuse(&error))?;
            Some(rendered)
        } else {
            None
        })
    }
    /// The cell `value` is in a report, its text kept at the end of `text`, reading the file it is about when it is not the one loaded.
    pub(crate) fn show_value(
        &self,
        text: &mut RenderedText,
        value: Value,
        files: &[CorpusInput],
        resolving: &Resolving<'_>,
        loaded: &mut Loaded,
    ) -> Option<Cell<TextRange>> {
        if let Some(file) = source_of(value, resolving)
            && loaded.held != HeldFile::File(file)
        {
            let mut bytes = FileBytes(std::mem::take(&mut loaded.bytes.0));
            bytes.0.clear();
            if let Some(CorpusInput::Available(entry)) = file_in(files, file) {
                // A refusal never exposes this worker's previous file or partial read.
                if self.corpus.read(entry, &mut bytes).is_err() {
                    bytes.0.clear();
                }
            }
            loaded.bytes.0 = bytes.0;
            loaded.held = HeldFile::File(file);
        }
        shown_cell(text, value, resolving, loaded)
    }
    /// Renders the findings the cross-file steps gave onto `into`, as the cells of a finding (its rule, its subject, its key, its
    /// columns, its line and its character), in the order they leave in. Their names are read back from the files they name, one file
    /// at a time.
    ///
    /// # Errors
    ///
    /// `NoSuchColumn` when a finding lacks the subject or a key column.
    fn render_cross(
        &self,
        crossing: &ProjectCrossing<'_>,
        cross: &Symbols,
        found: &[FoundRow],
        into: &mut Rendered,
    ) -> Result<(), FinishError> {
        let resolving = Resolving {
            paths: crossing.paths,
            arenas: Arenas(crossing.arenas),
            cross,
            unknown: crossing.unknown,
        };
        let mut loaded = Loaded::default();
        let (ids, files) = (crossing.ids, crossing.files);
        for each in found {
            let emit = crossing
                .program
                .emit(
                    ryo_execution::inbound::use_cases::metadata::interface::RuleIndex(
                        usize::try_from(each.rule.0).unwrap_or(usize::MAX),
                    ),
                )
                .map_err(|error| FinishError::Publication {
                    reason: error.to_string().into_boxed_str(),
                })?;
            let id = ids
                .get(usize::try_from(each.rule.0).unwrap_or(usize::MAX))
                .map(|id| keep(&mut into.text, Cell::Text(CellText(&id.0))));
            into.cells.push(id);
            let subject = self.show_value(
                &mut into.text,
                cell(&each.row, emit.subject)?,
                files,
                &resolving,
                &mut loaded,
            );
            into.cells.push(subject);
            into.cells.push(Some(key_count(KeyCount(emit.key.len()))));
            for index in &emit.key {
                let shown = self.show_value(
                    &mut into.text,
                    cell(&each.row, *index)?,
                    files,
                    &resolving,
                    &mut loaded,
                );
                into.cells.push(shown);
            }
            for (heading, value) in emit.headings.iter().zip(&each.row) {
                let named = keep(&mut into.text, Cell::Text(CellText(&heading.0)));
                into.cells.push(Some(named));
                let shown = self.show_value(&mut into.text, *value, files, &resolving, &mut loaded);
                into.cells.push(shown);
            }
            into.cells.push(line_cell(each.place));
            into.cells.push(column_cell(each.place));
            let row = next_row(into);
            end_row(into);
            into.order.push(RowOrder {
                rule: each.rule,
                line: each.place.line,
                column: each.place.column,
                row,
            });
        }
        sort_findings(into);
        Ok(())
    }

    /// Renders the rows the cross-file steps gave onto `into` as the rule's own columns, in the order of their places, at most as
    /// many as the answer writes: every row is counted. With more than one rule, the rule's id comes first and each column of the
    /// answer is the rule's own of that heading, or empty.
    fn render_rows(
        &self,
        crossing: &ProjectCrossing<'_>,
        cross: &Symbols,
        found: &[FoundRow],
        into: &mut Rendered,
    ) {
        let Answering::Rows(answer) = crossing.answering else {
            return;
        };
        let resolving = Resolving {
            paths: crossing.paths,
            arenas: Arenas(crossing.arenas),
            cross,
            unknown: crossing.unknown,
        };
        let mut loaded = Loaded::default();
        into.total = RowTotal(u64::try_from(found.len()).unwrap_or(u64::MAX));
        // Every row is rendered and the rows of one place are put in the order of their cells before the limit cuts: which rows of a
        // place were made first depends on the workers, and the answer must not.
        let mut starts = Vec::new();
        let mut before: Option<&FoundRow> = None;
        for each in found {
            if before.is_none_or(|last| last.file != each.file || last.place != each.place) {
                starts.push(RowNumber(
                    u32::try_from(into.order.len()).unwrap_or(u32::MAX),
                ));
            }
            before = Some(each);
            let rule = usize::try_from(each.rule.0).unwrap_or(usize::MAX);
            match &answer.columns {
                AnswerColumns::One => {
                    for value in &each.row {
                        let shown = self.show_value(
                            &mut into.text,
                            *value,
                            crossing.files,
                            &resolving,
                            &mut loaded,
                        );
                        into.cells.push(shown);
                    }
                }
                AnswerColumns::Many(cells) => {
                    let cells = cells.get(rule).map_or(&[][..], Vec::as_slice);
                    let id = crossing
                        .ids
                        .get(rule)
                        .map(|id| keep(&mut into.text, Cell::Text(CellText(&id.0))));
                    into.cells.push(id);
                    for each_cell in cells {
                        let shown = match each_cell {
                            AnswerCell::Column(place) => match each.row.get(place.0) {
                                Some(value) => self.show_value(
                                    &mut into.text,
                                    *value,
                                    crossing.files,
                                    &resolving,
                                    &mut loaded,
                                ),
                                None => None,
                            },
                            AnswerCell::Empty => None,
                        };
                        into.cells.push(shown);
                    }
                }
            }
            into.cells.push(line_cell(each.place));
            into.cells.push(column_cell(each.place));
            let row = next_row(into);
            end_row(into);
            into.order.push(RowOrder {
                rule: each.rule,
                line: each.place.line,
                column: each.place.column,
                row,
            });
        }
        order_alike(into, &starts);
        into.order
            .truncate(usize::try_from(answer.limit.0).unwrap_or(usize::MAX));
    }
}
