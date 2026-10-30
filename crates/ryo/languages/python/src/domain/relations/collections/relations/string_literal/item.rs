//! The `string_literal` relation of the language: every string, f-string and t-string, its parts joined as the library decoded them;
//! one that interpolates has the unknown value of an interpolated string. A bytes `Constant` is no string, and the format spec of an
//! interpolation is part of its f-string. The name `__import__(..)` or `importlib.import_module(..)` loads is an import specifier.

use crate::domain::fact_notes::service::specifier;
use crate::domain::model::types::Declarations;
use crate::domain::names::service::string_or_unknown;
use crate::domain::nodes::types::Node;
use crate::domain::scopes::service::owner;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::service::{kind, literal, standing};
use crate::domain::syntax::types::Kind;
use crate::domain::syntax::types::{Literal, Standing};
use ryo::languages;

/// Reads the `string-literal` rows of one Python file from its syntax tree.
pub(crate) struct StringLiteralItem;

ryo::reads! {
    StringLiteralItem => languages::StringLiteral, model(Declarations) |tree, model, at, symbols| {
        kinds: [Kind::Node(Node::Constant), Kind::Node(Node::JoinedStr), Kind::Node(Node::TemplateStr)] as kind_bits,
        require literal(tree, at) == Literal::String || kind(tree, at) != Kind::Node(Node::Constant);
        require standing(tree, at) == Standing::Own;
        row {
            occurrence: node(at),
            owner: value(owner(tree, at)),
            value: value(string_or_unknown(tree, symbols, at)),
            file: file,
            span: span(at),
            import: value(specifier(tree, at)),
        }
    }
}
