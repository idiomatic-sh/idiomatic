//! Engine-owned acquisition at declared invocation boundaries.
use crate::domain::operating::{
    errors::OperatingError,
    service::minted,
    types::{AfterTheFiles, OperatorContext},
};
use crate::domain::table_model::types::ParameterBinding;
use ryo_language::domain::relations::types::{Flag, Integer, Value};
use ryo_language::domain::symbols::{service::query_intern_derived, types::ByteSlice};
use ryo_language::domain::terms::types::NameText;
use ryo_values::domain::plan_parts::{
    service::parameter_in,
    types::{FlagState, Operand, QueryLiteral},
};
pub(crate) fn appended_values(
    values: &[Operand],
    context: &mut OperatorContext<'_>,
    appended: &mut Vec<Value>,
) -> Result<(), OperatingError> {
    let OperatorContext {
        bindings,
        symbols,
        source,
        arenas,
        ..
    } = context;
    let after = if arenas.is_empty() {
        AfterTheFiles::Before
    } else {
        AfterTheFiles::After
    };
    appended.clear();
    for operand in values {
        appended.push(match operand {
            Operand::Literal(QueryLiteral::Number(number)) => Value::Number(Integer(number.0)),
            // A text becomes a name of the file's names, the same name for the same text.
            Operand::Literal(QueryLiteral::Text(text)) => {
                let made = query_intern_derived(symbols, ByteSlice(source.0), NameText(&text.0));
                minted(symbols.as_ref(), made, after)
            }
            Operand::Column(_) | Operand::Function(_) => return Err(OperatingError::WrongBinding),
            Operand::Parameter(id) => {
                match parameter_in(bindings, *id).ok_or(OperatingError::Unbound)? {
                    ParameterBinding::Number(number) => Value::Number(Integer(number.0)),
                    ParameterBinding::Flag(flag) => Value::Flag(if *flag == FlagState::Raised {
                        Flag::Raised
                    } else {
                        Flag::Lowered
                    }),
                    ParameterBinding::Text(text) => {
                        let made =
                            query_intern_derived(symbols, ByteSlice(source.0), NameText(&text.0));
                        minted(symbols.as_ref(), made, after)
                    }
                    ParameterBinding::Numbers(_)
                    | ParameterBinding::Texts(_)
                    | ParameterBinding::TextSets(_)
                    | ParameterBinding::Tables(_) => {
                        return Err(OperatingError::WrongBinding);
                    }
                }
            }
        });
    }
    Ok(())
}
