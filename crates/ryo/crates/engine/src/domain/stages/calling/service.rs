//! The services of the `calling` part: the `Call` stage. One loop for each meaning, over the batch: each row's argument
//! cells are read, the operators crate's meaning is asked, and the rows it gives are written as cells, as many a row as
//! the call has outputs, with for each row of the batch the stretch of the rows it gave. A meaning that gives several
//! rows for one (a source search, a capture) writes them all here, inside the batch's loop.

use crate::domain::code::types::Op;
use crate::domain::filled::errors::FilledError;
use crate::domain::filled::types::{FillRuns, MissingCode, UnbuiltFill};
use crate::domain::indexes::types::RowRun;
use crate::domain::stages::nodes::service::NO_WORD;
use crate::domain::stages::service::value_at;
use crate::domain::stages::types::{BatchValue, CallMeans, CallOut, FunctionTexts, SearchedRows};
use crate::domain::steps::calls::types::{SearchGroup, SearchLines, SourceSearch};
use crate::domain::steps::types::{CallMeaning, RegexId, StepRegex};
use ryo_language::domain::facts::service::{intern, text, words_end};
use ryo_language::domain::facts::types::{CellWord, FactSource, FileWords};
use ryo_language::domain::semantics::types::{RenderContext, RenderOutcome, RenderRequest};
use ryo_operators::domain::capture::service::matches;
use ryo_operators::domain::capture::types::{CaptureSlots, SlotPattern, SlotSubject};
use ryo_operators::domain::contract::types::{PortableOperatorFault, SourceByteLength};
use ryo_operators::domain::fits_rendered::service::{fits, line_prefix};
use ryo_operators::domain::fits_rendered::types::{
    CandidateText, Fit, LinePrefix, LineWidth, TabStop,
};
use ryo_operators::domain::functions::service::apply;
use ryo_operators::domain::functions::types::MadeText;
use ryo_operators::domain::patterns::service::compiled;
use ryo_operators::domain::replace::service::replaced_by;
use ryo_operators::domain::replace::types::{ReplacedText, ReplacementText};
use ryo_operators::domain::source_matches::service::{checked_source, searched};
use ryo_operators::domain::source_matches::types::{
    BaseLineColumn, CaptureRequest, LineBytes, SearchedSpan, SourceMatchesInput,
};
use ryo_operators::domain::span_between::service::between;
use ryo_operators::domain::span_bounds::service::bounds;
use ryo_operators::domain::table_texts::types::TextView;
use ryo_relations::domain::relations::service::{from_bits, to_bits};
use ryo_relations::domain::relations::types::{Bits, CellKind, Count, Integer, NodeId, Value};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Function};
use ryo_values::domain::query_operators::types::{LineOutput, TextOutput};
use ryo_values::domain::row_texts::types::CellText;

/// The first cell of the values a meaning is handed: a span.
#[inline]
fn first_column() -> ColumnIndex {
    ColumnIndex(u32::MIN)
}

/// A meaning's fault on a row, as the fill side hands it on.
fn faulted(fault: PortableOperatorFault) -> FilledError {
    FilledError::Meaning { fault }
}

/// The source of a regular expression the code numbers.
fn regex_at(regexes: &[StepRegex], regex: RegexId) -> Result<&StepRegex, FilledError> {
    regexes
        .get(usize::try_from(regex.0).unwrap_or(usize::MAX))
        .ok_or(FilledError::Code {
            named: MissingCode::Regex,
        })
}

/// The cell of the `at`-th argument for a row of the batch.
#[inline]
fn argument(arguments: &[BatchValue<'_>], at: Count, row: Count) -> CellWord {
    match arguments.get(at.0) {
        Some(value) => value_at(*value, row),
        None => CellWord(0),
    }
}

/// Says that the batch's next row gave `given` rows, the rows before it being `made`.
#[inline]
fn spanned(spans: &mut Vec<RowRun>, made: &mut Count, given: Count) {
    let end = Count(made.0.saturating_add(given.0));
    spans.push(RowRun { start: *made, end });
    *made = end;
}

/// `SpanBounds`: for each row its span's start and end, two numbers. One row for one.
fn span_bounds(
    arguments: &[BatchValue<'_>],
    rows: Count,
    out: CallOut<'_>,
) -> Result<(), FilledError> {
    let mut made = Count(0);
    for row in 0..rows.0 {
        let span = from_bits(
            CellKind::Span,
            Bits(argument(arguments, Count(0), Count(row)).0),
        );
        let ends = bounds(&[span], first_column()).map_err(faulted)?;
        out.answers
            .extend(ends.iter().map(|value| CellWord(to_bits(*value).0)));
        spanned(out.spans, &mut made, Count(1));
    }
    Ok(())
}

/// `SpanBetween`: for each row the span from its first span's start to its second's end, the absent span when either
/// is absent. One row for one.
fn span_between(
    means: &CallMeans<'_>,
    arguments: &[BatchValue<'_>],
    rows: Count,
    out: CallOut<'_>,
) -> Result<(), FilledError> {
    let length = SourceByteLength(means.source.0.len());
    let mut made = Count(0);
    for row in 0..rows.0 {
        let span = |at: usize| {
            from_bits(
                CellKind::Span,
                Bits(argument(arguments, Count(at), Count(row)).0),
            )
        };
        let ends = [span(0), span(1)];
        let joined = between(&ends, first_column(), ColumnIndex(1), length).map_err(faulted)?;
        out.answers.push(CellWord(to_bits(joined).0));
        spanned(out.spans, &mut made, Count(1));
    }
    Ok(())
}

/// `Concat` and `Lower`: for each row the text the function makes of its arguments' texts, as a word of the file. An
/// argument that is no text (a cell that is no word of the file) makes the answer the cell that is no word, which no
/// text equals. One row for one.
fn made_texts(
    function: Function,
    words: &mut FileWords,
    arguments: &[BatchValue<'_>],
    rows: Count,
    out: CallOut<'_>,
) {
    let FunctionTexts {
        arguments: texts,
        made: written,
    } = &mut out.scratch.texts;
    texts.resize_with(arguments.len(), MadeText::default);
    let mut made = Count(0);
    for row in 0..rows.0 {
        let end = words_end(words);
        let mut strangers = Count(0);
        for (at, held) in texts.iter_mut().enumerate() {
            let cell = argument(arguments, Count(at), Count(row));
            held.0.clear();
            if cell < end {
                held.0.push_str(text(words, cell).0);
            } else {
                strangers = Count(strangers.0.saturating_add(1));
            }
        }
        out.answers.push(if strangers.0 == 0 {
            apply(function, texts, written);
            intern(words, NameText(&written.0))
        } else {
            NO_WORD
        });
        spanned(out.spans, &mut made, Count(1));
    }
}

/// `Replace`: for each row its text with every occurrence of the expression replaced, as a word of the file. One row for
/// one. The expression is compiled once, before the first row.
fn replace(
    means: CallMeans<'_>,
    words: &mut FileWords,
    regex: RegexId,
    arguments: &[BatchValue<'_>],
    rows: Count,
    out: CallOut<'_>,
) -> Result<(), FilledError> {
    let pattern = regex_at(means.regexes, regex)?;
    let expression = compiled(TextView(&pattern.0), means.compiled, means.matcher)
        .map_err(|invalid| faulted(invalid.into()))?;
    let scratch = out.scratch;
    let mut made = Count(0);
    for row in 0..rows.0 {
        let subject = text(words, argument(arguments, Count(0), Count(row)));
        let replacement = text(words, argument(arguments, Count(1), Count(row)));
        replaced_by(
            expression,
            CellText(subject.0),
            ReplacementText(replacement.0),
            means.matcher,
            ReplacedText(&mut scratch.written.0),
        )
        .map_err(faulted)?;
        out.answers
            .push(intern(words, NameText(&scratch.written.0)));
        spanned(out.spans, &mut made, Count(1));
    }
    Ok(())
}

/// `Capture`: for each row one row for each way the slot pattern matches the whole path, each the texts of the
/// pattern's slots in slot order, as words of the file.
fn capture(
    words: &mut FileWords,
    arguments: &[BatchValue<'_>],
    rows: Count,
    out: CallOut<'_>,
) -> Result<(), FilledError> {
    let scratch = out.scratch;
    let mut made = Count(0);
    for row in 0..rows.0 {
        scratch.text.0.clear();
        scratch
            .text
            .0
            .push_str(text(words, argument(arguments, Count(0), Count(row))).0);
        scratch.slots.0.clear();
        scratch.ways.ranges.0.clear();
        scratch.ways.ways = Count(0);
        matches(
            SlotPattern(text(words, argument(arguments, Count(1), Count(row))).0),
            SlotSubject(&scratch.text.0),
            SlotSubject(&scratch.text.0),
            CaptureSlots(&mut scratch.slots.0),
            &mut scratch.ways,
        );
        for range in &scratch.ways.ranges.0 {
            let slot = scratch.text.0.get(range.clone()).unwrap_or_default();
            out.answers.push(intern(words, NameText(slot)));
        }
        spanned(out.spans, &mut made, scratch.ways.ways);
    }
    Ok(())
}

/// `SourceMatches`: for each row one row for each occurrence of the expression in the text its span covers, left to
/// right: the occurrence's span, its line when lines are numbered, and for a named group its span, its text and its line
/// as asked. An absent span gives no row.
fn source_matches(
    means: CallMeans<'_>,
    words: &mut FileWords,
    regex: RegexId,
    search: SourceSearch,
    arguments: &[BatchValue<'_>],
    rows: Count,
    out: CallOut<'_>,
) -> Result<(), FilledError> {
    let pattern = regex_at(means.regexes, regex)?;
    let expression = compiled(TextView(&pattern.0), means.compiled, means.matcher)
        .map_err(|invalid| faulted(invalid.into()))?;
    let scratch = out.scratch;
    let base_line = match search.lines {
        SearchLines::Unnumbered => BaseLineColumn::Unnumbered,
        SearchLines::Numbered => {
            BaseLineColumn::At(ColumnIndex(first_column().0.saturating_add(1)))
        }
    };
    let (named, capture_text, capture_line) = match search.group {
        SearchGroup::Whole => (None, TextOutput::Omitted, LineOutput::Omitted),
        SearchGroup::Named { text, line } => {
            (Some(Count(arguments.len().saturating_sub(1))), text, line)
        }
    };
    let mut made = Count(0);
    for row in 0..rows.0 {
        scratch.values.clear();
        scratch.values.push(from_bits(
            CellKind::Span,
            Bits(argument(arguments, Count(0), Count(row)).0),
        ));
        if search.lines == SearchLines::Numbered {
            scratch.values.push(from_bits(
                CellKind::Number,
                Bits(argument(arguments, Count(1), Count(row)).0),
            ));
        }
        scratch.constant.0.clear();
        if let Some(name) = named {
            scratch
                .constant
                .0
                .push_str(text(words, argument(arguments, name, Count(row))).0);
        }
        let checked = match checked_source(
            &scratch.values,
            first_column(),
            base_line,
            SourceView(means.source.0),
        )
        .map_err(faulted)?
        {
            SearchedSpan::Checked(checked) => checked,
            SearchedSpan::Absent => {
                spanned(out.spans, &mut made, Count(0));
                continue;
            }
        };
        let mut driver = SearchedRows {
            words: &mut *words,
            answers: &mut *out.answers,
            rows: Count(0),
        };
        searched(
            SourceMatchesInput {
                row: &[],
                source: LineBytes(means.source.0),
                base: checked.base,
                text: checked.text,
                line: checked.line,
                regex: expression,
                capture: match named {
                    Some(_) => CaptureRequest::Group(CellText(&scratch.constant.0)),
                    None => CaptureRequest::Whole,
                },
                capture_text,
                capture_line,
            },
            means.matcher,
            &mut driver,
            &mut scratch.joined,
        )
        .map_err(faulted)?;
        let given = driver.rows;
        spanned(out.spans, &mut made, given);
    }
    Ok(())
}

/// `FitsRendered`: for each row its candidate, the n + 1 literals interleaved with its n texts, after the source line's
/// text before its span; the row goes on when every line of it stays within the widest line, and gives no cell. A text
/// cell that is no word of the file faults, as today's operator does on a cell that is no text.
fn fits_rendered(
    means: &CallMeans<'_>,
    words: &FileWords,
    arguments: &[BatchValue<'_>],
    rows: Count,
    out: CallOut<'_>,
) -> Result<(), FilledError> {
    // 2n + 4 arguments: the n texts, the span, the n + 1 literals, the widest line, the tab stop.
    let texts = arguments.len().saturating_sub(4) / 2;
    let span_at = Count(texts);
    let number = |at: usize, row: usize| match from_bits(
        CellKind::Number,
        Bits(argument(arguments, Count(at), Count(row)).0),
    ) {
        Value::Number(Integer(number)) => Ok(number),
        _ => Err(faulted(PortableOperatorFault::WrongBinding)),
    };
    let scratch = out.scratch;
    let mut made = Count(0);
    for row in 0..rows.0 {
        let end = words_end(words);
        let span = from_bits(
            CellKind::Span,
            Bits(argument(arguments, span_at, Count(row)).0),
        );
        let LinePrefix(before) = line_prefix(SourceView(means.source.0), span).map_err(faulted)?;
        scratch.text.0.clear();
        scratch.text.0.push_str(before);
        scratch.written.0.clear();
        for at in 0..texts {
            let literal = argument(
                arguments,
                Count(texts.saturating_add(1).saturating_add(at)),
                Count(row),
            );
            scratch.written.0.push_str(text(words, literal).0);
            let cell = argument(arguments, Count(at), Count(row));
            if cell >= end {
                return Err(faulted(PortableOperatorFault::ShapeMismatch));
            }
            scratch.written.0.push_str(text(words, cell).0);
        }
        let last = argument(
            arguments,
            Count(texts.saturating_mul(2).saturating_add(1)),
            Count(row),
        );
        scratch.written.0.push_str(text(words, last).0);
        let widest = number(texts.saturating_mul(2).saturating_add(2), row)?;
        let tab = number(texts.saturating_mul(2).saturating_add(3), row)?;
        let maximum =
            usize::try_from(widest).map_err(|_| faulted(PortableOperatorFault::WrongBinding))?;
        let stop =
            usize::try_from(tab).map_err(|_| faulted(PortableOperatorFault::WrongBinding))?;
        if stop == 0 {
            return Err(faulted(PortableOperatorFault::WrongBinding));
        }
        let fit = fits(
            CandidateText(&scratch.written.0),
            LinePrefix(&scratch.text.0),
            LineWidth(maximum),
            TabStop(stop),
        )
        .map_err(faulted)?;
        let given = match fit {
            Fit::Within => Count(1),
            Fit::Over => Count(0),
        };
        spanned(out.spans, &mut made, given);
    }
    Ok(())
}

/// A rendering: the language's source form of the node `handle` names, in `context`, as a word of the file; none when the
/// node has none in the role. Today's operator refuses the batch an unsupported rendering is in: here that row faults.
///
/// # Errors
///
/// `UnsupportedRender` when the language cannot prove the node's text in the role.
pub(crate) fn rendered(
    facts: &mut dyn FactSource,
    words: &mut FileWords,
    handle: CellWord,
    context: RenderContext,
) -> Result<Option<CellWord>, FilledError> {
    match facts.render(RenderRequest {
        node: NodeId(handle.0),
        context,
    }) {
        RenderOutcome::Known(text) => Ok(Some(intern(words, NameText(&text.0)))),
        RenderOutcome::NotApplicable => Ok(None),
        RenderOutcome::Unsupported(_) => Err(faulted(PortableOperatorFault::UnsupportedRender)),
    }
}

/// Whether a `Call` runs a meaning: every meaning a rule of the packs is lowered to; `Words` and
/// `Rank` are written by no rule. `call_batch` asks it before it runs one, so a meaning said to run here and not built
/// there fails its first case.
#[must_use]
pub(crate) const fn call_runs(meaning: CallMeaning) -> FillRuns {
    match meaning {
        CallMeaning::SpanBetween
        | CallMeaning::SpanBounds
        | CallMeaning::Replace(_)
        | CallMeaning::Capture
        | CallMeaning::SourceMatches(..)
        | CallMeaning::Concat
        | CallMeaning::Lower
        | CallMeaning::FitsRendered => FillRuns::Runs,
        CallMeaning::Words | CallMeaning::Rank => FillRuns::NotYet,
    }
}

/// `Call`: the meaning asked for every row of the batch. After it `out.answers` holds the cells of the rows given, row by
/// row, and `out.spans` for each row of the batch the stretch of those rows it gave.
///
/// # Errors
///
/// The meaning faulted on a row; the code names an expression it does not hold; or the meaning is one the fill side
/// does not run yet.
pub(crate) fn call_batch(
    means: CallMeans<'_>,
    words: &mut FileWords,
    meaning: CallMeaning,
    arguments: &[BatchValue<'_>],
    rows: Count,
    out: CallOut<'_>,
) -> Result<(), FilledError> {
    out.answers.clear();
    out.spans.clear();
    if call_runs(meaning) == FillRuns::NotYet {
        return Err(FilledError::Unbuilt {
            unbuilt: UnbuiltFill::Stage(Op::StageCall),
        });
    }
    match meaning {
        CallMeaning::SpanBetween => span_between(&means, arguments, rows, out),
        CallMeaning::SpanBounds => span_bounds(arguments, rows, out),
        CallMeaning::Concat => {
            made_texts(Function::Concat, words, arguments, rows, out);
            Ok(())
        }
        CallMeaning::Lower => {
            made_texts(Function::Lower, words, arguments, rows, out);
            Ok(())
        }
        CallMeaning::Replace(regex) => replace(means, words, regex, arguments, rows, out),
        CallMeaning::Capture => capture(words, arguments, rows, out),
        CallMeaning::SourceMatches(regex, search) => {
            source_matches(means, words, regex, search, arguments, rows, out)
        }
        CallMeaning::FitsRendered => fits_rendered(&means, words, arguments, rows, out),
        CallMeaning::Words | CallMeaning::Rank => Err(FilledError::Unbuilt {
            unbuilt: UnbuiltFill::Stage(Op::StageCall),
        }),
    }
}
