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

    /// An expression statement evaluates its single value expression.
    pub fn visit_expression_statements(&self, mut visit: impl FnMut(Position, Position)) {
        for Numbered { id, node } in self.expression_nodes(Node::Expr) {
            if let AnyNodeRef::StmtExpr(stmt) = node
                && let Some(value) = id_of(&*stmt.value)
            {
                visit(id, table_id(self.file.syntax.tree.segments, value));
            }
        }
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

impl ColumnFile<'_, '_> {
    /// Each adjacent operand pair is compared once in Python's chained-comparison grammar.
    pub fn visit_comparison_operands(&self, mut visit: impl FnMut(Position, Position, Position)) {
        let public =
            |expr: &Expr| id_of(expr).map(|at| table_id(self.file.syntax.tree.segments, at));
        for Numbered { id, node } in self.expression_nodes(Node::Compare) {
            let AnyNodeRef::ExprCompare(c) = node else {
                continue;
            };
            let Some((first, rest)) = c.operands.split_first() else {
                continue;
            };
            let mut left = public(first);
            for right in rest {
                let right = public(right);
                if let (Some(l), Some(r)) = (left, right) {
                    visit(id, l, r);
                }
                left = right;
            }
        }
    }

    /// Each explicit dictionary key is evaluated in display order, excluding mapping-unpack entries.
    pub fn visit_dictionary_keys(&self, mut visit: impl FnMut(Position, Position, usize)) {
        for Numbered { id, node } in self.expression_nodes(Node::Dict) {
            let AnyNodeRef::ExprDict(dict) = node else {
                continue;
            };
            let count = dict.items.iter().filter(|item| item.key.is_some()).count();
            for item in &dict.items {
                if let Some(key) = item.key.as_ref().and_then(id_of) {
                    visit(id, table_id(self.file.syntax.tree.segments, key), count);
                }
            }
        }
    }

    /// For ordinary assignment, names in each nested target are stored left to right, in the same namespace.
    pub fn visit_repeated_assignment_names(
        &self,
        mut visit: impl FnMut(Position, Position, &str, Position, Option<Position>),
    ) {
        fn leaves<'a>(
            expr: &'a Expr,
            direct: bool,
            emit: &mut impl FnMut(&'a ruff_python_ast::ExprName, bool),
        ) {
            match expr {
                Expr::Name(n) => emit(n, direct),
                Expr::Tuple(t) => {
                    for e in &t.elts {
                        leaves(e, false, emit);
                    }
                }
                Expr::List(t) => {
                    for e in &t.elts {
                        leaves(e, false, emit);
                    }
                }
                Expr::Starred(t) => leaves(&t.value, false, emit),
                _ => {}
            }
        }
        let mut seen = std::collections::HashMap::new();
        for Numbered { id, node } in self.expression_nodes(Node::Assign) {
            let AnyNodeRef::StmtAssign(stmt) = node else {
                continue;
            };
            if matches!(
                stmt.targets.as_slice(),
                [Expr::Name(_) | Expr::Attribute(_) | Expr::Subscript(_)]
            ) {
                continue;
            }
            seen.clear();
            for target in &stmt.targets {
                leaves(target, true, &mut |name, direct| {
                    if let Some(at) = id_of(name) {
                        let at = table_id(self.file.syntax.tree.segments, at);
                        let spelling = name.id.as_str();
                        let (previous, first_direct) = seen.entry(spelling).or_insert((None, None));
                        if let Some(previous) = previous.replace(at) {
                            visit(
                                id,
                                at,
                                spelling,
                                previous,
                                if direct { *first_direct } else { None },
                            );
                        }
                        if direct && first_direct.is_none() {
                            *first_direct = Some(at);
                        }
                    }
                });
            }
        }
    }

    /// The next token in the same lexical stream after skipping comments and nonterminating newlines.
    pub fn next_code_token(&self, id: Position) -> Option<Position> {
        use crate::domain::{syntax::service::kind_of, tokens::types::Token};
        for at in id.0 + 1..node_table::own_end(self.table).0 {
            let position = Position(at);
            match kind_of(node_table::kind_of(self.table, position)) {
                Kind::Token(Token::Comment | Token::Nl) | Kind::Extra(_) | Kind::Node(_) => {}
                Kind::Token(Token::Errortoken) => return None,
                Kind::Token(_) => return Some(position),
            }
        }
        None
    }

    /// Consecutive opening delimiter tokens can have comments and nonterminating newlines between them.
    pub fn visit_nested_delimiters(&self, mut visit: impl FnMut(Position, Position)) {
        use crate::domain::{syntax::service::kind_of, tokens::types::Token};
        let end = node_table::own_end(self.table).0;
        for opener in [Token::Lpar, Token::Lsqb, Token::Lbrace] {
            for &id in node_table::ids_of_kind(self.table, kind_bits(Kind::Token(opener))) {
                if id.0 < end
                    && let Some(next) = self.next_code_token(id)
                    && matches!(
                        kind_of(node_table::kind_of(self.table, next)),
                        Kind::Token(Token::Lpar | Token::Lsqb | Token::Lbrace)
                    )
                {
                    visit(id, next);
                }
            }
        }
    }
}
