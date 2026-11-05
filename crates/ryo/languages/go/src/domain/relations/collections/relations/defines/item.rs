//! The `defines` relation of the language: every name a file declares or brings in: functions and methods, types, constants and
//! variables, imported names, fields and methods of types, parameters, results and receivers, type parameters, and the names a short
//! variable declaration or a `range` clause declares, each in the function that holds it. The blank identifier declares nothing. An
//! import of `.` brings in names it cannot list: its name is the unknown value of a dot import.

use crate::domain::definitions::service::{declarations, declared_name, word};
use crate::domain::nodes::types::GoNode;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The reader the `parse` contract hands the `defines` relation to, which reads its rows from a file's tree.
pub(crate) struct DefinesItem;

ryo::reads! {
    DefinesItem => languages::Defines |tree, at, symbols| {
        kinds: [
            Kind::Node(GoNode::FuncDecl),
            Kind::Node(GoNode::TypeSpec),
            Kind::Node(GoNode::ValueSpec),
            Kind::Node(GoNode::ImportSpec),
            Kind::Node(GoNode::Field),
            Kind::Node(GoNode::AssignStmt),
            Kind::Node(GoNode::RangeStmt),
            Kind::Node(GoNode::LabeledStmt),
        ] as kind_bits,
        each (declarations(tree, at)) as declared {
            given name = declared_name(tree, &declared)?;
            row {
                id: node(at),
                kind: value(declared.defined.kind),
                refinement: value(word(tree, symbols, declared.defined.word)),
                name: value(name),
                owner: node(declared.owner),
                file: file,
                span: span(declared.named),
            }
        }
    }
}
