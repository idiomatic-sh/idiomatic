//! Type and pattern validation for the replacement projection.

use ryo_language::domain::relations::types::CellKind;
use ryo_language::domain::terms::types::Vocabulary;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::program::types::{Call, QuerySpelling, StepName};

use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{Builder, PlannedColumn, Sourced, Uniqueness};
use crate::domain::planning::service::push;
use crate::domain::resolving::service::{column_of, input_argument, position, word_of};
use crate::domain::syntax::types::ArgumentSlot;

/// Validate constant regex and append a new text column without changing input identity.
pub(crate) fn call_replace(
    builder: &mut Builder<'_>,
    name: &StepName,
    call: &Call,
) -> Result<(), LoweringError> {
    let Sourced {
        input: from,
        mut fields,
    } = input_argument(builder, call, ArgumentSlot::From)?;
    let text = position(
        &fields,
        QuerySpelling(&word_of(call, ArgumentSlot::Text)?.0),
        call.span,
    )?;
    if column_of(&fields, text).kind != CellKind::Symbol
        && column_of(&fields, text).kind != CellKind::File
    {
        return Err(LoweringError::KindMismatch { span: call.span });
    }
    let pattern = LiteralText(word_of(call, ArgumentSlot::Pattern)?.0);
    let replacement = LiteralText(word_of(call, ArgumentSlot::Replacement)?.0);
    let output = word_of(call, ArgumentSlot::Output)?;
    if fields.iter().any(|field| field.name == output) {
        return Err(LoweringError::DuplicateOutput {
            name: output.0.to_string(),
            span: call.span,
        });
    }
    let _compiled = regex_automata::meta::Regex::new(&pattern.0).map_err(|error| {
        LoweringError::InvalidRegex {
            reason: error.to_string(),
            span: call.span,
        }
    })?;
    fields.push(PlannedColumn {
        name: output,
        kind: CellKind::Symbol,
        unique: Uniqueness::Shared,
        words: Vocabulary::Open,
    });
    push(
        builder,
        Some(name),
        Operation::Replace {
            from,
            text,
            pattern,
            replacement,
        },
        fields,
    );
    Ok(())
}
