//! The services of the `rendering` concept: the text of an expression in a syntax role. Only the forms whose embedding the
//! syntax tree proves are rendered; every other valid form is no text, which the caller answers as unsupported rather than
//! silently losing a row.

use super::constants::LIST_SEPARATOR;
use ruff_python_ast::{ExprContext, ExprRef};
use ruff_text_size::Ranged as _;
use ryo::languages::{self, Flag, RenderContext};

/// The source text an expression spans.
fn source_slice<'a>(
    expr: ExprRef<'_>,
    source: languages::NameText<'a>,
) -> Option<languages::NameText<'a>> {
    let range = expr.range();
    let (Ok(start), Ok(end)) = (
        usize::try_from(range.start().to_u32()),
        usize::try_from(range.end().to_u32()),
    ) else {
        return None;
    };
    source.0.get(start..end).map(languages::NameText)
}

/// Whether an expression is stored to: of the six kinds with a load or store context, the stored one.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only the six expression kinds with a load or store context can be assigned to; every other expression is not a target"
)]
pub(crate) fn target(expr: ExprRef<'_>) -> Flag {
    let stored = match expr {
        ExprRef::Name(name) => name.ctx == ExprContext::Store,
        ExprRef::Attribute(attribute) => attribute.ctx == ExprContext::Store,
        ExprRef::Subscript(subscript) => subscript.ctx == ExprContext::Store,
        ExprRef::Tuple(tuple) => tuple.ctx == ExprContext::Store,
        ExprRef::List(list) => list.ctx == ExprContext::Store,
        ExprRef::Starred(starred) => starred.ctx == ExprContext::Store,
        _ => false,
    };
    if stored { Flag::Raised } else { Flag::Lowered }
}

/// The text of `expr` embedded in `context`; none for a form whose embedding the tree does not prove.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only the forms whose embedding the AST proves are rendered; every other expression renders as nothing"
)]
pub(crate) fn render(
    expr: ExprRef<'_>,
    context: RenderContext,
    source: languages::NameText<'_>,
) -> Option<String> {
    match expr {
        ExprRef::Name(name) => Some(name.id.to_string()),
        ExprRef::Attribute(attribute) => Some(format!(
            "{}.{}",
            render(
                ExprRef::from(&*attribute.value),
                RenderContext::AttributeReceiver,
                source
            )?,
            attribute.attr
        )),
        ExprRef::Subscript(subscript) => Some(format!(
            "{}[{}]",
            render(
                ExprRef::from(&*subscript.value),
                RenderContext::AttributeReceiver,
                source
            )?,
            render(
                ExprRef::from(&*subscript.slice),
                RenderContext::PositionalArgument,
                source
            )?
        )),
        ExprRef::Tuple(tuple) => {
            let mut items = Vec::with_capacity(tuple.elts.len());
            for item in &tuple.elts {
                items.push(render(
                    ExprRef::from(item),
                    RenderContext::PositionalArgument,
                    source,
                )?);
            }
            let mut text = items.join(LIST_SEPARATOR);
            if items.len() == 1 {
                text.push(',');
            }
            if context == RenderContext::AttributeReceiver
                || context == RenderContext::PositionalArgument
            {
                Some(format!("({text})"))
            } else {
                Some(text)
            }
        }
        ExprRef::List(list) => {
            let mut items = Vec::with_capacity(list.elts.len());
            for item in &list.elts {
                items.push(render(
                    ExprRef::from(item),
                    RenderContext::PositionalArgument,
                    source,
                )?);
            }
            Some(format!("[{}]", items.join(LIST_SEPARATOR)))
        }
        ExprRef::Starred(starred) if context == RenderContext::AssignmentTarget => Some(format!(
            "*{}",
            render(ExprRef::from(&*starred.value), context, source)?
        )),
        ExprRef::StringLiteral(_)
        | ExprRef::BytesLiteral(_)
        | ExprRef::NumberLiteral(_)
        | ExprRef::BooleanLiteral(_)
        | ExprRef::NoneLiteral(_)
        | ExprRef::EllipsisLiteral(_)
        | ExprRef::Lambda(_)
        | ExprRef::Dict(_)
        | ExprRef::Set(_) => {
            if context == RenderContext::AttributeReceiver {
                return None;
            }
            source_slice(expr, source).map(|slice| slice.0.to_owned())
        }
        _ => None,
    }
}
