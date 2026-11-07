//! The `capture` operator: match paths against a pattern with named slots and append what the slots matched.
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
pub(crate) struct CaptureItem;
impl OperatorsItem for CaptureItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputShape<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::Capture,
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
        let Operation::Capture {
            path: column,
            pattern,
            ..
        } = operation
        else {
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
        let width = memchr::memchr_iter(b'{', pattern.0.as_bytes()).count();
        let mut ways = 0;
        let mut slots = std::mem::take(&mut lease.workspace.stretches);
        slots.0.clear();
        let answer = lease.semantics().operators(
            ryo_operators::inbound::use_cases::operators::types::OperatorCall::Capture {
                pattern: ryo_operators::domain::capture::types::SlotPattern(&pattern.0),
                text: ryo_operators::domain::capture::types::SlotSubject(&text),
                slots: ryo_operators::domain::capture::types::CaptureSlots(&mut slots.0),
                each: &mut |places: &[std::ops::Range<usize>]| {
                    for place in places {
                        let made = query_intern_derived(
                            &mut lease.derived,
                            ByteSlice(lease.views.source.0),
                            NameText(text.get(place.clone()).unwrap_or_default()),
                        );
                        captured.push(minted(lease.derived.as_ref(), made, after));
                    }
                    ways += 1;
                },
            },
        );
        if let Err(error) = crate::domain::operator::contract::types::written_result(answer, None) {
            lease.workspace.captured = captured;
            lease.workspace.text = text;
            lease.workspace.stretches = slots;
            return Err(error);
        }
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
        lease.workspace.stretches = slots;
        result
    }
}
