//! Test support split from adapter_test: Said, accessor_roles, declared_roles and more.

use super::adapter::SyntaxRaApSyntaxAdapter;
use super::schemas::kind_of;
use crate::domain::syntax::constants::EXPANDED_MACROS;
use crate::domain::syntax::types::Kind;
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::outbound::syntax::types::Expanding;
use ra_ap_syntax::SyntaxKind;
use ryo::languages::{
    ByteSlice, Elements, Position, Symbols, end_at, kind_at, positions, start_at, text_of,
};

/// A child an accessor names: the construct it is a child of, its place, and the role the accessor is called.
pub(in super::super) type Said = (
    ra_ap_syntax::SyntaxNode,
    ra_ap_syntax::TextRange,
    &'static str,
);

/// The children the accessors of the library name in `source`, each with the construct it is a child of and the role the accessor is
/// called.
pub(in super::super) fn accessor_roles(source: &'static str) -> Vec<Said> {
    use ra_ap_syntax::{AstNode as _, Edition, SourceFile};
    let mut said: Vec<Said> = Vec::new();
    let file = SourceFile::parse(source, Edition::CURRENT).tree();
    for node in file.syntax().descendants() {
        declared_roles(&node, &mut said);
        expression_roles(&node, &mut said);
    }
    said
}

/// The children the accessors of a function, an impl or a `let` name.
pub(in super::super) fn declared_roles(owner: &ra_ap_syntax::SyntaxNode, said: &mut Vec<Said>) {
    use ra_ap_syntax::AstNode as _;
    use ra_ap_syntax::ast::{self, HasGenericParams as _, HasName as _, HasVisibility as _};
    let node = owner;
    let mut say = |child: Option<ra_ap_syntax::SyntaxNode>, role: &'static str| {
        if let Some(child) = child {
            said.push((owner.clone(), child.text_range(), role));
        }
    };
    if let Some(each) = ast::Fn::cast(node.clone()) {
        say(each.name().map(|x| x.syntax().clone()), "name");
        say(each.param_list().map(|x| x.syntax().clone()), "param-list");
        say(each.ret_type().map(|x| x.syntax().clone()), "ret-type");
        say(each.body().map(|x| x.syntax().clone()), "body");
        say(
            each.generic_param_list().map(|x| x.syntax().clone()),
            "generic-param-list",
        );
        say(
            each.where_clause().map(|x| x.syntax().clone()),
            "where-clause",
        );
        say(each.visibility().map(|x| x.syntax().clone()), "visibility");
        return;
    }
    if let Some(each) = ast::Impl::cast(node.clone()) {
        say(each.trait_().map(|x| x.syntax().clone()), "trait");
        say(each.self_ty().map(|x| x.syntax().clone()), "self-ty");
        return;
    }
    if let Some(each) = ast::LetStmt::cast(node.clone()) {
        say(
            each.initializer().map(|x| x.syntax().clone()),
            "initializer",
        );
        say(each.pat().map(|x| x.syntax().clone()), "pat");
    }
}

/// The children the accessors of an expression, a loop, a match arm or a range name.
pub(in super::super) fn expression_roles(owner: &ra_ap_syntax::SyntaxNode, said: &mut Vec<Said>) {
    use ra_ap_syntax::AstNode as _;
    use ra_ap_syntax::ast::{self, HasLoopBody as _, RangeItem as _};
    let node = owner;
    let mut say = |child: Option<ra_ap_syntax::SyntaxNode>, role: &'static str| {
        if let Some(child) = child {
            said.push((owner.clone(), child.text_range(), role));
        }
    };
    if let Some(each) = ast::BinExpr::cast(node.clone()) {
        say(each.lhs().map(|x| x.syntax().clone()), "lhs");
        say(each.rhs().map(|x| x.syntax().clone()), "rhs");
        return;
    }
    if let Some(each) = ast::IfExpr::cast(node.clone()) {
        say(each.condition().map(|x| x.syntax().clone()), "condition");
        say(
            each.then_branch().map(|x| x.syntax().clone()),
            "then-branch",
        );
        say(
            each.else_branch().map(|x| x.syntax().clone()),
            "else-branch",
        );
        return;
    }
    if let Some(each) = ast::IndexExpr::cast(node.clone()) {
        say(each.base().map(|x| x.syntax().clone()), "base");
        say(each.index().map(|x| x.syntax().clone()), "index");
        return;
    }
    if let Some(each) = ast::MethodCallExpr::cast(node.clone()) {
        say(each.receiver().map(|x| x.syntax().clone()), "receiver");
        return;
    }
    if let Some(each) = ast::ForExpr::cast(node.clone()) {
        say(each.iterable().map(|x| x.syntax().clone()), "iterable");
        say(each.loop_body().map(|x| x.syntax().clone()), "loop-body");
        say(each.pat().map(|x| x.syntax().clone()), "pat");
        return;
    }
    if let Some(each) = ast::WhileExpr::cast(node.clone()) {
        say(each.condition().map(|x| x.syntax().clone()), "condition");
        say(each.loop_body().map(|x| x.syntax().clone()), "loop-body");
        return;
    }
    if let Some(each) = ast::MatchArm::cast(node.clone()) {
        say(each.guard().map(|x| x.syntax().clone()), "guard");
        say(each.pat().map(|x| x.syntax().clone()), "pat");
        return;
    }
    if let Some(each) = ast::RangeExpr::cast(node.clone()) {
        say(each.start().map(|x| x.syntax().clone()), "start");
        say(each.end().map(|x| x.syntax().clone()), "end");
    }
}

/// The roles the table gives the children of the constructs of `source`, against the accessors of the library: every child an accessor
/// names is in the role the accessor is called, and the answer lists the ones that are not.
pub(in super::super) fn roles(source: &'static str) -> String {
    let language = crate::inbound::contracts::parse::contract::ParseContract::new(
        std::sync::Arc::new(SyntaxRaApSyntaxAdapter::new(Expanding(&EXPANDED_MACROS))),
        std::sync::Arc::new(crate::outbound::notes::factory::postcard()),
    );
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    ryo::languages::contract::build(
        &language,
        ryo::languages::ParseRequest {
            path: ryo::languages::SourcePath("src/lib.rs"),
            file: ryo::languages::FileId(0),
            bytes: ByteSlice(source.as_bytes()),
            notes: ryo::languages::NoteRequest::Unwanted,
            fields: ryo::languages::FieldRequest::Wanted,
        },
        &mut elements,
        &mut symbols,
    )
    .expect("the file parses");
    let said = accessor_roles(source);
    let tree = ryo::languages::Tree {
        elements: &elements,
        source: ByteSlice(source.as_bytes()),
        file: ryo::languages::FileId(0),
    };
    let range_of = |at: Position| {
        (
            usize::try_from(start_at(&elements, at).0).unwrap_or(0),
            usize::try_from(end_at(&elements, at).0).unwrap_or(0),
        )
    };
    let mut disagreements = Vec::new();
    for (owner, range, role) in said {
        let owner_range = owner.text_range();
        let wanted = (
            usize::from(owner_range.start()),
            usize::from(owner_range.end()),
        );
        let child_range = (usize::from(range.start()), usize::from(range.end()));
        let found = positions(&elements).find(|at| {
            range_of(*at) == wanted
                && crate::domain::syntax::service::kind_of(kind_at(&elements, *at))
                    == kind_of_library(owner.kind())
        });
        let role_found = found.and_then(|owner_at| {
            ryo::languages::children(&elements, owner_at)
                .find(|each| range_of(*each) == child_range)
        });
        let name = role_found.and_then(|each| {
            text_of(
                &symbols,
                tree.source,
                ryo::languages::field_name_at(&elements, each),
            )
            .map(|text| text.0)
        });
        if name != Some(role) {
            disagreements.push(format!("{role}: {name:?}"));
        }
    }
    if disagreements.is_empty() {
        "agreed".to_owned()
    } else {
        disagreements.join("; ")
    }
}

/// The kind the language has for a kind of the library.
pub(in super::super) fn kind_of_library(library: SyntaxKind) -> Kind {
    kind_of(library)
}
