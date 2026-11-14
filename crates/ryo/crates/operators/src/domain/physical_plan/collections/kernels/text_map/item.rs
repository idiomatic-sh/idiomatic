//! The `text_map` item of the `kernels` collection: a pipeline stage that runs each row through one text kind's per-row
//! meaning, in row order, stopping at the first row that faults; the kind's checks, its pure function and its cell reads
//! are its concept's, the row loop and the engine's resources are the kernel's.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use super::super::scan_facts::item::{count_of, kinds_of, row_of};
use crate::domain::capture::types::{CaptureSlots, SlotPattern, SlotPlaces, SlotSubject};
use crate::domain::contract::types::{
    PortableOperatorFault, RangeScratch, RowScratch, SourceByteLength, TextFound, TextScratch,
};
use crate::domain::fits_rendered::types::Fit;
use crate::domain::patterns::types::PatternMatcher;
#[cfg(test)]
use crate::domain::physical_plan::types::Lowering;
use crate::domain::physical_plan::types::file_buffers::value::NodeDecisions;
use crate::domain::physical_plan::types::{InputRows, Resources, RowMap};
use crate::domain::rank::types::RankLayers;
use crate::domain::replace::types::{ReplacedText, ReplacementText};
use crate::domain::source_matches::types::{
    CaptureRequest, DeferredInterning, LineBytes, SearchedGroup, SearchedSpan, SourceMatchesInput,
    SourceMatchesParts,
};
use crate::domain::table_texts::service::datum;
use crate::domain::table_texts::types::{Datum, TextView};
use crate::domain::words::types::WordPlaces;
use ryo_matching::inbound::use_cases::matching::types::{GroupName, SearchText};
use ryo_relations::domain::relations::types::{CellKind, Count, Value};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// `text_map`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct TextMapItem;

#[cfg(test)]
impl KernelsItem for TextMapItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        if matches!(lowering, Lowering::TextMap { .. }) {
            Reach::Runs
        } else {
            Reach::Skips
        }
    }
}

/// What one `text_map` stage borrows for a file: the engine's resources, the matching semantics, the node's compiled
/// patterns, the text scratch of the worker and the table it writes.
pub(crate) struct RowMapRun<'r> {
    /// The engine's resources: the file's texts and bytes, and the symbols a kind interns or mints.
    pub(crate) resources: &'r mut dyn Resources,
    /// The matching semantics a pattern compiles and matches through.
    pub(crate) matcher: PatternMatcher<'r>,
    /// The node's compiled patterns, kept across files.
    pub(crate) decisions: &'r mut NodeDecisions,
    /// The worker's row and text buffers, their capacity kept across files.
    pub(crate) scratch: &'r mut RowScratch,
    /// The places of the texts a row's slots or words are, minted once they are found.
    pub(crate) places: &'r mut RangeScratch,
    /// The rows the stage writes.
    pub(crate) output: &'r mut Table,
}

/// Runs every row of `input` through `map` into `run.output`, and answers the fault of the first row that raised one: the
/// rows written before it stay.
pub(crate) fn mapped(
    map: &RowMap,
    input: InputRows<'_>,
    run: &mut RowMapRun<'_>,
) -> Result<Option<PortableOperatorFault>, PortableOperatorFault> {
    run.output.kinds = kinds_of(input).to_vec();
    run.output.kinds.extend(appended_kinds(map));
    for row in 0..count_of(input).0 {
        let mut decode = std::mem::take(&mut run.scratch.other);
        let cells = row_of(input, RowIndex(row), &mut decode);
        let mapped_row = row_mapped(map, cells, run);
        run.scratch.other = decode;
        if let Err(fault) = mapped_row {
            return Ok(Some(fault));
        }
    }
    Ok(None)
}

/// The kinds of the columns `map` appends to each row.
fn appended_kinds(map: &RowMap) -> Vec<CellKind> {
    match map {
        RowMap::Capture(capture) => {
            vec![
                CellKind::Symbol;
                capture
                    .pattern
                    .0
                    .bytes()
                    .filter(|byte| *byte == b'{')
                    .count()
            ]
        }
        RowMap::Replace(_) | RowMap::Words(_) => vec![CellKind::Symbol],
        RowMap::SourceMatches(search) => {
            crate::domain::source_matches::service::appended_kinds(search)
        }
        RowMap::SpanBounds(_) => vec![CellKind::Number, CellKind::Number],
        RowMap::SpanBetween(_) => vec![CellKind::Span],
        RowMap::Rank(_) => vec![CellKind::Number],
        RowMap::FitsRendered(_) => Vec::new(),
    }
}

/// Writes `row` followed by `appended` as a row of `output`.
fn written(output: &mut Table, row: &[Value], appended: &[Value]) {
    output.cells.extend_from_slice(row);
    output.cells.extend_from_slice(appended);
    output.count = RowCount(output.count.0 + 1);
}

/// The cell of `row` at `column`: `NoSuchColumn` past the row.
fn cell(row: &[Value], column: ColumnIndex) -> Result<Value, PortableOperatorFault> {
    ryo_values::domain::rows::service::column_in(row, column)
        .copied()
        .ok_or(PortableOperatorFault::NoSuchColumn { index: column })
}

/// Copies the text of `value` into `into`, and answers whether it has one.
fn text_into(resources: &dyn Resources, value: Value, into: &mut TextScratch) -> TextFound {
    into.0.clear();
    match datum(value, &resources.texts()) {
        Datum::Text(TextView(text)) => {
            into.0.push_str(text);
            TextFound::Text
        }
        Datum::Unlocated
        | Datum::Number(_)
        | Datum::Flag(_)
        | Datum::Numbers(_)
        | Datum::Texts(_) => TextFound::NotText,
    }
}

/// Runs one row through `map`, writing the rows it gives.
///
/// # Errors
///
/// The row's fault at its first failing check, in the kind's order.
fn row_mapped(
    map: &RowMap,
    row: &[Value],
    run: &mut RowMapRun<'_>,
) -> Result<(), PortableOperatorFault> {
    match map {
        RowMap::SpanBounds(bounds) => {
            let appended = crate::domain::span_bounds::service::bounds(row, bounds.span)?;
            written(run.output, row, &appended);
        }
        RowMap::SpanBetween(between) => {
            let length = SourceByteLength(run.resources.source().0.len());
            let appended = crate::domain::span_between::service::between(
                row,
                between.start,
                between.end,
                length,
            )?;
            written(run.output, row, &[appended]);
        }
        RowMap::Rank(rank) => {
            let layers = match &rank.layers {
                RankLayers::Bound(layers) => layers,
                RankLayers::Refused(fault) => return Err(fault.clone()),
            };
            let value = cell(row, rank.column)?;
            if text_into(run.resources, value, &mut run.scratch.text) == TextFound::NotText {
                return Ok(());
            }
            if let Some(found) =
                crate::domain::rank::service::rank(CellText(&run.scratch.text.0), layers)
            {
                written(run.output, row, &[Value::Number(found)]);
            }
        }
        RowMap::Words(words) => {
            let value = cell(row, words.text)?;
            let RowScratch {
                text,
                word,
                replaced,
                ..
            } = &mut *run.scratch;
            replaced.0.clear();
            if text_into(run.resources, value, text) == TextFound::NotText {
                return Ok(());
            }
            run.places.0.clear();
            crate::domain::words::service::split(
                CellText(&text.0),
                crate::domain::words::types::WordScratch(&mut word.0),
                &mut WordPlaces {
                    words: replaced,
                    places: run.places,
                },
            );
            for place in &run.places.0 {
                let made = run
                    .resources
                    .mint(CellText(replaced.0.get(place.clone()).unwrap_or_default()));
                written(run.output, row, &[made]);
            }
        }
        RowMap::Capture(capture) => {
            let value = cell(row, capture.path)?;
            let RowScratch {
                text,
                captured,
                slots,
                ..
            } = &mut *run.scratch;
            if text_into(run.resources, value, text) == TextFound::NotText {
                return Ok(());
            }
            captured.clear();
            slots.0.clear();
            let width = capture
                .pattern
                .0
                .bytes()
                .filter(|byte| *byte == b'{')
                .count();
            run.places.0.clear();
            let mut minting = SlotPlaces {
                places: run.places,
                ways: Count(0),
            };
            crate::domain::capture::service::matches(
                SlotPattern(&capture.pattern.0),
                SlotSubject(&text.0),
                SlotSubject(&text.0),
                CaptureSlots(&mut slots.0),
                &mut minting,
            );
            for place in &minting.places.0 {
                captured.push(
                    run.resources
                        .mint(CellText(text.0.get(place.clone()).unwrap_or_default())),
                );
            }
            let ways = minting.ways;
            for way in 0..ways.0 {
                written(
                    run.output,
                    row,
                    captured
                        .get(way * width..(way + 1) * width)
                        .unwrap_or_default(),
                );
            }
        }
        RowMap::Replace(replace) => {
            let value = cell(row, replace.text)?;
            if text_into(run.resources, value, &mut run.scratch.text) == TextFound::NotText {
                return Err(PortableOperatorFault::ShapeMismatch);
            }
            let regex = crate::domain::patterns::service::compiled(
                TextView(&replace.pattern.0),
                &mut run.decisions.select.compiled_regexes,
                run.matcher,
            )?;
            let RowScratch { text, replaced, .. } = &mut *run.scratch;
            let matcher = run.matcher;
            crate::domain::replace::service::replace_compiled(
                &|each| {
                    matcher
                        .0
                        .visit_regex_matches(regex, SearchText(&text.0), &mut |found| each(found.0))
                },
                CellText(&text.0),
                ReplacementText(&replace.replacement.0),
                ReplacedText(&mut replaced.0),
            )?;
            let appended = run.resources.mint(CellText(&replaced.0));
            written(run.output, row, &[appended]);
        }
        RowMap::SourceMatches(search) => searched(search, row, run)?,
        RowMap::FitsRendered(fits) => {
            let RowScratch { text, replaced, .. } = &mut *run.scratch;
            let resources: &dyn Resources = &*run.resources;
            let source = SourceView(resources.source().0);
            let fit = crate::domain::fits_rendered::service::fitted(
                fits,
                row,
                source,
                &resources.texts(),
                text,
                replaced,
            )?;
            if fit == Fit::Within {
                written(run.output, row, &[]);
            }
        }
    }
    Ok(())
}

/// Searches one row's span: its refusal, the gate, the pattern, then the occurrences, each captured text interned once the
/// search is done, in occurrence order; the row's fault at its first failing check.
fn searched(
    search: &SourceMatchesParts,
    row: &[Value],
    run: &mut RowMapRun<'_>,
) -> Result<(), PortableOperatorFault> {
    crate::domain::source_matches::service::refused(search)?;
    let first_row = run.output.count;
    let RowScratch {
        row: joined,
        replaced: interned,
        slots: pending,
        ..
    } = &mut *run.scratch;
    interned.0.clear();
    pending.0.clear();
    {
        let resources: &dyn Resources = &*run.resources;
        let source = SourceView(resources.source().0);
        let SearchedSpan::Checked(checked) =
            crate::domain::source_matches::service::checked_source(
                row,
                search.span,
                search.base_line,
                source,
            )?
        else {
            return Ok(());
        };
        let regex = crate::domain::patterns::service::compiled(
            TextView(&search.pattern.0),
            &mut run.decisions.select.compiled_regexes,
            run.matcher,
        )?;
        let matcher = run.matcher;
        let text = checked.text.0;
        let capture = match &search.capture {
            SearchedGroup::Whole => CaptureRequest::Whole,
            SearchedGroup::Group(name) => CaptureRequest::Group(CellText(&name.0)),
        };
        let mut deferred = DeferredInterning {
            output: &mut *run.output,
            interned,
            pending,
        };
        crate::domain::source_matches::service::occurrences(
            SourceMatchesInput {
                row,
                source: LineBytes(source.0),
                base: checked.base,
                text: checked.text,
                line: checked.line,
                regex,
                capture,
                capture_text: search.capture_text,
                capture_line: search.capture_line,
            },
            &mut deferred,
            joined,
            &|each| match &search.capture {
                SearchedGroup::Group(name) => matcher
                    .0
                    .visit_regex_captures(
                        regex,
                        SearchText(text),
                        GroupName(&name.0),
                        &mut |found, group| each(found.0, Some(group.0)),
                    )
                    .map_err(|_| PortableOperatorFault::MissingCapture {
                        capture: name.clone(),
                    }),
                SearchedGroup::Whole => {
                    Ok(matcher
                        .0
                        .visit_regex_matches(regex, SearchText(text), &mut |found| {
                            each(found.0, None)
                        }))
                }
            },
        )?;
    }
    crate::domain::source_matches::service::interned_captures(
        search,
        row,
        first_row,
        interned,
        pending,
        run.output,
        &mut |found| run.resources.intern(found),
    );
    Ok(())
}
