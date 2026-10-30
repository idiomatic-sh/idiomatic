//! Cross-file evaluation and bounded source rendering over injected ports.
use crate::domain::answering::types::{Crossed, Todo};
use crate::domain::crossing::service::{cross_rule, placeable};
use crate::domain::crossing::types::{CrossingInput, Placeability};
use crate::domain::finish::errors::FinishError;
use crate::domain::finish::service::{
    column_cell, founds, line_cell, place_all, shown_cell, source_of,
};
use crate::domain::finish::types::{Arenas, FoundRow, HeldFile, Loaded, Resolving, RuleNumber};
use crate::domain::ordering::service::{order_alike, sort_findings};
use crate::domain::render::service::{end_row, key_count, next_row};
use crate::domain::render::types::{
    AnswerCell, AnswerColumns, Answering, KeyCount, Rendered, RowNumber, RowOrder,
};
use crate::domain::reporting::service::order_found;
use crate::domain::rules::errors::RulesError;
use crate::domain::rules::service::failed;
use crate::domain::table_model::service::cell;
use crate::domain::table_model::types::Frontier;
use crate::domain::table_texts::types::SourceView;
use crate::outbound::corpus::interface::CorpusPort;
use ryo_language::domain::relations::types::{FileId, Value};
use ryo_language::domain::symbols::service::file_in;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::files::types::{CorpusEntry, FileBytes};
use ryo_values::domain::records::service::keep;
use ryo_values::domain::records::types::RenderedText;
use ryo_values::domain::records::types::RowTotal;
use ryo_values::domain::records::types::{Cell, CellText, TextRange};

use std::sync::Arc;
/// Cross-file work sharing the caller's corpus resource.
pub struct CrossFile {
    /// Reads back the files the findings name.
    corpus: Arc<dyn CorpusPort>,
}

impl std::fmt::Debug for CrossFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl CrossFile {
    /// Bind source access once at composition.
    #[must_use]
    pub fn new(corpus: Arc<dyn CorpusPort>) -> Self {
        Self { corpus }
    }
    /// Evaluate, place and render one cross-file rule without publishing it.
    pub(crate) fn evaluate(
        &self,
        crossing: &CrossingInput<'_>,
        todo: &Todo,
        frontier: &[Frontier],
        cross: &mut Symbols,
    ) -> Result<Crossed, RulesError> {
        let (Some(plan), Some(_)) = (
            crossing.plans.get(todo.rule.0),
            crossing.bindings.get(todo.rule.0),
        ) else {
            return Ok(Crossed::default());
        };
        let evaluated = cross_rule(crossing, todo, frontier, cross)?;
        let refuse = |reason: &dyn std::fmt::Display| failed(&todo.id, reason);
        let rule_number = RuleNumber(u32::try_from(todo.rule.0).unwrap_or(u32::MAX));
        let mut found = founds(rule_number, &evaluated.table, &plan.emit, FileId(u32::MAX))
            .map_err(|error| refuse(&error))?;
        found.sort_by_key(|each| each.file);
        let mut bytes = FileBytes::default();
        if placeable(plan, &evaluated.table) == Placeability::Placeable {
            for group in found.chunk_by_mut(|left, right| left.file == right.file) {
                let Some(entry) = group
                    .first()
                    .and_then(|first| file_in(crossing.files, first.file))
                else {
                    continue;
                };
                if self.corpus.read(entry, &mut bytes).is_ok() {
                    place_all(group, &plan.emit, SourceView(&bytes.0), cross);
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
        Ok(Crossed {
            rendered,
            fed: evaluated.fed,
        })
    }
    /// The cell `value` is in a report, its text kept at the end of `text`, reading the file it is about when it is not the one loaded.
    pub(super) fn show_value(
        &self,
        text: &mut RenderedText,
        value: Value,
        files: &[CorpusEntry],
        resolving: &Resolving<'_>,
        loaded: &mut Loaded,
    ) -> Option<Cell<TextRange>> {
        if let Some(file) = source_of(value, resolving)
            && loaded.held != HeldFile::File(file)
        {
            if let Some(entry) = file_in(files, file) {
                let mut bytes = FileBytes(std::mem::take(&mut loaded.bytes.0));
                // A file that can no longer be read shows no text, rather than the last file's.
                if self.corpus.read(entry, &mut bytes).is_err() {
                    bytes.0.clear();
                }
                loaded.bytes.0 = bytes.0;
            }
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
        crossing: &CrossingInput<'_>,
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
        let (plans, ids, files) = (crossing.plans, crossing.ids, crossing.files);
        for each in found {
            let Some(plan) = plans.get(usize::try_from(each.rule.0).unwrap_or(usize::MAX)) else {
                continue;
            };
            let id = ids
                .get(usize::try_from(each.rule.0).unwrap_or(usize::MAX))
                .map(|id| keep(&mut into.text, Cell::Text(CellText(&id.0))));
            into.cells.push(id);
            let subject = self.show_value(
                &mut into.text,
                cell(&each.row, plan.emit.subject)?,
                files,
                &resolving,
                &mut loaded,
            );
            into.cells.push(subject);
            into.cells
                .push(Some(key_count(KeyCount(plan.emit.key.len()))));
            for index in &plan.emit.key {
                let shown = self.show_value(
                    &mut into.text,
                    cell(&each.row, *index)?,
                    files,
                    &resolving,
                    &mut loaded,
                );
                into.cells.push(shown);
            }
            for (heading, value) in plan.emit.headings.iter().zip(&each.row) {
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
        crossing: &CrossingInput<'_>,
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
