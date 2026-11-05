//! The `resolutions` service: free functions named by verb that say which declaration binds a type name where it is written, with no IO and
//! no ports.

use crate::domain::declarations::service::imported;
use crate::domain::nodes::types::RustNode;
use crate::domain::references::service::segments;
use crate::domain::syntax::service::{ancestors, child, kind, last_descendant, name_text};
use crate::domain::syntax::types::Kind;
use languages::Tree;
use ryo::languages;

/// The one name the path type at `at` writes: a type of a path of a single name, with no qualifier before it.
#[must_use]
pub(crate) fn plain_type_name(
    tree: &Tree<'_>,
    at: languages::Position,
) -> Option<languages::Symbol> {
    let path = child(tree, at, Kind::Node(RustNode::Path))?;
    let (mut counted, mut only) = (0_i32, languages::NO_SYMBOL);
    segments(tree, path, &mut |segment| {
        counted += 1_i32;
        only = segment;
    });
    (counted == 1).then_some(only)
}

/// What the scope `scope` binds `name` to, when it does: a type the scope declares, a name a `use` of it brings in, or a type
/// parameter of the item it is.
fn binds(
    tree: &Tree<'_>,
    scope: languages::Position,
    name: languages::Symbol,
) -> Option<languages::Position> {
    let named = |item: languages::Position| {
        child(tree, item, Kind::Node(RustNode::Name)).and_then(|each| name_text(tree, each))
            == Some(name)
    };
    match kind(tree, scope) {
        Kind::Node(RustNode::SourceFile | RustNode::ItemList | RustNode::StmtList) => {
            languages::children(tree.elements, scope).find_map(|item| match kind(tree, item) {
                Kind::Node(
                    RustNode::Struct | RustNode::Enum | RustNode::Union | RustNode::TypeAlias | RustNode::Trait,
                ) => named(item).then_some(item),
                Kind::Node(RustNode::Use) => (item.0 + 1..=last_descendant(tree, item).0)
                    .map(languages::Position)
                    .find(|each| {
                        kind(tree, *each) == Kind::Node(RustNode::UseTree)
                            && imported(tree, *each).is_some_and(|bound| bound.name == name)
                    }),
                Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => None,
            })
        }
        Kind::Node(
            RustNode::Fn
            | RustNode::Struct
            | RustNode::Enum
            | RustNode::Union
            | RustNode::Trait
            | RustNode::TypeAlias
            | RustNode::Impl,
        ) => {
            let list = child(tree, scope, Kind::Node(RustNode::GenericParamList))?;
            languages::children(tree.elements, list)
                .find(|each| kind(tree, *each) == Kind::Node(RustNode::TypeParam) && named(*each))
        }
        Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => None,
    }
}

/// The declaration, import or type parameter that binds `name` where the element at `at` is written: the innermost scope that binds
/// it wins.
#[must_use]
pub(crate) fn binding_of(
    tree: &Tree<'_>,
    at: languages::Position,
    name: languages::Symbol,
) -> Option<languages::Position> {
    ancestors(tree, at).find_map(|scope| binds(tree, scope, name))
}

/// What holds the element at `at`: the nearest item it is written in, else the file.
#[must_use]
pub(crate) fn holder(tree: &Tree<'_>, at: languages::Position) -> languages::NodeId {
    languages::node_id(
        tree.file,
        ancestors(tree, at)
            .find(|each| {
                matches!(
                    kind(tree, *each),
                    Kind::Node(
                        RustNode::Struct
                            | RustNode::Enum
                            | RustNode::Union
                            | RustNode::TypeAlias
                            | RustNode::Fn
                            | RustNode::Trait
                            | RustNode::Impl
                            | RustNode::Const
                            | RustNode::Static
                    )
                )
            })
            .unwrap_or(languages::Position(0)),
    )
}
