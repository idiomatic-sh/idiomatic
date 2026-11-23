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
    pub fn visit_comparison_operands(
        &self,
        mut visit: impl FnMut(Position, Position, Position, &'static str, usize),
    ) {
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
            for (right, op) in rest.iter().zip(c.ops.iter()) {
                let right = public(right);
                if let (Some(l), Some(r)) = (left, right) {
                    visit(id, l, r, op.as_str(), c.ops.len());
                }
                left = right;
            }
        }
    }

    /// Python evaluates adjacent operands of an and/or expression left to right, until it short circuits.
    pub fn visit_boolean_operands(
        &self,
        mut visit: impl FnMut(Position, Position, Position, &'static str, usize),
    ) {
        let segments = self.file.syntax.tree.segments;
        for Numbered { id, node } in self.expression_nodes(Node::BoolOp) {
            let AnyNodeRef::ExprBoolOp(expr) = node else {
                continue;
            };
            for pair in expr.values.windows(2) {
                if let (Some(left), Some(right)) = (id_of(&pair[0]), id_of(&pair[1])) {
                    visit(
                        id,
                        table_id(segments, left),
                        table_id(segments, right),
                        expr.op.as_str(),
                        expr.values.len(),
                    );
                }
            }
        }
    }

    /// A with statement's header ends at the colon after its final context manager, before its suite.
    pub fn visit_with_headers(&self, mut visit: impl FnMut(Position, Position)) {
        use crate::domain::tokens::types::Token;
        let colons = node_table::ids_of_kind(self.table, kind_bits(Kind::Token(Token::Colon)));
        let own_end = node_table::own_end(self.table).0;
        let colons = &colons[..colons.partition_point(|id| id.0 < own_end)];
        for kind in [Node::With, Node::AsyncWith] {
            for Numbered { id, node } in self.expression_nodes(kind) {
                let AnyNodeRef::StmtWith(stmt) = node else {
                    continue;
                };
                let Some(item) = stmt.items.last().and_then(id_of) else {
                    continue;
                };
                let Some(body) = stmt.body.first().and_then(id_of) else {
                    continue;
                };
                let segments = self.file.syntax.tree.segments;
                let begin = node_table::span_of(self.table, table_id(segments, item)).end;
                let end = node_table::span_of(self.table, table_id(segments, body)).start;
                let index = colons
                    .partition_point(|&c| node_table::span_of(self.table, c).start.0 < begin.0);
                if let Some(&c) = colons.get(index)
                    && node_table::span_of(self.table, c).end.0 <= end.0
                {
                    visit(id, c);
                }
            }
        }
    }

    /// The elements of a tuple, list or set display are evaluated in written order, including starred expressions.
    pub fn visit_display_elements(&self, mut visit: impl FnMut(Position, Position, usize, usize)) {
        for kind in [Node::Tuple, Node::List, Node::Set] {
            for Numbered { id, node } in self.expression_nodes(kind) {
                let elements = match node {
                    AnyNodeRef::ExprTuple(t) => &t.elts,
                    AnyNodeRef::ExprList(l) => &l.elts,
                    AnyNodeRef::ExprSet(s) => &s.elts,
                    _ => continue,
                };
                for (index, element) in elements.iter().enumerate() {
                    if let Some(place) = id_of(element) {
                        visit(
                            id,
                            table_id(self.file.syntax.tree.segments, place),
                            index,
                            elements.len(),
                        );
                    }
                }
            }
        }
    }

    /// Positional arguments in a call retain their written order, including starred expressions.
    pub fn visit_positional_arguments(&self, mut visit: impl FnMut(Position, Position, usize)) {
        for Numbered { id: owner, node } in self.expression_nodes(Node::Call) {
            let AnyNodeRef::ExprCall(call) = node else {
                continue;
            };
            for (ordinal, expr) in call.arguments.args.iter().enumerate() {
                if let Some(id) = id_of(expr) {
                    visit(owner, table_id(self.file.syntax.tree.segments, id), ordinal);
                }
            }
        }
    }

    /// A name or attribute has itself and each uninterrupted receiver prefix, with the number of expressions in that path.
    pub fn visit_attribute_prefixes(&self, mut visit: impl FnMut(Position, Position, usize)) {
        let segments = self.file.syntax.tree.segments;
        for kind in [Node::Name, Node::Attribute] {
            for Numbered { id, node } in self.expression_nodes(kind) {
                visit(id, id, 1);
                let mut current = node;
                let mut depth = 1;
                while let AnyNodeRef::ExprAttribute(attr) = current {
                    let value = attr.value.as_ref();
                    if !matches!(value, Expr::Name(_) | Expr::Attribute(_)) {
                        break;
                    }
                    let Some(place) = id_of(value) else { break };
                    depth += 1;
                    visit(table_id(segments, place), id, depth);
                    current = AnyNodeRef::from(value);
                }
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

    /// A later explicit constant key replaces the earlier equal key in the same dictionary display.
    pub fn visit_repeated_constant_keys(
        &mut self,
        mut visit: impl FnMut(Position, Position, Position),
    ) {
        use crate::domain::relations::collections::relations::constant_key::item::keyed_constants;
        let has_multiple_keys = self.expression_nodes(Node::Dict).any(|item| {
            let AnyNodeRef::ExprDict(dict) = item.node else {
                return false;
            };
            dict.items
                .iter()
                .filter_map(|item| item.key.as_ref())
                .nth(1)
                .is_some()
        });
        if !has_multiple_keys {
            return;
        }
        keyed_constants(self.file, self.words);
        let classes = &self.file.buffers.keys.classes;
        let segments = self.file.syntax.tree.segments;
        let mut previous = std::collections::HashMap::new();
        for Numbered { id, node } in self.expression_nodes(Node::Dict) {
            let AnyNodeRef::ExprDict(dict) = node else {
                continue;
            };
            previous.clear();
            for key in dict
                .items
                .iter()
                .filter_map(|item| item.key.as_ref().and_then(id_of))
            {
                if let Some(class) = classes.get(usize::from(key)).copied().flatten()
                    && let Some(earlier) = previous.insert(class.representative, key)
                {
                    visit(id, table_id(segments, key), table_id(segments, earlier));
                }
            }
        }
    }

    /// A string literal in the syntax subtree of an annotation or return annotation has that enclosing source annotation.
    pub fn visit_annotation_strings(&self, mut visit: impl FnMut(Position, Position)) {
        use crate::domain::labels::{service::field_bits, types::Field};
        let strings = node_table::ids_of_kind(self.table, kind_bits(Kind::Node(Node::Constant)));
        for field in [Field::Annotation, Field::Returns] {
            let code = ryo::languages::FieldCode(field_bits(field).0);
            for &root in node_table::ids_of_field(self.table, code) {
                let end = node_table::subtree_end_of(self.table, root);
                let start = strings.partition_point(|id| id.0 < root.0);
                for &id in strings[start..].iter().take_while(|id| id.0 < end.0) {
                    let tree = self.file.syntax.tree;
                    if let Some(place) = place_of_id(tree, id)
                        && matches!(
                            tree.nodes.get(usize::from(place)),
                            Some(AnyNodeRef::ExprStringLiteral(_))
                        )
                    {
                        visit(id, root);
                    }
                }
            }
        }
    }

    /// A subscription evaluates its receiver and slice; a bare receiver has an identifier spelling.
    pub fn visit_subscript_operands(
        &self,
        mut visit: impl FnMut(Position, Position, Position, Option<&str>),
    ) {
        for Numbered { id, node } in self.expression_nodes(Node::Subscript) {
            if let AnyNodeRef::ExprSubscript(sub) = node
                && let (Some(value), Some(slice)) = (id_of(&*sub.value), id_of(&*sub.slice))
            {
                let name = match &*sub.value {
                    Expr::Name(n) => Some(n.id.as_str()),
                    _ => None,
                };
                let segments = self.file.syntax.tree.segments;
                visit(
                    id,
                    table_id(segments, value),
                    table_id(segments, slice),
                    name,
                );
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
