//! The walker of the engine's node table: one source-order walk of the library's tree that writes every node as it opens, with the
//! field it fills under its parent, and the tokens between the nodes, and keeps the typed node of each id.
use super::constants::{ELSE_WITHOUT_EXCEPT, ELSE_WITHOUT_EXCEPT_CODE};
use super::future::{else_without_except, unknown_futures};
use super::lexical::opens;
use super::schemas::{concrete_node_kind, token_kind};
use super::types::{FieldIndex, OpenWalked, ParserRefusal, Said, Strings, Walker};
use super::writer::visit_element;
use crate::domain::extras::types::Extra;
use crate::domain::labels::types::Field;
use crate::domain::node::service::{field_code, token_field};
use crate::domain::node::types::{AnnotationSeed, Placed, StringSite};
use crate::domain::node_fields::service::child_fields;
use crate::domain::syntax::constants::{
    MODULE_FUTURE_ANNOTATIONS, MODULE_FUTURE_ANNOTATIONS_DISABLED,
    MODULE_FUTURE_ANNOTATIONS_ENABLED,
};
use crate::domain::syntax::service::kind_bits;
use crate::domain::model::lexical::service::opened;
use crate::domain::model::names::constants::FIXED_TEXT;
use crate::domain::model::names::service::{intern, text_of};
use crate::domain::model::names::types::NameCode;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ruff_python_ast::visitor::source_order::{
    SourceOrderVisitor, TraversalSignal, walk_annotation,
};
use ruff_python_ast::{AnyNodeRef, Expr, HasNodeIndex as _, InterpolatedStringElement, NodeIndex};
use ruff_text_size::{Ranged as _, TextSize};
use ryo::languages::{
    DiagnosticMetadata, DiagnosticPhase, FieldCode, NameText, Offset, Position, TreeImpact,
    node_table,
};

/// Where an offset the library gives is in the text being walked, which starts at `origin` among the library's offsets.
fn placed(origin: Offset, at: TextSize) -> Offset {
    Offset(at.to_u32().saturating_sub(origin.0))
}

impl Walker<'_, '_, '_> {
    /// Where the next token not written starts; the end of the text when every token is written.
    fn next_start(&self) -> Offset {
        self.tokens
            .get(self.next.0)
            .map_or(self.length, |token| placed(self.origin, token.start()))
    }

    /// Writes the white space between the last token written and `to`, as far as the next token, as one `whitespace` token: the
    /// tokens read back the whole file.
    fn fill(&mut self, to: Offset) {
        let to = to.min(self.next_start());
        if to > self.written {
            node_table::token(
                self.builder,
                kind_bits(Kind::Extra(Extra::Whitespace)),
                FieldCode(0),
                self.written,
                to,
            );
            self.written = to;
        }
    }

    /// Writes the next token, after the white space before it, with the field it fills under the open node. Its text stays in the
    /// file: the table holds its span, and what the library decoded of a string is read from the typed node.
    fn write_next(&mut self) {
        let Some(token) = self.tokens.get(self.next.0) else {
            return;
        };
        self.next.0 += 1;
        self.fill(placed(self.origin, token.start()));
        let start = placed(self.origin, token.start()).max(self.written);
        let end = placed(self.origin, token.end()).max(start);
        let kind = token_kind(token.kind());
        if matches!(kind, Kind::Token(Token::Comment))
            && let Some(last) = self.last_import_end.take()
        {
            // A comment belongs to an import's physical lines through the first newline after its end.
            self.import_presence.line_comment = start <= last
                || self
                    .text
                    .as_bytes()
                    .get(last.0 as usize..start.0 as usize)
                    .is_some_and(|gap| !gap.iter().any(|b| matches!(b, b'\n' | b'\r')));
        }
        let field = match (self.open.last(), kind) {
            (Some(open), Kind::Token(named)) => token_field(open.node, token.kind(), named),
            (Some(_) | None, Kind::Node(_) | Kind::Token(_) | Kind::Extra(_)) => Field::None,
        };
        if matches!(kind, Kind::Token(Token::Name)) && !self.text.is_empty() {
            if token.kind().is_non_soft_keyword() {
                // A keyword is a fixed text of the language: its code is its kind's, the same in every file.
                self.names.of_token.push(NameCode(FIXED_TEXT | token.kind() as u32));
            } else {
                self.name_token(start, end);
            }
        }
        if let Some(shapes) = &mut self.shapes
            && let Kind::Token(token) = kind
            && crate::domain::expression_shapes::service::significant(token).is_some()
        {
            shapes.token(kind_bits(kind), self.text.get(start.0 as usize..end.0 as usize).unwrap_or_default());
        }
        node_table::token(self.builder, kind_bits(kind), field_code(field), start, end);
        self.written = end;
    }

    /// Interns the text of a name token of the file's own tree, from `start` to `end`: the name of the innermost open node when
    /// it is that text (a name's token under its node), else the text itself.
    fn name_token(&mut self, start: Offset, end: Offset) {
        let text = self
            .text
            .get(usize::from(Position(start.0))..usize::from(Position(end.0)))
            .unwrap_or_default();
        let held = self
            .open
            .last()
            .and_then(|open| self.names.lexical.name.get(usize::from(open.at)).copied())
            .filter(|code| text_of(&self.names.names, *code) == Some(NameText(text)));
        let code = held.unwrap_or_else(|| intern(&mut self.names.names, NameText(text)));
        self.names.of_token.push(code);
    }

    /// Writes every token that ends at or before `at`: what comes before a node that starts there. A token of no length where the
    /// innermost open node ends is outside that node, so it waits until the node is closed.
    fn write_before(&mut self, at: Offset) {
        let edge = self.ends.last().copied();
        let origin = self.origin;
        while self.tokens.get(self.next.0).is_some_and(|token| {
            placed(origin, token.end()) <= at
                && !(token.range().is_empty() && Some(placed(origin, token.start())) == edge)
        }) {
            self.write_next();
        }
    }

    /// Writes every token that starts before `at` and ends at or before it: what is inside a node that ends there.
    fn write_within(&mut self, at: Offset) {
        let origin = self.origin;
        while self.tokens.get(self.next.0).is_some_and(|token| {
            placed(origin, token.start()) < at && placed(origin, token.end()) <= at
        }) {
            self.write_next();
        }
    }
}

impl<'t> Walker<'_, '_, 't> {
    /// The field `node` fills under the innermost open node, as that node's typed fields say it; none under no node. Children open
    /// in the order their fields were read, so the next field is tried first.
    fn field_of(&mut self, node: AnyNodeRef<'t>) -> Field {
        let Some(open) = self.open.last_mut() else {
            return Field::None;
        };
        let own = self.fields.get(open.from.0..).unwrap_or_default();
        let next = open.next.0.saturating_sub(open.from.0);
        let found = match own.get(next) {
            Some(candidate) if candidate.child.ptr_eq(node) => Some(next),
            Some(_) | None => own.iter().position(|each| each.child.ptr_eq(node)),
        };
        let Some(place) = found else {
            return Field::None;
        };
        open.next = FieldIndex(open.from.0 + place + 1);
        own.get(place).map_or(Field::None, |each| each.field)
    }
}

impl<'t> SourceOrderVisitor<'t> for Walker<'_, '_, 't> {
    fn visit_interpolated_string_element(&mut self, element: &'t InterpolatedStringElement) {
        visit_element(self, element);
    }

    fn visit_annotation(&mut self, annotation: &'t Expr) {
        self.annotating.0 += 1;
        walk_annotation(self, annotation);
        self.annotating.0 = self.annotating.0.saturating_sub(1);
    }

    fn enter_node(&mut self, node: AnyNodeRef<'t>) -> TraversalSignal {
        let root = self.open.is_empty();
        let start = if root {
            Offset(0)
        } else {
            placed(self.origin, node.start())
        };
        self.write_before(start);
        self.fill(start);
        let field = self.field_of(node);
        let parent = self.open.last().map(|open| open.node);
        let strings = self.strings.last().copied().unwrap_or(Strings::Formatted);
        // The id the table gives the node as it opens: the walk keeps it in the library's own node index.
        let id = u32::try_from(self.nodes.len()).unwrap_or(u32::MAX);
        node.node_index().set(NodeIndex::from(id));
        let kind = concrete_node_kind(node, strings);
        if let Some(shapes) = &mut self.shapes {
            shapes.enter(Position(id), kind, field);
        }
        node_table::enter_node(
            self.builder,
            kind_bits(kind),
            field_code(field),
            start,
        );
        if matches!(node, AnyNodeRef::ModModule(_)) {
            node_table::node_property(
                self.builder,
                MODULE_FUTURE_ANNOTATIONS,
                NameText(if self.future_annotations.0 {
                    MODULE_FUTURE_ANNOTATIONS_ENABLED
                } else {
                    MODULE_FUTURE_ANNOTATIONS_DISABLED
                }),
            );
            node_table::node_property(
                self.builder,
                crate::domain::syntax::constants::MODULE_ANNOTATION_TIMING,
                super::future::annotation_timing(self.annotations_deferred),
            );
            node_table::node_property(
                self.builder,
                crate::domain::syntax::constants::MODULE_ANNOTATED_DECLARATIONS,
                super::future::declarations_text(self.declarations),
            );
        }
        self.nodes.push(node);
        let at = Position(id);
        // The root of a file is its own parent; the root of a string's expression has the string for its parent.
        self.parents.push(
            self.open
                .last()
                .map_or_else(|| self.host.unwrap_or(at), |open| open.at),
        );
        if matches!(node, AnyNodeRef::ExprStringLiteral(_)) {
            self.literals.push(StringSite {
                at,
                seed: if self.annotating.0 > 0 {
                    AnnotationSeed::Annotation
                } else {
                    AnnotationSeed::Candidate
                },
            });
        } else if matches!(
            node,
            AnyNodeRef::StmtImport(_) | AnyNodeRef::StmtImportFrom(_)
        ) {
            self.import_presence.statements += 1;
            if !self.import_presence.line_comment {
                self.last_import_end = Some(placed(self.origin, node.end()));
            }
            if matches!(node, AnyNodeRef::StmtImportFrom(stmt) if stmt.level != 0) {
                self.relative_froms.push(at);
            }
        } else if matches!(node, AnyNodeRef::Alias(_)) {
            self.imports.push(at);
        } else {
            // No other node is kept apart for the reader of the tree.
        }
        self.subtree_ends.push(Position(id.saturating_add(1)));
        opened(&mut self.names.lexical, &mut self.names.names, &self.nodes, &self.parents, &self.subtree_ends, at);
        let from = FieldIndex(self.fields.len());
        child_fields(
            Placed {
                node,
                field,
                parent,
            },
            &mut self.fields,
        );
        self.open.push(OpenWalked {
            node,
            at,
            from,
            next: from,
        });
        if let Some(opened) = opens(node.kind()) {
            self.strings.push(opened);
        }
        if let AnyNodeRef::StmtTry(statement) = node
            && let Some(at) = else_without_except(statement, self.parsed_tokens)
        {
            self.refused.push(ParserRefusal {
                at,
                said: Said(ELSE_WITHOUT_EXCEPT.to_owned()),
                metadata: DiagnosticMetadata {
                    phase: DiagnosticPhase::Parse,
                    impact: TreeImpact::Incomplete,
                    code: ELSE_WITHOUT_EXCEPT_CODE,
                },
            });
        }
        if let AnyNodeRef::StmtImportFrom(statement) = node {
            self.refused.extend(unknown_futures(statement));
        }
        self.ends.push(if root {
            self.length
        } else {
            placed(self.origin, node.end())
        });
        TraversalSignal::Traverse
    }

    fn leave_node(&mut self, node: AnyNodeRef<'t>) {
        // Where the node ends, as it was placed when it opened.
        let placed_end = self.ends.pop();
        if opens(node.kind()).is_some() {
            let _: Option<Strings> = self.strings.pop();
        }
        // The node stays open while its last tokens are written: they are its own.
        let end = if self.open.len() <= 1 {
            while self.next.0 < self.tokens.len() {
                self.write_next();
            }
            self.length
        } else {
            let end = placed_end.unwrap_or_else(|| placed(self.origin, node.end()));
            self.write_within(end);
            end
        };
        self.fill(end);
        if let Some(open) = self.open.pop() {
            if let Some(shapes) = &mut self.shapes {
                shapes.leave(open.at, node, self.names.lexical.name[usize::from(open.at)]);
            }
            self.fields.truncate(open.from.0);
            let after = Position(u32::try_from(self.nodes.len()).unwrap_or(u32::MAX));
            if let Some(subtree_end) = self.subtree_ends.get_mut(usize::from(open.at)) {
                *subtree_end = after;
            }
        }
        node_table::leave_node(self.builder, end);
    }
}
