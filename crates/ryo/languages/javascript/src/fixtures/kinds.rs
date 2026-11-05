//! How each kind of the language's trees is shown: the source whose tree holds it, or the reason no relation reads it.

use ryo::languages::contract::KindSample;
use ryo::languages::{KindName, Sample, SampleText, Why};

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::constants::UNREAD_TEXT;
use crate::domain::syntax::service::{kind_bits, kind_name, kinds};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// Every kind, in the order of their numbers, with how the language shows it.
pub(super) fn shown() -> Vec<KindSample> {
    kinds()
        .map(|kind| KindSample {
            kind: KindName(kind_name(kind).0.into()),
            bits: kind_bits(kind),
            sample: sample(kind),
        })
        .collect()
}

/// How the language shows a kind of its trees: the source whose tree holds it, for a kind a relation reads, and for every other the
/// reason no relation models it. The match has no wildcard arm, so a kind the
/// grammar gains is a compile error until it is shown.
#[must_use]
pub(crate) const fn sample(kind: Kind) -> Sample {
    match kind {
        Kind::Node(node) => node_sample(node),
        Kind::Token(Token::Line) => Sample::Token(SampleText("// café\n")),
        Kind::Token(Token::Block) => Sample::Token(SampleText("/* café */")),
        Kind::Token(
            Token::Identifier
            | Token::PrivateIdentifier
            | Token::Keyword
            | Token::Numeric
            | Token::String
            | Token::RegularExpression
            | Token::Template
            | Token::Punctuator
            | Token::JSXIdentifier
            | Token::JSXText,
        ) => Sample::Unread(Why(
            "A lexical leaf retained by the lossless tree; semantic readers consume its AST parent.",
        )),
        Kind::Extra(Extra::Whitespace) => Sample::Unread(Why(
            "Inter-token trivia remains in the lossless syntax tree.",
        )),
        Kind::Extra(Extra::Error) => Sample::Unread(Why(UNREAD_TEXT)),
    }
}
/// Semantic dispatch kinds have examples; syntax-only `ESTree` kinds are proved by complete adapter trees.
const fn node_sample(node: Node) -> Sample {
    match node {
        Node::Program => Sample::Node(SampleText("")),
        Node::Literal => Sample::Node(SampleText("const a = 1;")),
        Node::FunctionDeclaration => Sample::Node(SampleText("function f(){return 1;}")),
        Node::FunctionExpression => Sample::Node(SampleText("const f=function named(){};")),
        Node::ArrowFunctionExpression => Sample::Node(SampleText("const f=(x)=>x+1;")),
        Node::NewExpression => Sample::Node(SampleText("new Box(value);")),
        Node::CallExpression => Sample::Node(SampleText("object.method(value);")),
        Node::ImportDeclaration => Sample::Node(SampleText(
            "import value, {named as local} from \"pkg\"; import * as ns from \"other\";",
        )),
        Node::Identifier
        | Node::PrivateIdentifier
        | Node::ExpressionStatement
        | Node::BlockStatement
        | Node::EmptyStatement
        | Node::DebuggerStatement
        | Node::WithStatement
        | Node::ReturnStatement
        | Node::LabeledStatement
        | Node::BreakStatement
        | Node::ContinueStatement
        | Node::IfStatement
        | Node::SwitchStatement
        | Node::SwitchCase
        | Node::ThrowStatement
        | Node::TryStatement
        | Node::CatchClause
        | Node::WhileStatement
        | Node::DoWhileStatement
        | Node::ForStatement
        | Node::ForInStatement
        | Node::ForOfStatement
        | Node::VariableDeclaration
        | Node::VariableDeclarator
        | Node::ThisExpression
        | Node::Super
        | Node::ArrayExpression
        | Node::ObjectExpression
        | Node::Property
        | Node::SequenceExpression
        | Node::UnaryExpression
        | Node::BinaryExpression
        | Node::AssignmentExpression
        | Node::UpdateExpression
        | Node::LogicalExpression
        | Node::ConditionalExpression
        | Node::MemberExpression
        | Node::YieldExpression
        | Node::AwaitExpression
        | Node::TemplateLiteral
        | Node::TaggedTemplateExpression
        | Node::TemplateElement
        | Node::SpreadElement
        | Node::RestElement
        | Node::AssignmentPattern
        | Node::ArrayPattern
        | Node::ObjectPattern
        | Node::ClassDeclaration
        | Node::ClassExpression
        | Node::ClassBody
        | Node::MethodDefinition
        | Node::PropertyDefinition
        | Node::StaticBlock
        | Node::MetaProperty
        | Node::ImportSpecifier
        | Node::ImportDefaultSpecifier
        | Node::ImportNamespaceSpecifier
        | Node::ImportExpression
        | Node::ImportAttribute
        | Node::ExportNamedDeclaration
        | Node::ExportDefaultDeclaration
        | Node::ExportAllDeclaration
        | Node::ExportSpecifier
        | Node::ChainExpression
        | Node::JSXElement
        | Node::JSXOpeningElement
        | Node::JSXClosingElement
        | Node::JSXFragment
        | Node::JSXOpeningFragment
        | Node::JSXClosingFragment
        | Node::JSXNamespacedName
        | Node::JSXMemberExpression
        | Node::JSXExpressionContainer
        | Node::JSXEmptyExpression
        | Node::JSXAttribute
        | Node::JSXSpreadAttribute
        | Node::JSXIdentifier
        | Node::JSXSpreadChild
        | Node::JSXText => Sample::Unread(Why(
            "An ESTree structure retained by syntax_tree and syntax_child; semantic readers dispatch on its enclosing construct.",
        )),
    }
}
