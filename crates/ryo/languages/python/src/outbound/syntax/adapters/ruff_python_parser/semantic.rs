//! Lazy, file-scoped Python syntax rendering for demanded AST nodes.

use super::constants::LIST_SEPARATOR;
use std::collections::HashMap;
use std::sync::Arc;

use ruff_python_ast::visitor::{Visitor, walk_expr};
use ruff_python_ast::{AnyNodeRef, Expr, ExprContext};
use ruff_text_size::Ranged as _;
use ryo::languages::{
    self, RenderContext, RenderError, RenderOutcome, RenderRequest, SemanticSession,
};

use super::schemas::concrete_node_kind;
use super::types::Strings;
use crate::domain::syntax::service::kind_bits;
use crate::outbound::syntax::types::PythonSourceKind;
use crate::settings::PythonParserSettings;

const UNSUPPORTED: languages::UnknownReason = languages::UnknownReason(24);

/// Native element identity retained only for a file's demanded semantic session.
pub(super) struct PythonSemanticSession {
    elements: Vec<(u32, u32, languages::KindBits)>,
    source_kind: PythonSourceKind,
    settings: PythonParserSettings,
    parsed: Option<ruff_python_parser::Parsed<ruff_python_ast::Mod>>,
}

impl PythonSemanticSession {
    pub(super) fn new(
        tree: &languages::Elements,
        source_kind: PythonSourceKind,
        settings: PythonParserSettings,
    ) -> Self {
        let elements = tree
            .start
            .iter()
            .zip(tree.end.iter())
            .zip(tree.kind.iter())
            .map(|((start, end), kind)| (start.0, end.0, *kind))
            .collect();
        Self {
            elements,
            source_kind,
            settings,
            parsed: None,
        }
    }
}

fn source_slice<'a>(expr: &Expr, source: &'a str) -> Option<&'a str> {
    let range = expr.range();
    let (Ok(start), Ok(end)) = (
        usize::try_from(range.start().to_u32()),
        usize::try_from(range.end().to_u32()),
    ) else {
        return None;
    };
    source.get(start..end)
}

fn target(expr: &Expr) -> bool {
    match expr {
        Expr::Name(name) => name.ctx == ExprContext::Store,
        Expr::Attribute(attribute) => attribute.ctx == ExprContext::Store,
        Expr::Subscript(subscript) => subscript.ctx == ExprContext::Store,
        Expr::Tuple(tuple) => tuple.ctx == ExprContext::Store,
        Expr::List(list) => list.ctx == ExprContext::Store,
        Expr::Starred(starred) => starred.ctx == ExprContext::Store,
        _ => false,
    }
}

/// Render only the syntax forms whose embedding can be proven from the AST.
/// Other valid forms remain explicit Unknown instead of silently losing a row.
fn render(expr: &Expr, context: RenderContext, source: &str) -> Option<String> {
    match expr {
        Expr::Name(name) => Some(name.id.to_string()),
        Expr::Attribute(attribute) => Some(format!(
            "{}.{}",
            render(&attribute.value, RenderContext::AttributeReceiver, source)?,
            attribute.attr
        )),
        Expr::Subscript(subscript) => Some(format!(
            "{}[{}]",
            render(&subscript.value, RenderContext::AttributeReceiver, source)?,
            render(&subscript.slice, RenderContext::PositionalArgument, source)?
        )),
        Expr::Tuple(tuple) => {
            let mut items = Vec::with_capacity(tuple.elts.len());
            for item in &tuple.elts {
                items.push(render(item, RenderContext::PositionalArgument, source)?);
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
        Expr::List(list) => {
            let mut items = Vec::with_capacity(list.elts.len());
            for item in &list.elts {
                items.push(render(item, RenderContext::PositionalArgument, source)?);
            }
            Some(format!("[{}]", items.join(LIST_SEPARATOR)))
        }
        Expr::Starred(starred) if context == RenderContext::AssignmentTarget => {
            Some(format!("*{}", render(&starred.value, context, source)?))
        }
        Expr::StringLiteral(_)
        | Expr::BytesLiteral(_)
        | Expr::NumberLiteral(_)
        | Expr::BooleanLiteral(_)
        | Expr::NoneLiteral(_)
        | Expr::EllipsisLiteral(_)
        | Expr::Lambda(_)
        | Expr::Dict(_)
        | Expr::Set(_) => {
            if context == RenderContext::AttributeReceiver {
                return None;
            }
            source_slice(expr, source).map(str::to_owned)
        }
        _ => None,
    }
}

type Key = (u32, u32, languages::KindBits);

struct Requested<'a> {
    wanted: HashMap<Key, Vec<(usize, RenderContext)>>,
    answers: Vec<Option<RenderOutcome>>,
    source: &'a str,
}

impl<'a> Visitor<'a> for Requested<'_> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        let range = expr.range();
        let key = (
            range.start().to_u32(),
            range.end().to_u32(),
            kind_bits(concrete_node_kind(
                AnyNodeRef::from(expr),
                Strings::Formatted,
            )),
        );
        if let Some(indices) = self.wanted.get(&key) {
            for &(index, context) in indices {
                let answer = if context == RenderContext::AssignmentTarget && !target(expr) {
                    RenderOutcome::NotApplicable
                } else {
                    render(expr, context, self.source)
                        .map(|text| RenderOutcome::Known(Arc::from(text)))
                        .unwrap_or(RenderOutcome::Unsupported(UNSUPPORTED))
                };
                self.answers[index] = Some(match self.answers[index].take() {
                    Some(_) => RenderOutcome::Unsupported(UNSUPPORTED),
                    None => answer,
                });
            }
        }
        walk_expr(self, expr);
    }
}

impl SemanticSession for PythonSemanticSession {
    fn render_batch(
        &mut self,
        source: languages::ByteSlice<'_>,
        requests: &[RenderRequest],
    ) -> Result<Vec<RenderOutcome>, RenderError> {
        let Ok(source) = std::str::from_utf8(source.0) else {
            return Ok(vec![
                RenderOutcome::Unsupported(UNSUPPORTED);
                requests.len()
            ]);
        };
        let parsed = self.parsed.get_or_insert_with(|| {
            ruff_python_parser::parse_unchecked(
                source,
                super::options::options(self.source_kind, &self.settings),
            )
        });
        let mut asked = Requested {
            wanted: HashMap::new(),
            answers: vec![None; requests.len()],
            source,
        };
        for (index, request) in requests.iter().enumerate() {
            let Some(&(start, end, kind)) = self.elements.get(
                usize::try_from(languages::node_position(request.node).0).unwrap_or(usize::MAX),
            ) else {
                asked.answers[index] = Some(RenderOutcome::Unsupported(UNSUPPORTED));
                continue;
            };
            asked
                .wanted
                .entry((start, end, kind))
                .or_default()
                .push((index, request.context));
        }
        if let Some(module) = parsed.syntax().as_module() {
            asked.visit_body(&module.body);
        }
        Ok(asked
            .answers
            .into_iter()
            .map(|answer| answer.unwrap_or(RenderOutcome::Unsupported(UNSUPPORTED)))
            .collect())
    }
}
