//! A walked file for the tests of the concepts that read one: the source walked into a table by the language's own walk, and the
//! typed nodes of a second parse numbered in the same order, which stand for the nodes the walk lends only inside a visit.

use crate::domain::model::lexical::types::TypedTree;
use crate::domain::model::types::BindingAnnotationTiming;
use crate::domain::node::types::{SyntaxFile, Wholeness};
use ruff_python_ast::visitor::source_order::{SourceOrderVisitor, TraversalSignal};
use ruff_python_ast::{AnyNodeRef, HasNodeIndex as _, InterpolatedStringElement, NodeIndex};
use ryo::languages::{FileId, NameText, NodeTable, Position, SyntaxCompleteness, node_table};

/// The visitor that numbers the nodes as the walk does: in the order it opens them, a format spec entered as the node it is.
struct Numbering<'n, 't>(&'n mut Vec<AnyNodeRef<'t>>);

impl<'t> SourceOrderVisitor<'t> for Numbering<'_, 't> {
    fn enter_node(&mut self, node: AnyNodeRef<'t>) -> TraversalSignal {
        let id = u32::try_from(self.0.len()).unwrap_or(u32::MAX);
        node.node_index().set(NodeIndex::from(id));
        self.0.push(node);
        TraversalSignal::Traverse
    }

    fn visit_interpolated_string_element(&mut self, element: &'t InterpolatedStringElement) {
        let InterpolatedStringElement::Interpolation(interpolation) = element else {
            ruff_python_ast::visitor::source_order::walk_interpolated_string_element(self, element);
            return;
        };
        if self.enter_node(AnyNodeRef::from(element)).is_traverse() {
            self.visit_expr(&interpolation.expression);
            if let Some(spec) = &interpolation.format_spec
                && self
                    .enter_node(AnyNodeRef::from(spec.as_ref()))
                    .is_traverse()
            {
                for part in &spec.elements {
                    self.visit_interpolated_string_element(part);
                }
            }
        }
    }
}

/// The parent of each node of a finished table and where its subtree ends, by id: what the walk lends beside its typed nodes.
pub(crate) fn outline(table: &NodeTable) -> (Vec<Position>, Vec<Position>) {
    let ids =
        || (0..u32::try_from(node_table::node_count(table).0).unwrap_or(u32::MAX)).map(Position);
    (
        ids().map(|id| node_table::parent_of(table, id)).collect(),
        ids()
            .map(|id| node_table::subtree_end_of(table, id))
            .collect(),
    )
}

/// The names of `nodes` as the walk interns them as each opens (`lexical::named`); no name token codes.
pub(crate) fn names_of(nodes: &[ruff_python_ast::AnyNodeRef<'_>]) -> crate::domain::model::names::types::WalkNames {
    let mut names = crate::domain::model::names::types::WalkNames::default();
    for node in nodes {
        let code = crate::domain::model::lexical::service::named(&mut names.names, *node);
        names.of_node.push(code);
    }
    names
}

/// Walks `source` and hands `read` the walked file.
///
/// # Panics
///
/// When the source is not walked or the parser refuses it: a test's source is Python the parser reads.
pub(crate) fn with_file(source: &str, read: &mut dyn FnMut(&SyntaxFile<'_>)) {
    let mut table = NodeTable::default();
    let mut scratch = Default::default();
    let walk = crate::walk();
    let walked = super::walking::walked(
        &walk,
        "module.py",
        source.as_bytes(),
        &[],
        &mut table,
        &mut scratch,
        &mut |_file| {},
    );
    assert!(walked.is_ok(), "the source is walked");
    let parsed = ruff_python_parser::parse_unchecked(
        source,
        ruff_python_parser::ParseOptions::from(ruff_python_ast::PySourceType::Python),
    );
    let mut nodes = Vec::new();
    let mut numbering = Numbering(&mut nodes);
    numbering.visit_mod(parsed.syntax());
    let (parents, ends) = outline(&table);
    let named = names_of(&nodes);
    read(&SyntaxFile {
        table: &table,
        nodes: &nodes,
        tree: TypedTree {
            segments: &[],
            parents: &parents,
            ends: &ends,
            nodes: &nodes,
            annotations: BindingAnnotationTiming::Immediate,
            named: &named,
        },
        text: NameText(source),
        file: FileId(0),
        completeness: Wholeness {
            syntax: SyntaxCompleteness::Complete,
            lexical: SyntaxCompleteness::Complete,
        },
    });
}
