//! The `static_call` relation of the language: every call, by the dotted name it calls; a callee that is no dotted name has the unknown
//! value of a dynamic callee. A call of a type written out (`[]byte(s)`) converts, and has no row.


use crate::domain::names::types::{Callee, Unresolved};
use crate::domain::nodes::types::Node;

use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

use crate::domain::file_view::types::{GoWalk, Words};
use crate::domain::walked::syntax as walked_syntax;
use ryo::languages::node_table;

/// The reader the `parse` contract hands the `static_call` relation to, which reads its rows from a file's tree.
pub(crate) struct StaticCallItem;

// The engine fills `static_call` from the walked file: each call, with the name it calls or the unknown value of a callee that is a value.
ryo::fills! {
    StaticCallItem => languages::StaticCall, walk(GoWalk) |file, words, rows| {
        /// One row of the relation.
        type Row = <languages::StaticCall as languages::CatalogRelation>::Row;
        let tree = file.syntax;
        let words = Words(std::cell::RefCell::new(words));
        for &at in node_table::ids_of_kind(tree.table, kind_bits(Kind::Node(Node::CallExpr))) {
            let Some(callee) = crate::domain::walked::syntax::nodes(&tree, at).next() else { continue };
            let target = match crate::domain::walked::names::callee_of(&tree, callee) {
                Callee::Named => {
                    let Some(target) = crate::domain::walked::names::dotted(&tree, &words, callee) else { continue };
                    target
                }
                Callee::Dynamic => crate::domain::walked::names::unknown(Unresolved::DynamicCallee),
                Callee::Conversion => continue,
            };
            rows.write(Row {
                occurrence: walked_syntax::node_cell(&tree, at),
                owner: crate::domain::walked::scopes::owner(&tree, at),
                target,
                file: tree.file,
                span: walked_syntax::span_of(&tree, callee),
            });
        }
    }
}
