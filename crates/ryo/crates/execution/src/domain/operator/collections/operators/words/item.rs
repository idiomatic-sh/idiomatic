//! The `words` operator: split a text column into words and append each word, one row for each.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::minted;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_texts::service::datum;
use crate::domain::table_texts::types::{Datum, Texts};
use ryo_language::domain::symbols::service::query_intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::rows::service::cell;
pub(crate) struct WordsItem;
impl OperatorsItem for WordsItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputShape<'_>],
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
        let mut text = std::mem::take(&mut lease.workspace.text);
        text.clear();
        text.push_str(value.0);
        let after = lease.views.phase;
        let mut captured = std::mem::take(&mut lease.workspace.captured);
        captured.clear();
        let width = 1;
        let mut word = std::mem::take(&mut lease.workspace.word);
        let answer = lease.semantics().operators(
            ryo_operators::inbound::use_cases::operators::types::OperatorCall::Words {
                text: ryo_values::domain::row_texts::types::CellText(&text),
                word: ryo_operators::domain::words::types::WordScratch(&mut word.0),
                each: &mut |each: &str| {
                    let made = query_intern_derived(
                        &mut lease.derived,
                        ByteSlice(lease.views.source.0),
                        NameText(each),
                    );
                    captured.push(minted(lease.derived.as_ref(), made, after));
                },
            },
        );
        let result = crate::domain::operator::contract::types::written_result(answer, None);
        if let Err(error) = result {
            lease.workspace.captured = captured;
            lease.workspace.text = text;
            lease.workspace.word = word;
            return Err(error);
        }
        let ways = captured.len();
        let mut joined = std::mem::take(&mut lease.workspace.row);
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
        lease.workspace.row = joined;
        lease.workspace.captured = captured;
        lease.workspace.text = text;
        lease.workspace.word = word;
        result
    }
}
