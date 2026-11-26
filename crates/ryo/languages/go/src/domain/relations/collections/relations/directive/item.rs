//! The `directive` relation of the language: every build constraint line of a file's header, `//go:build` and `// +build`, by its kind
//! and its expression, as `go/build` reads them.


use crate::domain::constraints::types::Constraint;

use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ryo::languages;

use crate::domain::file_view::types::{GoWalk, Words};
use crate::domain::walked::syntax as walked_syntax;
use ryo::languages::node_table;

/// The reader the `parse` contract hands the `directive` relation to, which reads its rows from a file's tree.
pub(crate) struct DirectiveItem;

// The engine fills `directive` from the walked file: every build constraint line.
ryo::fills! {
    DirectiveItem => languages::Directive, walk(GoWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::Directive as languages::CatalogRelation>::Row;
        let tree = file.syntax;
        let words = Words(std::cell::RefCell::new(words));
        for &at in node_table::ids_of_kind(tree.table, kind_bits(Kind::Token(Token::Comment))) {
            if crate::domain::walked::constraints::constraint_at(&tree, at) == Constraint::Other {
                continue;
            }
            rows.write(Row {
                file: tree.file,
                span: walked_syntax::span_of(&tree, at),
                kind: crate::domain::walked::names::directive_kind(&tree, &words, at),
                text: crate::domain::walked::names::directive_text(&tree, &words, at),
            });
        }
    }
}
