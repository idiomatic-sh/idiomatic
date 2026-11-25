//! Lazy, file-scoped Python syntax rendering for demanded AST nodes.

use std::collections::HashMap;
use std::sync::Arc;

use ruff_python_ast::visitor::{Visitor, walk_expr};
use ruff_python_ast::{AnyNodeRef, Expr, ExprRef};
use ruff_text_size::Ranged as _;
use ryo::languages::{
    self, Flag, RenderContext, RenderOutcome, RenderRequest, SemanticSession, SemanticsError,
};

use super::schemas::concrete_node_kind;
use super::types::Strings;
use crate::domain::rendering::constants::UNRENDERED;
use crate::domain::rendering::service::{render, target};
use crate::domain::syntax::service::kind_bits;
use crate::outbound::syntax::types::PythonSourceKind;
use crate::settings::PythonParserSettings;

/// Native element identity retained only for a file's demanded semantic session.
pub(super) struct PythonSemanticSession {
    elements: Vec<ElementKey>,
    source_kind: PythonSourceKind,
    settings: PythonParserSettings,
    parsed: Option<ruff_python_parser::Parsed<ruff_python_ast::Mod>>,
}

impl PythonSemanticSession {
    pub(super) fn new(
        tree: &languages::Elements,
        source_kind: PythonSourceKind,
        settings: PythonParserSettings,
        parsed: Option<ruff_python_parser::Parsed<ruff_python_ast::Mod>>,
    ) -> Self {
        let elements = tree
            .start
            .iter()
            .zip(tree.end.iter())
            .zip(tree.kind.iter())
            .map(|((start, end), kind)| ElementKey {
                start: *start,
                end: *end,
                kind: *kind,
            })
            .collect();
        Self {
            elements,
            source_kind,
            settings,
            parsed,
        }
    }
}

/// The extent and kind that identify one element of the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ElementKey {
    start: languages::Offset,
    end: languages::Offset,
    kind: languages::KindBits,
}

/// One requested rendering: which answer slot it fills and in what context.
#[derive(Clone, Copy)]
struct Wanted {
    slot: Slot,
    context: RenderContext,
}

/// The index of an answer in the batch.
#[derive(Clone, Copy)]
struct Slot(usize);

struct Requested<'a> {
    wanted: HashMap<ElementKey, Vec<Wanted>>,
    answers: Vec<Option<RenderOutcome>>,
    source: languages::NameText<'a>,
}

impl<'a> Visitor<'a> for Requested<'_> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        let range = expr.range();
        let key = ElementKey {
            start: languages::Offset(range.start().to_u32()),
            end: languages::Offset(range.end().to_u32()),
            kind: kind_bits(concrete_node_kind(
                AnyNodeRef::from(expr),
                Strings::Formatted,
            )),
        };
        if let Some(indices) = self.wanted.get(&key) {
            for &Wanted {
                slot: Slot(index),
                context,
            } in indices
            {
                let answer = if context == RenderContext::AssignmentTarget
                    && target(ExprRef::from(expr)) == Flag::Lowered
                {
                    RenderOutcome::NotApplicable
                } else {
                    render(ExprRef::from(expr), context, self.source)
                        .map(|text| RenderOutcome::Known(languages::RenderedNode(Arc::from(text))))
                        .unwrap_or(RenderOutcome::Unsupported(UNRENDERED))
                };
                self.answers[index] = Some(match self.answers[index].take() {
                    Some(_) => RenderOutcome::Unsupported(UNRENDERED),
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
    ) -> Result<Vec<RenderOutcome>, SemanticsError> {
        let Ok(source) = std::str::from_utf8(source.0) else {
            return Ok(vec![RenderOutcome::Unsupported(UNRENDERED); requests.len()]);
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
            source: languages::NameText(source),
        };
        for (index, request) in requests.iter().enumerate() {
            let Some(&key) = self.elements.get(
                usize::try_from(languages::node_position(request.node).0).unwrap_or(usize::MAX),
            ) else {
                asked.answers[index] = Some(RenderOutcome::Unsupported(UNRENDERED));
                continue;
            };
            asked.wanted.entry(key).or_default().push(Wanted {
                slot: Slot(index),
                context: request.context,
            });
        }
        if let Some(module) = parsed.syntax().as_module() {
            asked.visit_body(&module.body);
        }
        Ok(asked
            .answers
            .into_iter()
            .map(|answer| answer.unwrap_or(RenderOutcome::Unsupported(UNRENDERED)))
            .collect())
    }
}
