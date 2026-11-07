//! The `static_import` relation of the language: each name an `import` or a `from ... import` brings in, by the path it imports, the
//! dots of a relative import kept; `from m import *` imports the module `m`, its target, and binds names it cannot list: its binding is the
//! unknown value of a wildcard import.

use crate::domain::imports::service::{import_binding, import_target};
use crate::domain::model::types::Declarations;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service::owner;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// Reads the `static-import` rows of one Python file from its syntax tree.
pub(crate) struct StaticImportItem;

ryo::reads! {
    StaticImportItem => languages::StaticImport, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Alias)] as kind_bits,
        given binding = import_binding(tree, at)?;
        given target = import_target(tree, symbols, at)?;
        row {
            occurrence: node(at),
            owner: value(owner(tree, at)),
            binding: value(binding),
            target: value(target),
            file: file,
            span: span(at),
        }
    }
}
