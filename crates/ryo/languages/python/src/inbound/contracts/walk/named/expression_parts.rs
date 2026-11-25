//! Ordered operands of binary expressions and chained comparisons.
use super::{NamedFile, Kind, Node, Numbered, qualify_cells};
use crate::domain::node::service::{id_of, place_of_id, table_id};
use crate::domain::syntax::service::kind_bits;
use ruff_python_ast::{AnyNodeRef, Expr, Operator};
use ryo::languages::{self as l, CellKind, FileRelation, Position, node_table};

const BINARY_OPERANDS: &str = "binary-operands";
const COMPARISONS: &str = "comparison-operands";

impl NamedFile<'_, '_> {
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

    /// Python's binary-expression grammar fixes the operator and its two ordered operands.
    fn visit_binary_operands(
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

    /// Each adjacent operand pair is compared once in Python's chained-comparison grammar.
    fn visit_comparison_operands(
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

    /// File-scoped projections of Python operator syntax.
    pub(crate) const EXPRESSION_SCHEMAS: &'static [FileRelation] = &[
        FileRelation {
            name: BINARY_OPERANDS,
            columns: &[
                ("id", CellKind::Node),
                ("left", CellKind::Node),
                ("operator", CellKind::Symbol),
                ("right", CellKind::Node),
                ("file", CellKind::File),
            ],
        },
        FileRelation {
            name: COMPARISONS,
            columns: &[
                ("id", CellKind::Node),
                ("left", CellKind::Node),
                ("right", CellKind::Node),
                ("operator", CellKind::Symbol),
                ("count", CellKind::Number),
                ("file", CellKind::File),
            ],
        },
    ];

    pub(super) fn named_expression(
        &self,
        schema: &FileRelation,
        file: l::FileId,
        bytes: l::ByteSlice<'_>,
        symbols: &mut l::Symbols,
    ) -> l::FileRows {
        let mut columns = vec![Vec::new(); schema.columns.len()];
        let file_id = u64::from(file.0);
        let mut write = |row: &[u64]| {
            debug_assert_eq!(columns.len(), row.len());
            for (out, &cell) in columns.iter_mut().zip(row) {
                out.push(cell);
            }
        };
        let mut word = |text| u64::from(l::intern_derived(symbols, bytes, l::NameText(text)).0);
        match schema.name {
            BINARY_OPERANDS => self.visit_binary_operands(|id, left, op, right| {
                write(&[
                    id.0.into(),
                    left.0.into(),
                    word(op),
                    right.0.into(),
                    file_id,
                ]);
            }),
            COMPARISONS => self.visit_comparison_operands(|id, left, right, op, count| {
                write(&[
                    id.0.into(),
                    left.0.into(),
                    right.0.into(),
                    word(op),
                    count as u64,
                    file_id,
                ]);
            }),
            _ => unreachable!("checked expression schema"),
        }
        l::FileRows {
            columns: qualify_cells(schema, file, columns),
            partial: None,
        }
    }
}
