//! Read proven source origins without importing any linter's matching rules.
use crate::domain::{
    model::types::Declarations,
    nodes::types::Node,
    qualification::{
        constants::INCOMPLETE,
        service::{complete, qualify},
    },
    scopes::{service::context, types::Context},
    syntax::{
        service::{kind, kind_bits},
        types::Kind,
    },
};
use ryo::languages;
/// Python load-expression provenance.
pub(crate) struct QualifiedNameItem;
ryo::reads! {
    QualifiedNameItem => languages::QualifiedName, model(Declarations) |tree, model, at, symbols| {
        kinds:[Kind::Node(Node::Module)] as kind_bits,
        when complete(tree, model) {
            None=>{partial INCOMPLETE => languages::Missing::Any;}
            Some(index)=>{
                each (languages::positions(tree.elements)) as expression {
                    require matches!(kind(tree, expression), Kind::Node(Node::Name | Node::Attribute));
                    require context(tree, expression) == Context::Load;
                    given answer = qualify(tree, model, &index, expression, symbols);
                    row {
                        id:node(expression),path:value(answer.path),provenance:value(answer.provenance),
                        since_major:value(answer.since.major),since_minor:value(answer.since.minor),
                        file:file,span:span(expression),
                    }
                }
            }
        }
    }
}
