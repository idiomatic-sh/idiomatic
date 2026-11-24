//! The project's rows kept for the scans (`kept`), and the fact source that answers them to one file's scan around the
//! language's (`HostedFacts`).

use super::types::{HostedCell, HostedPartial, HostedRows, HostedWriter, WrittenRow};
use ryo_language::domain::facts::projection::ColumnProjection;
use ryo_language::domain::facts::service::intern;
use ryo_language::domain::facts::types::{
    CellWord, FactAnswer, FactSource, FileWords, FillExtent, FilledColumn, FilledColumns,
    FillerSentence, PartialRows, Wanted,
};
use ryo_language::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_language::domain::symbols::service::text_of;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{
    CellKind, FileId, Missing, Relation, ReportText, Rows,
};
use ryo_relations::domain::terms::types::{NameText, Symbol};

/// What `HostedWriter` writes rows with, one `Rows` for each row type of the catalog, generated from its one list: each
/// row kept with its relation and its cells' kinds.
macro_rules! hosted_writers {
    (
        $( $group:ident { $( $variant:ident $module:ident $row:ident $column:ident ; )* } )*
    ) => {
        $( $(
            impl Rows<ryo_relations::domain::relations::collections::relations::$module::item::$row> for HostedWriter {
                fn write(&mut self, row: ryo_relations::domain::relations::collections::relations::$module::item::$row) {
                    use ryo_relations::domain::relations::collections::relations::interface::RelationsItem as _;
                    type Item = ryo_relations::domain::relations::collections::relations::$module::item::$variant;
                    let words = Item::words(row);
                    self.rows.push(WrittenRow {
                        relation: Item::relation(),
                        kinds: Item::kinds(),
                        words: words.as_ref().iter().map(|word| word.0).collect(),
                    });
                }

                fn partial(&mut self, cause: ReportText<'_>, missing: Missing) {
                    use ryo_relations::domain::relations::collections::relations::interface::RelationsItem as _;
                    type Item = ryo_relations::domain::relations::collections::relations::$module::item::$variant;
                    self.partials.push(HostedPartial { relation: Item::relation(), file: None, cause: cause.0.into(), missing });
                }

                fn partial_at(&mut self, file: FileId, cause: ReportText<'_>, missing: Missing) {
                    use ryo_relations::domain::relations::collections::relations::interface::RelationsItem as _;
                    type Item = ryo_relations::domain::relations::collections::relations::$module::item::$variant;
                    self.partials.push(HostedPartial { relation: Item::relation(), file: Some(file), cause: cause.0.into(), missing });
                }
            }
        )* )*
    };
}

ryo_relations::each_relation!(hosted_writers);

/// The rows a project reader wrote into `written`, kept for the scans: each by its relation and the file it is about (its
/// first file cell; a row with none is about no file and is dropped), its names given their texts from `symbols`, the
/// project's own. `held` are the relations the host answers whether or not a row was written.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::hosted::service::kept;
/// use ryo_execution_runtime::application::file_scan::hosted::types::HostedWriter;
/// use ryo_language::domain::symbols::types::Symbols;
/// use ryo_relations::domain::relations::types::Relation;
///
/// let rows = kept(HostedWriter::default(), &Symbols::default(), vec![Relation::ModuleIdentity]);
/// assert_eq!(rows.held, vec![Relation::ModuleIdentity]);
/// ```
#[must_use]
pub fn kept(written: HostedWriter, symbols: &Symbols, mut held: Vec<Relation>) -> HostedRows {
    held.sort_unstable();
    held.dedup();
    let mut rows = HostedRows {
        held,
        partials: written.partials,
        ..HostedRows::default()
    };
    for row in written.rows {
        let file = row
            .kinds
            .iter()
            .zip(&row.words)
            .find(|(kind, _)| **kind == CellKind::File)
            .map(|(_, word)| FileId(u32::try_from(*word).unwrap_or(u32::MAX)));
        let Some(file) = file else {
            continue;
        };
        let cells = row
            .kinds
            .iter()
            .zip(&row.words)
            .map(|(kind, word)| match kind {
                CellKind::Symbol => {
                    let symbol = Symbol(u32::try_from(*word).unwrap_or(u32::MAX));
                    let text = text_of(symbols, ByteSlice(&[]), symbol).map_or("", |text| text.0);
                    HostedCell::Text(text.into())
                }
                CellKind::Node | CellKind::File | CellKind::Span | CellKind::Number | CellKind::Flag => {
                    HostedCell::Word(*word)
                }
            })
            .collect();
        rows.rows.entry((row.relation, file)).or_default().push(cells);
    }
    rows
}

/// Whether `rows` hold a row about `file`: a file no language walks is scanned with no node table only then.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_scan::hosted::service::holds_file;
/// use ryo_execution_runtime::application::file_scan::hosted::types::HostedRows;
/// use ryo_relations::domain::relations::types::FileId;
///
/// assert!(!holds_file(&HostedRows::default(), FileId(0)));
/// ```
#[must_use]
pub fn holds_file(rows: &HostedRows, file: FileId) -> bool {
    rows.rows.keys().any(|(_, each)| *each == file)
}

/// The fact source of one file's scan: the held relations answered from the host's rows about the file, every other
/// relation the language's.
pub(crate) struct HostedFacts<'a> {
    /// The language's view of the file.
    pub(crate) language: &'a mut dyn FactSource,
    /// The host's rows.
    pub(crate) rows: &'a HostedRows,
    /// The file's number in the run.
    pub(crate) file: FileId,
}

impl HostedFacts<'_> {
    fn write(
        &mut self,
        relation: Relation,
        wanted: Wanted,
        words: &mut FileWords,
        out: &mut FilledColumns,
        mask: Option<&ColumnProjection>,
        count: &mut usize,
    ) -> FactAnswer {
        let extent = match wanted {
            Wanted::Whole => FillExtent::Asked,
            Wanted::Key(_, _) => FillExtent::Whole,
        };
        if self.rows.held.binary_search(&relation).is_err() {
            return if let Some(mask) = mask {
                    let (answer, rows) = self.language.fill_columns(relation, wanted, mask, words, out);
                    *count = rows;
                    answer
                } else { self.language.fill(relation, wanted, words, out) };
        }
        let partial = self.rows.partials.iter().find(|each| {
            each.relation == relation && each.file.is_none_or(|file| file == self.file)
        });
        if let Some(partial) = partial {
            return FactAnswer::Partial(PartialRows {
                cause: FillerSentence(partial.cause.clone()),
                missing: partial.missing,
            });
        }
        for row in self.rows.rows.get(&(relation, self.file)).into_iter().flatten() {
            if out.0.len() < row.len() {
                out.0.resize_with(row.len(), FilledColumn::default);
            }
            *count += 1;
            for (i, (column, cell)) in out.0.iter_mut().zip(row).enumerate() {
                if mask.is_some_and(|m| !m.contains(i)) { continue; }
                column.0.push(match cell {
                    HostedCell::Word(word) => CellWord(*word),
                    HostedCell::Text(text) => intern(words, NameText(text)),
                });
            }
        }
        FactAnswer::Answered(extent)
    }

}

impl FactSource for HostedFacts<'_> {
    fn fill(
        &mut self, relation: Relation, wanted: Wanted, words: &mut FileWords, out: &mut FilledColumns,
    ) -> FactAnswer {
        self.write(relation, wanted, words, out, None, &mut 0)
    }

    fn fill_columns(
        &mut self, relation: Relation, wanted: Wanted, mask: &ColumnProjection,
        words: &mut FileWords, out: &mut FilledColumns,
    ) -> (FactAnswer, usize) {
        out.clear_for(relation);
        let mut rows = 0;
        let answer = self.write(relation, wanted, words, out, Some(mask), &mut rows);
        (answer, rows)
    }

    fn render(&mut self, request: RenderRequest) -> RenderOutcome {
        self.language.render(request)
    }
}

/// The view of a file no language walks: no relation has a row, and no node is rendered.
pub(crate) struct NoLanguage;

impl FactSource for NoLanguage {
    fn fill(
        &mut self,
        _relation: Relation,
        wanted: Wanted,
        _words: &mut FileWords,
        _out: &mut FilledColumns,
    ) -> FactAnswer {
        FactAnswer::Answered(match wanted {
            Wanted::Whole => FillExtent::Asked,
            Wanted::Key(_, _) => FillExtent::Whole,
        })
    }

    fn fill_columns(
        &mut self, relation: Relation, wanted: Wanted, _mask: &ColumnProjection,
        words: &mut FileWords, out: &mut FilledColumns,
    ) -> (FactAnswer, usize) {
        out.clear_for(relation);
        (self.fill(relation, wanted, words, out), 0)
    }

    fn render(&mut self, _request: RenderRequest) -> RenderOutcome {
        RenderOutcome::NotApplicable
    }
}
