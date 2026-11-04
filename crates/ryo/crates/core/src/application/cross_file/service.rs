//! Cross-file evaluation and bounded source rendering over injected ports.
use crate::application::file_dag::crossing::cross_rule_with_inputs;
use crate::application::file_dag::settings::CrossingInput;
use crate::domain::answering::types::{Crossed, Todo};
use crate::domain::crossing::service::placeable;
use crate::domain::crossing::types::Placeability;
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
use ryo_language::domain::symbols::service::file_in;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{FileId, Value};
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
    /// Diagnostic-only entry; demanded observation phases require the explicit reserved-slot entry.
    pub(crate) fn evaluate(
        &self,
        crossing: &CrossingInput<'_>,
        todo: &Todo,
        frontier: Vec<Frontier>,
        cross: &mut Symbols,
    ) -> Result<Crossed, RulesError> {
        self.evaluate_phase(
            crossing,
            todo,
            super::types::PreparedFrontiers {
                rows: frontier,
                refused: Vec::new(),
            },
            cross,
            &[],
            super::types::PublicationSlot::Unused,
        )
    }
    pub(crate) fn evaluate_phase(
        &self,
        crossing: &CrossingInput<'_>,
        todo: &Todo,
        mut frontier: super::types::PreparedFrontiers,
        cross: &mut Symbols,
        bound_inputs: &[crate::domain::crossing::types::BoundInputWords],
        publication: super::types::PublicationSlot,
    ) -> Result<Crossed, RulesError> {
        let scope = crate::application::file_dag::scope::ExecutionScope::new(
            std::sync::Arc::clone(&crossing.graph),
            crate::application::file_dag::scope::ScopePhase::Project,
            false, crossing.executor,
        ).map_err(|error| failed(&todo.id, &error))?;
        #[cfg(feature = "diagnostic-row-progress")]
        let _project_progress =
            crate::application::file_dag::route_counts::FileScope::begin(usize::MAX);
        #[cfg(feature = "diagnostic-row-progress")]
        let _project_rule =
            crate::application::file_dag::route_counts::rule(todo.rule.0, todo.id.0.as_ref());
        #[cfg(feature = "diagnostic-row-progress")]
        crate::application::file_dag::route_counts::file_name("<project>");
        let (Some(plan), Some(_)) = (
            crossing.plans.get(todo.rule.0),
            crossing.bindings.get(todo.rule.0),
        ) else {
            return Ok(Crossed::default());
        };
        for input in bound_inputs {
            match super::inputs::observed(crossing, scope.executor(), todo.rule, input, cross) {
                Ok(input) => frontier.rows.push(input),
                Err(error) => {
                    frontier
                        .refused
                        .push(crate::domain::crossing::types::RefusedFrontier {
                            input: ryo_values::domain::plan_parts::types::Input::BoundObservation(
                                input.input,
                            ),
                            error,
                        })
                }
            }
        }
        let evaluated =
            cross_rule_with_inputs(crossing, &scope, todo, frontier.rows, &frontier.refused, cross)?;
        let refuse = |reason: &dyn std::fmt::Display| failed(&todo.id, reason);
        let roots = evaluated.roots;
        let mut unknowns = if let Some(error) = &roots.emit_error {
            crossing
                .paths
                .iter()
                .take(crossing.files.len())
                .map(|path| crate::domain::coverage::types::Unknown {
                    rule: todo.id.clone(),
                    path: path.clone(),
                    message: crate::domain::coverage::types::Message(error.to_string().into()),
                })
                .collect()
        } else {
            Vec::new()
        };
        let rendered = match self.render_emit(crossing, todo, plan, roots.emit, cross) {
            Ok(rendered) => rendered,
            Err(error) if !plan.observations.is_empty() || !plan.observation_inputs.is_empty() => {
                unknowns.extend(
                    crossing
                        .paths
                        .iter()
                        .take(crossing.files.len())
                        .map(|path| crate::domain::coverage::types::Unknown {
                            rule: todo.id.clone(),
                            path: path.clone(),
                            message: crate::domain::coverage::types::Message(
                                error.to_string().into(),
                            ),
                        }),
                );
                None
            }
            Err(error) => return Err(error),
        };
        let observations = if roots.observations.is_empty() {
            None
        } else {
            let super::types::PublicationSlot::Reserved(worker) = publication else {
                return Err(refuse(
                    &"demanded observation phase has no reserved publication arena",
                ));
            };
            Some(super::publication::pack_roots(
                roots.observations,
                cross,
                crossing.arenas,
                worker,
            ))
        };
        Ok(Crossed {
            unknowns,
            rendered,
            observations,
            fed: evaluated.fed,
        })
    }
    /// Place and render the actual diagnostic root; named-root production is independent.
    fn render_emit(
        &self,
        crossing: &CrossingInput<'_>,
        todo: &Todo,
        plan: &ryo_values::domain::plans::types::Plan,
        table: Option<crate::domain::table_model::types::Table>,
        cross: &mut Symbols,
    ) -> Result<Option<Rendered>, RulesError> {
        let refuse = |reason: &dyn std::fmt::Display| failed(&todo.id, reason);
        let rule_number = RuleNumber(u32::try_from(todo.rule.0).unwrap_or(u32::MAX));
        Ok(if let Some(table) = table {
            let mut found = founds(rule_number, &table, &plan.emit, FileId(u32::MAX))
                .map_err(|error| refuse(&error))?;
            found.sort_by_key(|each| each.file);
            let mut bytes = FileBytes::default();
            if placeable(plan, &table) == Placeability::Placeable {
                for group in found.chunk_by_mut(|left, right| left.file == right.file) {
                    let Some(entry) = group
                        .first()
                        .and_then(|first| file_in(crossing.files, first.file))
                    else {
                        continue;
                    };
                    let CorpusInput::Available(entry) = entry else {
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
            Some(rendered)
        } else {
            None
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
