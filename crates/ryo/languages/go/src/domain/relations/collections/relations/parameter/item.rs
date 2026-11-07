//! The `parameter` relation of the language: the receiver of a method, each parameter of a function, a method or a function literal,
//! and each of its named results, each in its place in its list. A parameter the declaration does not name has the unknown value of
//! that for its name.

use crate::domain::nodes::types::Node;
use crate::domain::signatures::service::parameters;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `parameter` relation to, which reads its rows from a file's tree.
pub(crate) struct ParameterItem;

ryo::reads! {
    ParameterItem => languages::Parameter |tree, at, symbols| {
        kinds: [Kind::Node(Node::Field)] as kind_bits,
        each (parameters(tree, at)) as parameter {
            given refinement = languages::intern_derived(symbols, tree.source, languages::NameText(parameter.word.0));
            row {
                id: node(parameter.named),
                kind: value(parameter.kind),
                refinement: value(refinement),
                name: value(parameter.name),
                ordinal: value(parameter.ordinal),
                owner: node(parameter.callable),
                file: file,
                span: span(parameter.named),
            }
        }
    }
}
