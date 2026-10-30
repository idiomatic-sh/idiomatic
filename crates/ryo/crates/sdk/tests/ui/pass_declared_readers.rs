//! pass: every statement of a declaration, for a reader of trees and for a reader of a project's manifests.

use ryo::languages::{self, KindBits, Position, Symbol, Tree};

/// The number the language's trees give a kind.
const fn kind_bits(kind: u16) -> KindBits {
    KindBits(kind)
}

fn kind(tree: &Tree<'_>, at: Position) -> u16 {
    languages::kind_at(tree.elements, at).0
}

fn named(tree: &Tree<'_>, at: Position) -> Option<Symbol> {
    languages::first_token_text(tree.elements, at)
}

fn children_of<'a>(tree: &'a Tree<'a>, at: Position) -> impl Iterator<Item = Position> + 'a {
    languages::children(tree.elements, at)
}

struct FunctionItem;

ryo::reads! {
    FunctionItem => languages::Function |tree, at, _symbols| {
        kinds: [1, 2] as kind_bits,
        require kind(tree, at) != 0;
        given name = named(tree, at)?;
        row { id: node(at), name: value(name), file: file, span: span(at) }
    }
}

struct TreeItem;

ryo::reads! {
    TreeItem => languages::SyntaxTree |tree, at, _symbols| {
        kinds: every,
        row { id: node(at), kind: text(at), text: text(at), file: file, span: span(at) }
    }
}

struct ParameterLikeItem;

ryo::reads! {
    ParameterLikeItem => languages::Parameter |tree, at, symbols| {
        kinds: [3] as kind_bits,
        each (children_of(tree, at)) as (ordinal, child) {
            when kind(tree, child) {
                4 => {
                    row {
                        id: node(child),
                        kind: value(languages::ParameterKind::Receiver),
                        refinement: value(languages::intern_derived(symbols, tree.source, languages::NameText("receiver"))),
                        name: value(languages::intern_derived(symbols, tree.source, languages::NameText("self"))),
                        ordinal: value(languages::Integer(i64::from(ordinal.0))),
                        owner: node(at),
                        file: file,
                        span: span(child),
                    }
                }
                _ => {
                    partial "a parameter this reader cannot represent" => languages::Missing::Any;
                }
            }
        }
        each (children_of(tree, at)) as other {
            given text = languages::first_token_text(tree.elements, other)?;
            row {
                id: node(other),
                kind: value(languages::ParameterKind::Positional),
                refinement: value(text),
                name: value(text),
                ordinal: value(languages::Integer(0)),
                owner: node(at),
                file: file,
                span: span(other),
            }
        }
    }
}

/// What a project's manifests say, as the language reads them.
struct Model {
    members: Vec<String>,
    refused: Vec<String>,
}

struct WorkspaceMemberItem(());

ryo::reads! {
    WorkspaceMemberItem => languages::WorkspaceMember, manifests(()) |input, symbols|
    model model: Model = (Some(Model { members: vec![input.request.manifests.len().to_string()], refused: Vec::new() })) {
        each (&model.refused) as refused {
            partial refused => languages::Missing::Any;
        }
        each (&model.members) as member {
            row {
                workspace: value(languages::FileId(u32::try_from(member.len()).unwrap_or(0))),
                package: value(languages::FileId(u32::try_from(member.len()).unwrap_or(0))),
            }
        }
    }
}

fn main() {
    let _ = <FunctionItem as languages::ReadItem>::READER;
    let _ = <TreeItem as languages::ReadItem>::READER;
    let _ = <ParameterLikeItem as languages::ReadItem>::READER;
    let _ = <WorkspaceMemberItem as languages::ManifestItem>::READER;
}
