//! How each kind of the language's trees is shown: the source whose tree holds it, or the reason no relation reads it.

use ryo::languages::contract::KindSample;
use ryo::languages::{KindName, Sample, SampleText, Why};

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::JavaScriptNode;
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
const fn node_sample(node: JavaScriptNode) -> Sample {
    match node {
        JavaScriptNode::Program => Sample::Node(SampleText("")),
        JavaScriptNode::Literal => Sample::Node(SampleText("const a = 1;")),
        JavaScriptNode::FunctionDeclaration => Sample::Node(SampleText("function f(){return 1;}")),
        JavaScriptNode::FunctionExpression => Sample::Node(SampleText("const f=function named(){};")),
        JavaScriptNode::ArrowFunctionExpression => Sample::Node(SampleText("const f=(x)=>x+1;")),
        JavaScriptNode::NewExpression => Sample::Node(SampleText("new Box(value);")),
        JavaScriptNode::CallExpression => Sample::Node(SampleText("object.method(value);")),
        JavaScriptNode::ImportDeclaration => Sample::Node(SampleText(
            "import value, {named as local} from \"pkg\"; import * as ns from \"other\";",
        )),
        JavaScriptNode::Identifier
        | JavaScriptNode::PrivateIdentifier
        | JavaScriptNode::ExpressionStatement
        | JavaScriptNode::BlockStatement
        | JavaScriptNode::EmptyStatement
        | JavaScriptNode::DebuggerStatement
        | JavaScriptNode::WithStatement
        | JavaScriptNode::ReturnStatement
        | JavaScriptNode::LabeledStatement
        | JavaScriptNode::BreakStatement
        | JavaScriptNode::ContinueStatement
        | JavaScriptNode::IfStatement
        | JavaScriptNode::SwitchStatement
        | JavaScriptNode::SwitchCase
        | JavaScriptNode::ThrowStatement
        | JavaScriptNode::TryStatement
        | JavaScriptNode::CatchClause
        | JavaScriptNode::WhileStatement
        | JavaScriptNode::DoWhileStatement
        | JavaScriptNode::ForStatement
        | JavaScriptNode::ForInStatement
        | JavaScriptNode::ForOfStatement
        | JavaScriptNode::VariableDeclaration
        | JavaScriptNode::VariableDeclarator
        | JavaScriptNode::ThisExpression
        | JavaScriptNode::Super
        | JavaScriptNode::ArrayExpression
        | JavaScriptNode::ObjectExpression
        | JavaScriptNode::Property
        | JavaScriptNode::SequenceExpression
        | JavaScriptNode::UnaryExpression
        | JavaScriptNode::BinaryExpression
        | JavaScriptNode::AssignmentExpression
        | JavaScriptNode::UpdateExpression
        | JavaScriptNode::LogicalExpression
        | JavaScriptNode::ConditionalExpression
        | JavaScriptNode::MemberExpression
        | JavaScriptNode::YieldExpression
        | JavaScriptNode::AwaitExpression
        | JavaScriptNode::TemplateLiteral
        | JavaScriptNode::TaggedTemplateExpression
        | JavaScriptNode::TemplateElement
        | JavaScriptNode::SpreadElement
        | JavaScriptNode::RestElement
        | JavaScriptNode::AssignmentPattern
        | JavaScriptNode::ArrayPattern
        | JavaScriptNode::ObjectPattern
        | JavaScriptNode::ClassDeclaration
        | JavaScriptNode::ClassExpression
        | JavaScriptNode::ClassBody
        | JavaScriptNode::MethodDefinition
        | JavaScriptNode::PropertyDefinition
        | JavaScriptNode::StaticBlock
        | JavaScriptNode::MetaProperty
        | JavaScriptNode::ImportSpecifier
        | JavaScriptNode::ImportDefaultSpecifier
        | JavaScriptNode::ImportNamespaceSpecifier
        | JavaScriptNode::ImportExpression
        | JavaScriptNode::ImportAttribute
        | JavaScriptNode::ExportNamedDeclaration
        | JavaScriptNode::ExportDefaultDeclaration
        | JavaScriptNode::ExportAllDeclaration
        | JavaScriptNode::ExportSpecifier
        | JavaScriptNode::ChainExpression
        | JavaScriptNode::JSXElement
        | JavaScriptNode::JSXOpeningElement
        | JavaScriptNode::JSXClosingElement
        | JavaScriptNode::JSXFragment
        | JavaScriptNode::JSXOpeningFragment
        | JavaScriptNode::JSXClosingFragment
        | JavaScriptNode::JSXNamespacedName
        | JavaScriptNode::JSXMemberExpression
        | JavaScriptNode::JSXExpressionContainer
        | JavaScriptNode::JSXEmptyExpression
        | JavaScriptNode::JSXAttribute
        | JavaScriptNode::JSXSpreadAttribute
        | JavaScriptNode::JSXIdentifier
        | JavaScriptNode::JSXSpreadChild
        | JavaScriptNode::JSXText => Sample::Unread(Why(
            "An ESTree structure retained by syntax_tree and syntax_child; semantic readers dispatch on its enclosing construct.",
        )),
    }
}
