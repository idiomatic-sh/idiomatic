//! A walk for the tests of the semantic model and its fillers: the file parsed with the library and its nodes walked in source
//! order into a node table (nodes only: the model reads no token), with the library's node of each id. The production walk is
//! the provider's `node` concept; this one exists so the model is tested without it.

use crate::domain::standard_library::types::TargetVersion;
use crate::domain::syntax::constants::{
    MODULE_FUTURE_ANNOTATIONS, MODULE_FUTURE_ANNOTATIONS_DISABLED, MODULE_FUTURE_ANNOTATIONS_ENABLED,
};
use crate::domain::syntax::service::{kind_bits, kinds};
use crate::outbound::syntax::adapters::ruff_python_parser::future::module_future_annotations;
use crate::outbound::syntax::adapters::ruff_python_parser::options::options;
use crate::outbound::syntax::types::PythonSourceKind;
use crate::settings::PythonParserSettings;
use crate::outbound::syntax::adapters::ruff_python_parser::schemas::concrete_node_kind;
use crate::outbound::syntax::adapters::ruff_python_parser::types::Strings;
use ruff_python_ast::visitor::source_order::{SourceOrderVisitor, TraversalSignal};
use ruff_python_ast::{AnyNodeRef, HasNodeIndex as _, Mod, NodeIndex};
use ruff_python_parser::parse_unchecked;
use ruff_text_size::Ranged as _;
use ryo::languages::node_table::{builder, enter_node, finished, leave_node, node_property, set_syntax_acceptance};
use ryo::languages::{Count, FieldCode, NameText, NodeTable, NodeTableBuilder, Offset, SyntaxAcceptance};

/// The walk: every node the library's source-order visit enters, numbered as it opens.
struct Walker<'b, 't, 'a> {
    /// The table's builder.
    builder: &'b mut NodeTableBuilder<'t>,
    /// The library's node of each id so far.
    nodes: Vec<AnyNodeRef<'a>>,
    /// What the module's future-annotations property says.
    future: NameText<'static>,
}

impl<'a> SourceOrderVisitor<'a> for Walker<'_, '_, 'a> {
    fn enter_node(&mut self, node: AnyNodeRef<'a>) -> TraversalSignal {
        let id = u32::try_from(self.nodes.len()).unwrap_or(u32::MAX);
        node.node_index().set(NodeIndex::from(id));
        enter_node(
            self.builder,
            kind_bits(concrete_node_kind(node, Strings::Formatted)),
            FieldCode(0),
            Offset(node.start().to_u32()),
        );
        if matches!(node, AnyNodeRef::ModModule(_)) {
            node_property(self.builder, MODULE_FUTURE_ANNOTATIONS, self.future);
        }
        self.nodes.push(node);
        TraversalSignal::Traverse
    }

    fn leave_node(&mut self, node: AnyNodeRef<'a>) {
        leave_node(self.builder, Offset(node.end().to_u32()));
    }
}

/// Parses `source` under `settings`, walks it into `table`, and lends the finished table, the library's node of each node id
/// and the settings' language target to `visit`.
pub(crate) fn walked(
    table: &mut NodeTable,
    source: &str,
    settings: PythonParserSettings,
    visit: &mut dyn FnMut(&NodeTable, &[AnyNodeRef<'_>], TargetVersion),
) {
    let parsed = parse_unchecked(source, options(PythonSourceKind::Python, &settings));
    let module = parsed.syntax();
    let enabled = match module {
        Mod::Module(body) => module_future_annotations(body, &mut Vec::new()).0,
        Mod::Expression(_) => false,
    };
    let future = NameText(if enabled { MODULE_FUTURE_ANNOTATIONS_ENABLED } else { MODULE_FUTURE_ANNOTATIONS_DISABLED });
    let mut table_builder = builder(table, Count(kinds().count()));
    set_syntax_acceptance(
        &mut table_builder,
        if parsed.errors().is_empty() { SyntaxAcceptance::Accepted } else { SyntaxAcceptance::Rejected },
    );
    let mut walker = Walker { builder: &mut table_builder, nodes: Vec::new(), future };
    let root = AnyNodeRef::from(module);
    if walker.enter_node(root).is_traverse() {
        match module {
            Mod::Module(body) => walker.visit_body(&body.body),
            Mod::Expression(body) => walker.visit_expr(&body.body),
        }
    }
    walker.leave_node(root);
    let nodes = walker.nodes;
    visit(finished(&mut table_builder), &nodes, settings.target_version().components());
}
