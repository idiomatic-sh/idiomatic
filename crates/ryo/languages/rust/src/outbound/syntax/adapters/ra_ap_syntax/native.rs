//! A Rust file in concrete preorder. Lexical leaves keep the same positions as their
//! parser events. Parsed macro arguments follow the concrete prefix.
use super::adapter::SyntaxRaApSyntaxAdapter;
use crate::outbound::syntax::interface::Output;
use crate::domain::fields::service::{NativeTree, field_bits, field_of};
use crate::domain::syntax::{constants::OPAQUE_MACROS, service::kinds};
use crate::outbound::syntax::types::{OpaqueMacros, SourceText};
use ryo::languages::{self as l, node_table as t};

struct Writer<'a, 'b> {
    table: &'a mut l::NodeTableBuilder<'b>,
    host: Option<l::Position>,
    depth: usize,
    numeric: bool,
    strings: bool,
    record_origins: bool,
    origins: &'a mut Vec<l::ExpandedPosition>,
    flags: &'a mut Vec<l::Flags>,
}

impl Writer<'_, '_> {
    fn begin_root(&mut self) {
        if self.depth == 0
            && let Some(host) = self.host
        {
            let _ = t::embed(
                self.table,
                host,
                l::EmbeddedText {
                    text: l::NameText(""),
                    mapping: l::EmbeddedMapping::Exact(l::Offset(0)),
                },
            );
        }
    }
}

impl Output for Writer<'_, '_> {
    fn string_values(&self) -> bool {
        self.strings
    }
    fn numeric_values(&self) -> bool {
        self.numeric
    }
    fn enter(&mut self, kind: l::KindBits, start: l::Offset) {
        self.flags.push(l::Flags(0));
        self.begin_root();
        t::enter_node(self.table, kind, l::FieldCode(0), start);
        self.depth += 1;
    }
    fn leave(&mut self, end: l::Offset) {
        self.depth -= 1;
        t::leave_node(self.table, end);
    }
    fn token(
        &mut self,
        kind: l::KindBits,
        start: l::Offset,
        end: l::Offset,
        decoded: Option<l::NameText<'_>>,
    ) {
        self.enter(kind, start);
        *self.flags.last_mut().expect("token position") = l::TOKEN;
        if let Some(text) = decoded {
            t::node_property(self.table, l::PropertyId(0), text);
        }
        self.leave(end);
    }
    fn property(&mut self, key: l::PropertyId, value: l::NameText<'_>) {
        t::node_property(self.table, key, value);
    }
    fn formatted(
        &mut self,
        key: l::PropertyId,
        value: std::fmt::Arguments<'_>,
    ) -> std::fmt::Result {
        t::node_property_fmt(self.table, key, value)
    }
    fn diagnostic(&mut self, message: &str, start: l::Offset, end: l::Offset) {
        t::diagnose(self.table, start, end, l::NameText(message));
    }
    fn adopt(&mut self, host: l::Position) {
        self.host = Some(host);
    }
    fn origin(&mut self, element: l::Position, host: l::Position, expansion: l::ExpansionId) {
        if !self.record_origins {
            return;
        }
        self.origins.push(l::ExpandedPosition {
            element,
            call_site: host,
            expansion,
        });
    }
    fn release(&mut self) {
        self.host = None;
    }
}

/// Reads directly into the table that the evaluator borrows. The first origin
/// belongs to the element at `node_table::own_end`.
pub(crate) fn read(
    bytes: &[u8],
    table: &mut l::NodeTable,
    origins: &mut Vec<l::ExpandedPosition>,
    flags: &mut Vec<l::Flags>,
    columns: l::NativeColumns,
    access: l::NodeAccess<'_>,
) {
    read_with(bytes, table, origins, flags, columns, access);
}

fn read_with(
    bytes: &[u8],
    table: &mut l::NodeTable,
    origins: &mut Vec<l::ExpandedPosition>,
    flags: &mut Vec<l::Flags>,
    columns: l::NativeColumns,
    access: l::NodeAccess<'_>,
) {
    origins.clear();
    flags.clear();
    let mut builder = t::builder(table, l::Count(kinds().count()));
    SyntaxRaApSyntaxAdapter::new(OpaqueMacros(&OPAQUE_MACROS)).walk(
        SourceText(bytes),
        &mut Writer {
            table: &mut builder,
            host: None,
            depth: 0,
            record_origins: matches!(access, l::NodeAccess::Inspect(_)),
            strings: columns.notes || match access {
                l::NodeAccess::Inspect(_) => true,
                l::NodeAccess::Query(rows) => rows.iter().any(|relation| matches!(relation,
                    l::Relation::SyntaxTree | l::Relation::SemanticSyntax | l::Relation::TokenLink
                    | l::Relation::StringLiteral | l::Relation::ExpressionShape
                )),
            },
            numeric: match access {
                l::NodeAccess::Inspect(_) => true,
                l::NodeAccess::Query(rows) => rows.iter().any(|relation| {
                    matches!(relation, l::Relation::NumericLiteral | l::Relation::ExpressionShape)
                }),
            },
            origins,
            flags,
        },
    );
    let _ = t::finished(&mut builder);
    if columns.fields {
        t::classify_fields(table, |tree, at| {
            l::FieldCode(field_bits(field_of(&NativeTree { table: tree, flags }, at)).0)
        });
    }
}
