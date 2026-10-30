//! The `words` operator: split a text column into words and append each word, one row for each.

use ryo_language::domain::relations::types::{CellKind, Value};
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::terms::types::NameText;

use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::first;
use crate::domain::operating::service::minted;
use crate::domain::operating::types::AfterTheFiles;
use crate::domain::operating::types::{Emit, Feed, InputPlace, OperatorContext, Room};
use crate::domain::replay::service::derive;
use crate::domain::replay::types::{Collect, Pipe};
use crate::domain::table_model::service::{cell, count_of, kinds_of, positions, read_row, reset};
use crate::domain::table_model::types::{Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_scratch::types::ScratchWord;
use crate::domain::table_texts::service::datum;
use crate::domain::table_texts::types::{Datum, TextView, Texts};
use ryo_values::domain::plans::types::Operation;

/// `words`, one item of the `operators` collection.
pub(crate) struct WordsItem;

/// Gives `each` the words of `text`: runs of letters and digits, split where a lower case letter or a
/// digit meets an upper case one, in lower case. `_`, `-`, spaces and punctuation separate words.
fn split(text: TextView<'_>, word: &mut ScratchWord, mut each: impl FnMut(TextView<'_>)) {
    word.clear();
    let mut before: Option<char> = None;
    for character in text.0.chars() {
        if !character.is_alphanumeric() {
            if !word.is_empty() {
                each(TextView(word.as_str()));
                word.clear();
            }
            before = None;
            continue;
        }
        if character.is_uppercase()
            && before.is_some_and(|previous| previous.is_lowercase() || previous.is_ascii_digit())
            && !word.is_empty()
        {
            each(TextView(word.as_str()));
            word.clear();
        }
        word.extend(character.to_lowercase());
        before = Some(character);
    }
    if !word.is_empty() {
        each(TextView(word.as_str()));
        word.clear();
    }
}

/// Passes on the rows that `row` makes: it with each word of its text, one row for each.
fn words_row(
    operation: &Operation,
    row: &[Value],
    context: &mut OperatorContext<'_>,
    joined: &mut Vec<Value>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    let Operation::Words { text, .. } = operation else {
        return Err(OperatingError::WrongOperation);
    };
    {
        let OperatorContext {
            symbols,
            source,
            paths,
            arenas,
            scratch,
            ..
        } = &mut *context;
        let after = if arenas.is_empty() {
            AfterTheFiles::Before
        } else {
            AfterTheFiles::After
        };
        let OperatorScratch {
            text: holder,
            word,
            captured,
            ..
        } = &mut **scratch;
        holder.clear();
        captured.clear();
        let texts = Texts {
            symbols,
            source: *source,
            paths,
            arenas,
        };
        let Datum::Text(read) = datum(cell(row, *text)?, &texts) else {
            return Ok(());
        };
        holder.push_str(read.0);
        split(TextView(holder), word, |each| {
            let made = intern_derived(symbols, ByteSlice(source.0), NameText(each.0));
            captured.push(minted(symbols, made, after));
        });
    }
    for place in 0..context.scratch.captured.len() {
        let Some(&word) = context.scratch.captured.get(place) else {
            break;
        };
        joined.clear();
        joined.extend_from_slice(row);
        joined.push(word);
        emit.row(context, joined)?;
    }
    Ok(())
}

impl OperatorsItem for WordsItem {
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let rows = first(inputs)?;
        reset(out, kinds_of(rows));
        out.kinds.push(CellKind::Symbol);
        let mut buffer = std::mem::take(&mut context.scratch.row);
        let mut joined = std::mem::take(&mut context.scratch.other);
        let outcome = (|| {
            let mut emit = Collect(out);
            for position in positions(count_of(rows)) {
                let Some(row) = read_row(rows, position, &mut buffer) else {
                    continue;
                };
                words_row(operation, row, context, &mut joined, &mut emit)?;
            }
            Ok(())
        })();
        context.scratch.row = buffer;
        context.scratch.other = joined;
        outcome
    }

    fn stream(
        &self,
        operation: &Operation,
        ins: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        let Room { memo, scratch, .. } = room;
        let mut joined = std::mem::take(&mut scratch.other);
        let outcome = ins.stream(
            InputPlace(0),
            context,
            &mut Pipe(|each: &mut OperatorContext<'_>, row: &[Value]| {
                let mut own = derive(each, memo, scratch);
                words_row(operation, row, &mut own, &mut joined, emit)
            }),
        );
        scratch.other = joined;
        outcome
    }
}
