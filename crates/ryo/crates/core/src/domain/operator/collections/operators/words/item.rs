//! The `words` operator: split a text column into words and append each word, one row for each.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::minted;
use crate::domain::operating::types::AfterTheFiles;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_model::service::cell;
use crate::domain::table_model::types::ParameterBinding;
use crate::domain::table_scratch::types::ScratchWord;
use crate::domain::table_texts::service::datum;
use crate::domain::table_texts::types::TextView;
use crate::domain::table_texts::types::{Datum, Texts};
use ryo_language::domain::symbols::service::query_intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plans::types::{Operation, OperationKind};
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

pub(crate) struct WordsItem;
impl OperatorsItem for WordsItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputSchema<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::Words,
            operation,
            inputs,
            bindings,
        )
    }
    fn evaluate(
        &self,
        operation: &Operation,
        inputs: PreparedInputs<'_>,
        lease: &mut PrimitiveLease<'_>,
        emit: &mut dyn RowSink,
    ) -> Result<(), OperatingError> {
        let PreparedInputs::Row { row, .. } = inputs else {
            return Err(OperatingError::WrongOperation);
        };
        let Operation::Words { text: column, .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let texts = Texts {
            symbols: lease.derived.as_ref(),
            source: lease.views.source,
            paths: lease.views.paths,
            arenas: lease.views.arenas,
        };
        let Datum::Text(value) = datum(cell(row, *column)?, &texts) else {
            return Ok(());
        };
        // Own the input while derived interning may grow the symbol table.
        let mut text = std::mem::take(lease.workspace.text);
        text.clear();
        text.push_str(value.0);
        let after = if lease.views.arenas.is_empty() {
            AfterTheFiles::Before
        } else {
            AfterTheFiles::After
        };
        let mut captured = std::mem::take(lease.workspace.captured);
        captured.clear();
        let width = 1;
        let mut word = std::mem::take(lease.workspace.word);
        split(TextView(&text), &mut word, |each| {
            let made = query_intern_derived(
                &mut lease.derived,
                ByteSlice(lease.views.source.0),
                NameText(each.0),
            );
            captured.push(minted(lease.derived.as_ref(), made, after));
        });
        let ways = captured.len();
        let mut joined = std::mem::take(lease.workspace.row);
        let result = (|| {
            for way in 0..ways {
                joined.clear();
                joined.extend_from_slice(row);
                joined.extend_from_slice(
                    captured
                        .get(way * width..(way + 1) * width)
                        .unwrap_or_default(),
                );
                emit.row(lease, &joined)?;
            }
            Ok(())
        })();
        *lease.workspace.row = joined;
        *lease.workspace.captured = captured;
        *lease.workspace.text = text;
        *lease.workspace.word = word;
        result
    }
}
