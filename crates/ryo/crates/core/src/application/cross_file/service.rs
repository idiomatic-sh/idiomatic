//! Cross-file evaluation and bounded source rendering over injected ports.
use crate::application::run_coordination::crossing::cross_rule_with_inputs;
use crate::application::run_coordination::settings::CrossingInput;
use crate::domain::answering::types::{Crossed, Todo};
use crate::domain::crossing::service::placeable;
use crate::domain::crossing::types::Placeability;
use crate::domain::finish::errors::FinishError;
use crate::domain::finish::service::{
    column_cell, founds, line_cell, place_all, shown_cell, source_of,
};
use crate::domain::ordering::service::{order_alike, sort_findings};
use crate::domain::render::service::{end_row, key_count, next_row};
use crate::domain::render::types::{
    AnswerCell, AnswerColumns, Answering, KeyCount, Rendered, RowNumber, RowOrder,
};
use crate::domain::reporting::service::order_found;
use crate::domain::rules::service::failed;
use crate::outbound::corpus::interface::CorpusPort;
use crate::outbound::publication::interface::PublicationPort;
use ryo_execution::inbound::use_cases::execution::interface::{
    ExecutionRequest, PreparedExecution,
};
use ryo_execution::inbound::use_cases::execution::types::{
    BoundInput, PreparedFrontiers, ProjectScope, PublicationSlot,
};
use ryo_execution::inbound::use_cases::metadata::interface::MetadataUseCase;
use ryo_execution::inbound::use_cases::metadata::interface::{Governing, Mask};
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_language::domain::symbols::service::file_in;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{FileId, Value};
use ryo_values::domain::files::types::Hits;
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::files::types::{CorpusInput, FileBytes};
use ryo_values::domain::records::service::keep;
use ryo_values::domain::records::types::RenderedText;
use ryo_values::domain::records::types::RowTotal;
use ryo_values::domain::records::types::{Cell, CellText, TextRange};
use ryo_values::domain::rows::service::cell;
use ryo_values::domain::rule_requests::errors::RulesError;
use {
    crate::domain::finish::types::Arenas, crate::domain::finish::types::FoundRow,
    crate::domain::finish::types::HeldFile, crate::domain::finish::types::Loaded,
    crate::domain::finish::types::Resolving, ryo_values::domain::runs::types::RuleNumber,
};

use std::sync::Arc;
/// Cross-file work sharing the caller's corpus resource.
pub struct CrossFile {
    /// Reads back the files the findings name.
    corpus: Arc<dyn CorpusPort>,
    publication: Arc<dyn PublicationPort>,
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
    pub fn new(corpus: Arc<dyn CorpusPort>, publication: Arc<dyn PublicationPort>) -> Self {
        Self {
            corpus,
            publication,
        }
    }
    /// Evaluate, place and render one cross-file rule without publishing it.
    /// Diagnostic-only entry; demanded observation phases require the explicit reserved-slot entry.
    pub(crate) fn evaluate(
        &self,
        crossing: &CrossingInput<'_>,
        todo: &Todo,
        frontier: PreparedFrontiers,
        cross: &mut Symbols,
    ) -> Result<Crossed, RulesError> {
        let scope = crossing
            .services
            .project_scope(
                todo.rule,
                crossing.files,
                crossing.covered,
                crossing.skipped,
            )
            .map_err(|error| failed(&todo.id, &error))?;
        self.evaluate_phase(
            crossing,
            todo,
            frontier,
            cross,
            &[],
            PublicationSlot::Unused,
            &scope,
        )
        .map(|result| result.crossed)
    }
    pub(crate) fn evaluate_phase(
        &self,
        crossing: &CrossingInput<'_>,
        todo: &Todo,
        frontier: PreparedFrontiers,
        cross: &mut Symbols,
        bound_inputs: &[BoundInput],
        publication: PublicationSlot,
        scope: &ProjectScope,
    ) -> Result<super::types::CrossedWithPublications, RulesError> {
        let emit = crossing
            .program
            .emit(todo.rule)
            .map_err(|error| failed(&todo.id, &error))?;
        let evaluated =
            cross_rule_with_inputs(crossing, todo, frontier, bound_inputs, cross, scope)?;
        let refuse = |reason: &dyn std::fmt::Display| failed(&todo.id, reason);
        let roots = evaluated.roots;
        if let Some(error) = &roots.emit_error
            && crossing.files.is_empty()
        {
            return Err(refuse(error));
        }
        let mut unknowns = if let Some(error) = &roots.emit_error {
            crossing
                .paths
                .iter()
                .take(crossing.files.len())
                .map(
                    |path| ryo_execution::inbound::use_cases::metadata::interface::Unknown {
                        rule: todo.id.clone(),
                        path: path.clone(),
                        message: ryo_execution::inbound::use_cases::metadata::interface::Message(
                            error.to_string().into(),
                        ),
                    },
                )
                .collect()
        } else {
            Vec::new()
        };
        if let Some(affected) = &roots.emit_affected {
            for index in MetadataUseCaseImpl
                .affected_ordinals(
                    ryo_execution::inbound::use_cases::metadata::types::AffectedFiles {
                        affected,
                        file_count:
                            ryo_execution::inbound::use_cases::metadata::types::CoverageFileCount(
                                crossing.files.len(),
                            ),
                    },
                )
                .map_err(|error| refuse(&error))?
                .0
            {
                let path = crossing.paths.get(index).ok_or_else(|| {
                    refuse(&ryo_execution::inbound::use_cases::execution::types::OperatingError::NotRun)
                })?;
                unknowns.push(
                    ryo_execution::inbound::use_cases::metadata::interface::Unknown {
                        rule: todo.id.clone(),
                        path: path.clone(),
                        message: ryo_execution::inbound::use_cases::metadata::interface::Message(
                            "physical source facts are incomplete".into(),
                        ),
                    },
                );
            }
        }
        let rendered = match self.render_emit(crossing, todo, emit, roots.emit, cross) {
            Ok(rendered) => rendered,
            Err(error)
                if crossing
                    .program
                    .has_observation_contract(todo.rule)
                    .map_err(|reason| failed(&todo.id, &reason))? =>
            {
                unknowns.extend(
                    crossing
                        .paths
                        .iter()
                        .take(crossing.files.len())
                        .map(|path| {
                            ryo_execution::inbound::use_cases::metadata::interface::Unknown {
                                rule: todo.id.clone(),
                                path: path.clone(),
                                message:
                                    ryo_execution::inbound::use_cases::metadata::interface::Message(
                                        error.to_string().into(),
                                    ),
                            }
                        }),
                );
                None
            }
            Err(error) => return Err(error),
        };
        let observations = if roots.observations.is_empty() {
            None
        } else {
            let PublicationSlot::Reserved(worker) = publication else {
                return Err(refuse(
                    &"demanded observation phase has no reserved publication arena",
                ));
            };
            Some(super::publication::pack_roots(
                roots.observations,
                cross,
                crossing.arenas,
                worker,
                self.publication.as_ref(),
            ))
        };
        let mut publications = evaluated.publications;
        if let Some(packed) = &observations {
            publications.retain(|(output, _)| {
                packed.roots.iter().any(|root| {
                    root.output == *output
                        && root.readiness
                            == ryo_values::domain::observations::types::ProducerReadiness::Complete
                })
            });
        } else {
            publications.clear();
        }
        Ok(super::types::CrossedWithPublications {
            crossed: Crossed {
                unknowns,
                rendered,
                observations,
                fed: evaluated.fed,
            },
            publications,
        })
    }
    /// Place and render the actual diagnostic root; named-root production is independent.
    fn render_emit(
        &self,
        crossing: &CrossingInput<'_>,
        todo: &Todo,
        emit: &ryo_values::domain::plans::types::Emit,
        table: Option<ryo_values::domain::rows::types::Table>,
        cross: &mut Symbols,
    ) -> Result<Option<Rendered>, RulesError> {
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
                let Some(path) = usize::try_from(each.file.0)
                    .ok()
                    .filter(|index| *index < crossing.files.len())
                    .and_then(|index| crossing.paths.get(index))
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
    pub(super) fn show_value(
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
