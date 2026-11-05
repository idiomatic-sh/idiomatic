//! The `shapes` service: free functions named by verb over the library's tree, with no IO and no ports. It visits the nodes of the
//! tree in the order of the file and finds the nodes and tokens of `go/ast` and `go/token` that each is.

use ryo::languages::NameText;

use super::types::{Descent, Silence, Visit};
use crate::domain::declarations::service::{
    closure, declaration, fields, function, method_element, signature,
};
use crate::domain::declarations::types::Grouping;
use crate::domain::expressions::service::{
    call, elements, field_declaration, generic_type, implicit_array, instantiation, interface,
    unary, union, variadic_parameter,
};
use crate::domain::expressions::types::Conversion;
use crate::domain::grammar::service::{
    child_count, children_back, ending, parent_form, raw, written,
};
use crate::domain::grammar::types::{ByteIndex, Concrete, Form, Index, Presence};
use crate::domain::literals::service::unquoted;
use crate::domain::literals::types::Written;
use crate::domain::nodes::types::Node;
use crate::domain::pieces::service::{node, string, token};
use crate::domain::pieces::types::Pieces;
use crate::domain::statements::service::{
    branch, clauses, labeled, receive, repetition, type_switch,
};
use crate::domain::tokens::constants::LINE_COMMENT;
use crate::domain::tokens::types::Token;

/// Finds the token of a comment: the comment without the carriage return that ends a line comment on a file with Windows line ends.
fn comment(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index, from: ByteIndex, to: ByteIndex) {
    let text = written(tree, at).0;
    let end = if text.starts_with(LINE_COMMENT) && text.ends_with('\r') {
        ByteIndex(to.0.saturating_sub(1))
    } else {
        to
    };
    token(pieces, Token::Comment, from, end);
}

/// Finds the nodes and tokens of a string: a `BasicLit` over a string token, with the text the string stands for when it is text.
fn text(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index, from: ByteIndex, to: ByteIndex) {
    node(pieces, Node::BasicLit, from, to);
    match unquoted(Written(written(tree, at).0)) {
        Some(value) => string(pieces, from, to, NameText(&value.0)),
        None => token(pieces, Token::String, from, to),
    }
}

/// Finds a node over the same bytes, a `CaseClause` being a `CommClause` in a `select`, as the other clauses of it are.
fn same(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index, kind: Node) -> Descent {
    if kind == Node::StarExpr && pieces.consumed.contains(&at) {
        return Descent::Below;
    }
    let kind = if kind == Node::CaseClause && parent_form(tree, at) == Some(Form::Select) {
        Node::CommClause
    } else {
        kind
    };
    node(pieces, kind, raw(tree, at).start, ending(tree, at));
    Descent::Below
}

/// Finds the nodes of the construct the node at `at` is, one `go/ast` builds from parts the library does not group. Only the forms that
/// are such constructs find anything here: every other form was told apart by `find_at`, which calls this for the rest.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "the constructs are the forms that need the parts around them; every other form is told apart before this is asked"
)]
fn build(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index, form: Form) {
    match form {
        Form::Function | Form::Method => function(tree, pieces, at),
        Form::Closure => closure(tree, pieces, at),
        Form::Signature => signature(tree, pieces, at),
        Form::MethodElement => method_element(tree, pieces, at),
        Form::Union => union(tree, pieces, at),
        Form::GenericType => generic_type(tree, pieces, at),
        Form::Instantiation => instantiation(tree, pieces, at),
        Form::Call => call(tree, pieces, at, Conversion::Call),
        Form::Conversion => call(tree, pieces, at, Conversion::Written),
        Form::Unary => unary(tree, pieces, at),
        Form::Interface => interface(tree, pieces, at),
        Form::FieldDeclaration => field_declaration(tree, pieces, at),
        Form::Variadic => variadic_parameter(tree, pieces, at),
        Form::ImplicitArray => implicit_array(tree, pieces, at),
        Form::Elements => elements(tree, pieces, at),
        Form::Declaration | Form::Import => declaration(tree, pieces, at),
        Form::Switch => clauses(tree, pieces, Node::SwitchStmt, at),
        Form::Select => clauses(tree, pieces, Node::SelectStmt, at),
        Form::TypeSwitch => type_switch(tree, pieces, at),
        Form::Loop => repetition(tree, pieces, at),
        Form::Receive => receive(tree, pieces, at),
        Form::Labeled => labeled(tree, pieces, at),
        Form::Branch(jump) => branch(tree, pieces, at, jump),
        _ => {}
    }
}

/// Finds the nodes and tokens of the node at `at`, and says whether to go on below it. Each arm names its form, but for the constructs
/// `go/ast` builds from several nodes, which `build` tells apart.
fn find_at(tree: &Concrete<'_>, pieces: &mut Pieces, at: Index, silence: Silence) -> Descent {
    let this = raw(tree, at);
    if this.presence == Presence::Missing {
        return Descent::Stop;
    }
    let (from, to) = (this.start, ending(tree, at));
    match this.form {
        Form::Spelled(kind) => {
            if this.end > this.start {
                token(pieces, kind, this.start, this.end);
            }
            Descent::Stop
        }
        Form::Unspelled => {
            if this.end > this.start {
                token(pieces, Token::Illegal, this.start, this.end);
            }
            Descent::Stop
        }
        Form::Inside => Descent::Stop,
        Form::Comment => {
            comment(tree, pieces, at, from, to);
            Descent::Stop
        }
        Form::Name | Form::TypeName => {
            node(pieces, Node::Ident, from, to);
            token(pieces, Token::Ident, from, to);
            Descent::Stop
        }
        Form::Dot => {
            node(pieces, Node::Ident, from, to);
            token(pieces, Token::Period, from, to);
            Descent::Stop
        }
        Form::Literal(kind) => {
            node(pieces, Node::BasicLit, from, to);
            token(pieces, kind, from, to);
            Descent::Stop
        }
        Form::Text => {
            text(tree, pieces, at, from, to);
            Descent::Stop
        }
        Form::Source | Form::Transparent | Form::StatementList | Form::RangeClause => {
            Descent::Below
        }
        Form::Broken | Form::Unknown => {
            if child_count(tree, at).0 == 0 && to > from {
                token(pieces, Token::Illegal, from, to);
                Descent::Stop
            } else {
                Descent::Below
            }
        }
        Form::Node(Node::Field) | Form::Variadic if silence == Silence::Quiet => Descent::Below,
        Form::Node(kind) => same(tree, pieces, at, kind),
        Form::Parameters => {
            node(pieces, Node::FieldList, from, to);
            match fields(tree, pieces, at) {
                Grouping::Grouped => Descent::Fieldless,
                Grouping::Ungrouped => Descent::Below,
            }
        }
        built @ (Form::Function
        | Form::Method
        | Form::Closure
        | Form::Signature
        | Form::MethodElement
        | Form::Union
        | Form::GenericType
        | Form::Instantiation
        | Form::Call
        | Form::Conversion
        | Form::Unary
        | Form::Interface
        | Form::FieldDeclaration
        | Form::Variadic
        | Form::ImplicitArray
        | Form::Elements
        | Form::Declaration
        | Form::Import
        | Form::Switch
        | Form::Select
        | Form::TypeSwitch
        | Form::Loop
        | Form::Receive
        | Form::Labeled
        | Form::Branch(_)) => {
            build(tree, pieces, at, built);
            Descent::Below
        }
    }
}

/// Finds the nodes and tokens of `go/ast` and `go/token` in the tree, visiting its nodes in the order of the file.
pub(crate) fn find(tree: &Concrete<'_>, pieces: &mut Pieces) {
    let mut pending = vec![Visit {
        at: Index(0),
        silence: Silence::Speaks,
    }];
    while let Some(visit) = pending.pop() {
        let silence = match find_at(tree, pieces, visit.at, visit.silence) {
            Descent::Stop => continue,
            Descent::Below => Silence::Speaks,
            Descent::Fieldless => Silence::Quiet,
        };
        for child in children_back(tree, visit.at) {
            pending.push(Visit { at: child, silence });
        }
    }
}
