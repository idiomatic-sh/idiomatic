//! The `operands` service: what one side of a comparison is for one row, with no IO and no ports.

use ryo_relations::domain::relations::types::Value;

use crate::domain::functions::service::apply;
use crate::domain::functions::types::MadeText;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::service::cell;
use crate::domain::table_model::types::ParameterBinding;
use crate::domain::table_texts::service::datum;
use crate::domain::table_texts::types::{Datum, TextView, Texts};
use ryo_values::domain::plan_parts::service::parameter_in;
use ryo_values::domain::plan_parts::types::WholeNumber;
use ryo_values::domain::plan_parts::types::{Applied, Operand, QueryLiteral};

/// What one side of a comparison is for one row: borrowed from the row's texts, the bound
/// parameters or the plan, so that comparing allocates nothing; a function's text is written into `made`.
pub(crate) fn resolve<'x>(
    operand: &'x Operand,
    row: &[Value],
    bindings: &'x [ParameterBinding],
    texts: &Texts<'x>,
    made: &'x mut MadeText,
) -> Result<Datum<'x>, OperatingError> {
    match operand {
        Operand::Function(applied) => applied_to(applied, row, bindings, texts, made),
        Operand::Column(index) => Ok(datum(cell(row, *index)?, texts)),
        Operand::Parameter(id) => {
            let binding = parameter_in(bindings, *id).ok_or(OperatingError::Unbound)?;
            Ok(match binding {
                ParameterBinding::Number(number) => Datum::Number(*number),
                ParameterBinding::Flag(flag) => Datum::Flag(*flag),
                ParameterBinding::Text(text) => Datum::Text(TextView(&text.0)),
                ParameterBinding::Numbers(numbers) => Datum::Numbers(numbers),
                ParameterBinding::Texts(words) => Datum::Texts(words),
                ParameterBinding::TextSets(_) | ParameterBinding::Tables(_) => {
                    return Err(OperatingError::WrongBinding);
                }
            })
        }
        Operand::Literal(QueryLiteral::Number(number)) => Ok(Datum::Number(WholeNumber(number.0))),
        Operand::Literal(QueryLiteral::Text(text)) => Ok(Datum::Text(TextView(&text.0))),
    }
}

/// The text a function makes for one row, written into `made`. An argument that is not text, such as a name whose text is not
/// held, makes the function's answer not text either: it is unlocated, as a span with no place is, and matches no text.
fn applied_to<'x>(
    applied: &'x Applied,
    row: &[Value],
    bindings: &'x [ParameterBinding],
    texts: &Texts<'x>,
    made: &'x mut MadeText,
) -> Result<Datum<'x>, OperatingError> {
    let mut arguments = Vec::with_capacity(applied.arguments.len());
    for argument in &applied.arguments {
        let mut inner = MadeText::default();
        match resolve(argument, row, bindings, texts, &mut inner)? {
            Datum::Text(text) => arguments.push(MadeText(text.0.to_owned())),
            Datum::Unlocated
            | Datum::Number(_)
            | Datum::Flag(_)
            | Datum::Numbers(_)
            | Datum::Texts(_) => return Ok(Datum::Unlocated),
        }
    }
    apply(applied.function, &arguments, made);
    Ok(Datum::Text(TextView(&made.0)))
}
