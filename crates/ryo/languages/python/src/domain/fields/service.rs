//! The `fields` service: free functions named by verb that say the role of an element under its parent, by the field of
//! `Python.asdl` that holds it. The library's source-order walk writes each child without its field, so a role is read from the kinds
//! of the parent and the element, from where the element stands among the nodes under the parent, and from the keyword or the
//! punctuation before it (`else`, `finally`, `from`, `as`, `:`, `/`, `*`), as `CPython`'s parser tells the fields apart. The adapter's
//! cases hold every role to what `CPython`'s `ast.dump` prints for the pinned files.

use ryo::languages::{self, Position, Tree};

use crate::domain::extras::types::Extra;
use crate::domain::labels::service::field_bits;
use crate::domain::labels::types::Field;
use crate::domain::nodes::types::{Category, Node};
use crate::domain::siblings::constants::{AS, COLON, ELSE, EQUAL, FROM, IF, IN};
use crate::domain::siblings::service::{
    before, block, index, last_word, parameter, placement, preceding, statement_block, token_under,
    within,
};
use crate::domain::siblings::types::{Placement, SiblingIndex};
use crate::domain::syntax::service::{category, is_root, is_token, kind, parent};
use crate::domain::syntax::types::{Kind, Leaf, Rootness};
use crate::domain::tokens::types::Token;

/// The first of two roles for an element of the kind `wanted`, the second for every other.
fn choose(element: Kind, wanted: Kind, matched: Field, other: Field) -> Field {
    if element == wanted { matched } else { other }
}

/// The role of a statement, `body`, or the other role for every other node.
fn statement_or(of: Category, other: Field) -> Field {
    if of == Category::Statement {
        Field::Body
    } else {
        other
    }
}

/// The first of two roles for the first node under the parent, the second for every later one.
fn first_or(tree: &Tree<'_>, at: Position, first: Field, later: Field) -> Field {
    if index(tree, at) == SiblingIndex(0) {
        first
    } else {
        later
    }
}

/// The role of a node under a definition, `def` or `class`: its decorators, name, type parameters, parameters, body and return
/// annotation.
fn definition(element: Kind, of: Category) -> Field {
    if element == Kind::Extra(Extra::Decorator) {
        Field::DecoratorList
    } else if element == Kind::Extra(Extra::Identifier) {
        Field::Name
    } else if element == Kind::Extra(Extra::TypeParams) {
        Field::TypeParams
    } else if element == Kind::Node(Node::Arguments) {
        Field::Args
    } else if of == Category::Statement {
        Field::Body
    } else if of == Category::Expression {
        Field::Returns
    } else {
        Field::None
    }
}

/// The role of the node at `at` under a node of the standard, `holder`, by the field of `holder` that holds it.
#[expect(
    clippy::too_many_lines,
    reason = "one arm for each node of the standard, the fields of each in it: the table is the mapping, and splitting it would scatter it"
)]
fn node_under(tree: &Tree<'_>, at: Position, holder: Node) -> Field {
    let element = kind(tree, at);
    let of = category(tree, at);
    match holder {
        Node::Module | Node::Interactive | Node::Expression => Field::Body,
        Node::FunctionDef | Node::AsyncFunctionDef => definition(element, of),
        Node::ClassDef => choose(
            element,
            Kind::Extra(Extra::CallArguments),
            Field::None,
            definition(element, of),
        ),
        Node::Return
        | Node::Expr
        | Node::Await
        | Node::Yield
        | Node::YieldFrom
        | Node::Starred
        | Node::MatchValue
        | Node::Constant => Field::Value,
        Node::Delete => Field::Targets,
        Node::Assign => match placement(tree, at) {
            Placement::Last => Field::Value,
            Placement::Followed => Field::Targets,
        },
        Node::TypeAlias => choose(
            element,
            Kind::Extra(Extra::TypeParams),
            Field::TypeParams,
            first_or(tree, at, Field::Name, Field::Value),
        ),
        Node::AnnAssign => match index(tree, at) {
            SiblingIndex(0) => Field::Target,
            SiblingIndex(1) => Field::Annotation,
            _ => Field::Value,
        },
        Node::For | Node::AsyncFor => {
            statement_block(tree, at, of, first_or(tree, at, Field::Target, Field::Iter))
        }
        Node::While => statement_block(tree, at, of, Field::Test),
        Node::If => choose(
            element,
            Kind::Extra(Extra::ElifElseClause),
            Field::Orelse,
            statement_or(of, Field::Test),
        ),
        Node::With | Node::AsyncWith => choose(
            element,
            Kind::Node(Node::Withitem),
            Field::Items,
            Field::Body,
        ),
        Node::Withitem => first_or(tree, at, Field::ContextExpr, Field::OptionalVars),
        Node::Match => choose(
            element,
            Kind::Node(Node::MatchCase),
            Field::Cases,
            Field::Subject,
        ),
        Node::MatchCase => {
            if of == Category::Pattern {
                Field::Pattern
            } else {
                statement_or(of, Field::Guard)
            }
        }
        Node::Raise => match last_word(tree, at, &[FROM]) {
            Some(_) => Field::Cause,
            None => Field::Exc,
        },
        Node::Try | Node::TryStar => choose(
            element,
            Kind::Node(Node::ExceptHandler),
            Field::Handlers,
            block(tree, at),
        ),
        Node::Assert => first_or(tree, at, Field::Test, Field::Msg),
        Node::Import | Node::Global | Node::Nonlocal => Field::Names,
        Node::ImportFrom => choose(
            element,
            Kind::Node(Node::Alias),
            Field::Names,
            Field::Module,
        ),
        Node::Alias => match last_word(tree, at, &[AS]) {
            Some(_) => Field::Asname,
            None => Field::Name,
        },
        Node::BoolOp => Field::Values,
        Node::AugAssign | Node::NamedExpr => first_or(tree, at, Field::Target, Field::Value),
        Node::BinOp => first_or(tree, at, Field::Left, Field::Right),
        Node::UnaryOp => Field::Operand,
        Node::Lambda => choose(
            element,
            Kind::Node(Node::Arguments),
            Field::Args,
            Field::Body,
        ),
        Node::IfExp => match last_word(tree, at, &[IF, ELSE]) {
            Some(word) if word == IF => Field::Test,
            Some(_) => Field::Orelse,
            None => Field::Body,
        },
        Node::Dict => match preceding(tree, at) {
            Some(Kind::Token(Token::Colon | Token::Doublestar)) => Field::Values,
            Some(_) | None => Field::Keys,
        },
        Node::Set | Node::List | Node::Tuple => Field::Elts,
        Node::ListComp | Node::SetComp | Node::GeneratorExp => choose(
            element,
            Kind::Node(Node::Comprehension),
            Field::Generators,
            Field::Elt,
        ),
        Node::DictComp => choose(
            element,
            Kind::Node(Node::Comprehension),
            Field::Generators,
            first_or(tree, at, Field::Key, Field::Value),
        ),
        Node::Comprehension => match last_word(tree, at, &[IN, IF]) {
            None => Field::Target,
            Some(word) if word == IN => Field::Iter,
            Some(_) => Field::Ifs,
        },
        Node::Compare => first_or(tree, at, Field::Left, Field::Comparators),
        Node::Call => choose(
            element,
            Kind::Extra(Extra::CallArguments),
            Field::None,
            Field::Func,
        ),
        Node::FormattedValue | Node::Interpolation => {
            first_or(tree, at, Field::Value, Field::FormatSpec)
        }
        Node::JoinedStr | Node::TemplateStr => {
            if matches!(
                element,
                Kind::Extra(Extra::FStringPart | Extra::TStringPart)
            ) {
                Field::None
            } else {
                Field::Values
            }
        }
        Node::Attribute => choose(
            element,
            Kind::Extra(Extra::Identifier),
            Field::Attr,
            Field::Value,
        ),
        Node::Subscript => first_or(tree, at, Field::Value, Field::Slice),
        Node::Slice => match before(tree, at)
            .filter(|each| kind(tree, *each) == Kind::Token(Token::Colon))
            .count()
        {
            0 => Field::Lower,
            1 => Field::Upper,
            _ => Field::Step,
        },
        Node::MatchSequence | Node::MatchOr => Field::Patterns,
        Node::MatchMapping => {
            if element == Kind::Extra(Extra::Identifier) {
                Field::Rest
            } else if of == Category::Pattern {
                Field::Patterns
            } else {
                Field::Keys
            }
        }
        Node::MatchClass => choose(
            element,
            Kind::Extra(Extra::PatternArguments),
            Field::None,
            Field::Cls,
        ),
        Node::MatchStar => Field::Name,
        Node::MatchAs => choose(
            element,
            Kind::Extra(Extra::Identifier),
            Field::Name,
            Field::Pattern,
        ),
        Node::ExceptHandler => choose(
            element,
            Kind::Extra(Extra::Identifier),
            Field::Name,
            statement_or(of, Field::Type),
        ),
        Node::Arguments => parameter(tree, at),
        Node::Arg => choose(
            element,
            Kind::Extra(Extra::Identifier),
            Field::Arg,
            Field::Annotation,
        ),
        Node::Keyword => choose(
            element,
            Kind::Extra(Extra::Identifier),
            Field::Arg,
            Field::Value,
        ),
        Node::TypeVar | Node::ParamSpec | Node::TypeVarTuple => {
            if element == Kind::Extra(Extra::Identifier) {
                Field::Name
            } else if last_word(tree, at, &[COLON, EQUAL]) == Some(COLON) {
                Field::Bound
            } else {
                Field::DefaultValue
            }
        }
        // A node of the standard with no node of a field under it: a statement of one keyword, a name, a constant's single
        // literal (a token), and the operators and contexts, which the tree writes as tokens or not at all.
        Node::FunctionType
        | Node::Pass
        | Node::Break
        | Node::Continue
        | Node::Name
        | Node::MatchSingleton
        | Node::TypeIgnore
        | Node::Load
        | Node::Store
        | Node::Del
        | Node::And
        | Node::Or
        | Node::Add
        | Node::Sub
        | Node::Mult
        | Node::MatMult
        | Node::Div
        | Node::Mod
        | Node::Pow
        | Node::LShift
        | Node::RShift
        | Node::BitOr
        | Node::BitXor
        | Node::BitAnd
        | Node::FloorDiv
        | Node::Invert
        | Node::Not
        | Node::UAdd
        | Node::USub
        | Node::Eq
        | Node::NotEq
        | Node::Lt
        | Node::LtE
        | Node::Gt
        | Node::GtE
        | Node::Is
        | Node::IsNot
        | Node::In
        | Node::NotIn => Field::None,
    }
}

/// The role of the element at `at` under its parent; `None` for the root and for an edge the standard gives no field.
#[must_use]
pub(crate) fn field_of(tree: &Tree<'_>, at: Position) -> Field {
    if is_root(tree, at) == Rootness::Root {
        return Field::None;
    }
    match (kind(tree, parent(tree, at)), is_token(tree, at)) {
        (Kind::Node(holder), Leaf::Node) => node_under(tree, at, holder),
        (Kind::Node(holder), Leaf::Token) => token_under(tree, at, holder),
        (Kind::Extra(holder), _) => within(tree, at, holder),
        (Kind::Token(_), _) => Field::None,
    }
}

/// The number of the role of the element at `at`, for the table to hold.
#[must_use]
pub(crate) fn role_of(tree: &Tree<'_>, at: Position) -> languages::FieldId {
    field_bits(field_of(tree, at))
}
