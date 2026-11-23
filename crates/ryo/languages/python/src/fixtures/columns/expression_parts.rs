//! Syntactic operands at calls and binary expressions.
use super::{ColumnFile, Kind, Node, Numbered};
use crate::domain::node::service::{id_of, place_of_id, table_id};
use crate::domain::syntax::service::kind_bits;
use ruff_python_ast::{AnyNodeRef, Expr, Operator};
use ryo::languages::{Position, node_table};

/// A call expression, with the positional list separate from its keyword arguments.
#[derive(Debug)]
pub struct CallSite<'a> {
    /// The call and its callee.
    pub call: Position,
    /// The expression evaluated to obtain the callable.
    pub function: Position,
    /// The identifier for a simple name, or the empty string.
    pub name: &'a str,
    /// The name on an attribute access, or the empty string.
    pub attribute: &'a str,
    /// The object expression on an attribute access.
    pub receiver: Option<Position>,
    /// Positional arguments, including starred expressions.
    pub positional: usize,
    /// Keyword arguments, including double-star unpacking.
    pub keywords: usize,
    /// Whether the call belongs to a parsed string annotation.
    pub string_annotation: bool,
    /// The first positional expression, when present.
    pub first: Option<Position>,
}

impl ColumnFile<'_, '_> {
    // Embedded expressions have typed-tree places distinct from their public node ids.
    fn expression_nodes(&self, kind: Node) -> impl Iterator<Item = Numbered<'_>> {
        let tree = self.file.syntax.tree;
        node_table::ids_of_kind(self.table, kind_bits(Kind::Node(kind)))
            .iter()
            .filter_map(move |&id| {
                let at = place_of_id(tree, id)?;
                Some(Numbered {
                    id,
                    node: *tree.nodes.get(usize::from(at))?,
                })
            })
    }

    /// A Python call has one callee and ordered positional and keyword argument lists.
    pub fn visit_call_sites(&self, mut visit: impl FnMut(CallSite<'_>)) {
        for Numbered { id, node } in self.expression_nodes(Node::Call) {
            let AnyNodeRef::ExprCall(call) = node else {
                continue;
            };
            let Some(function) = id_of(&*call.func) else {
                continue;
            };
            let (name, attribute, receiver) = match &*call.func {
                Expr::Name(name) => (name.id.as_str(), "", None),
                Expr::Attribute(attribute) => {
                    ("", attribute.attr.as_str(), id_of(&*attribute.value))
                }
                _ => ("", "", None),
            };
            visit(CallSite {
                call: id,
                function: table_id(self.file.syntax.tree.segments, function),
                name,
                attribute,
                receiver: receiver.map(|at| table_id(self.file.syntax.tree.segments, at)),
                string_annotation: self
                    .file
                    .syntax
                    .tree
                    .segments
                    .first()
                    .is_some_and(|s| s.root <= id),
                positional: call.arguments.args.len(),
                keywords: call.arguments.keywords.len(),
                first: call
                    .arguments
                    .args
                    .first()
                    .and_then(id_of)
                    .map(|at| table_id(self.file.syntax.tree.segments, at)),
            });
        }
    }

    /// Python's binary-expression grammar fixes the operator and its two ordered operands.
    pub fn visit_binary_operands(
        &self,
        mut visit: impl FnMut(Position, Position, &'static str, Position),
    ) {
        for Numbered { id, node } in self.expression_nodes(Node::BinOp) {
            let AnyNodeRef::ExprBinOp(expr) = node else {
                continue;
            };
            let (Some(left), Some(right)) = (id_of(&*expr.left), id_of(&*expr.right)) else {
                continue;
            };
            let op = match expr.op {
                Operator::Add => "+",
                Operator::Sub => "-",
                Operator::Mult => "*",
                Operator::MatMult => "@",
                Operator::Div => "/",
                Operator::Mod => "%",
                Operator::Pow => "**",
                Operator::LShift => "<<",
                Operator::RShift => ">>",
                Operator::BitOr => "|",
                Operator::BitXor => "^",
                Operator::BitAnd => "&",
                Operator::FloorDiv => "//",
            };
            visit(
                id,
                table_id(self.file.syntax.tree.segments, left),
                op,
                table_id(self.file.syntax.tree.segments, right),
            );
        }
    }
}
