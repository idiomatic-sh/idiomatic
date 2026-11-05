//! The `static_import` relation of the language: every import, by the path it imports. Its binding is the name it gives the package;
//! an import without one binds the name the package declares, which the file does not say, and `import . "p"` binds every exported
//! name of `p`: each has the unknown value of that.

use crate::domain::names::service::{import_binding, import_target};
use crate::domain::nodes::types::GoNode;
use crate::domain::scopes::service::owner;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `static_import` relation to, which reads its rows from a file's tree.
pub(crate) struct StaticImportItem;

ryo::reads! {
    StaticImportItem => languages::StaticImport |tree, at, symbols| {
        kinds: [Kind::Node(GoNode::ImportSpec)] as kind_bits,
        given target = import_target(tree, symbols, at)?;
        row {
            occurrence: node(at),
            owner: value(owner(tree, at)),
            binding: value(import_binding(tree, at)),
            target: value(target),
            file: file,
            span: span(at),
        }
    }
}
