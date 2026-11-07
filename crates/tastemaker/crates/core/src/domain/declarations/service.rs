//! The `declarations` service: free functions named by verb that read the declarations a file of code makes from its rows
//! of the shared relations, each named within the declarations whose scopes hold it, with no IO and no ports. What is
//! declared in a body (a function's or a method's, or a scope no declaration names) is the body's, and is not read; nor
//! are the parts of a declaration or what a file imports.

use std::collections::{BTreeMap, BTreeSet};

use super::types::{Held, Typing};
use crate::domain::contracts::constants::{
    ARGUMENT_SEPARATOR, BODIES, BRACKETS, HELD, PARTS, TYPED,
};
use crate::domain::contracts::service::holding;
use crate::domain::contracts::types::{
    AttributeText, DataRow, Declaration, DeclarationKind, DeclarationName, Decorations, Facts,
    Holding, PartText, RowNode, RowPlace,
};

/// The rows of `rows` by the node that holds them.
fn by_owner<'r>(rows: &[&'r DataRow]) -> BTreeMap<&'r RowNode, Vec<&'r DataRow>> {
    let mut found: BTreeMap<&RowNode, Vec<&DataRow>> = BTreeMap::new();
    for row in rows {
        found.entry(&row.owner).or_default().push(row);
    }
    found
}

/// A text without its whitespace.
fn squeezed(text: PartText<'_>) -> AttributeText {
    AttributeText(text.0.split_whitespace().collect::<String>().into())
}

/// The arguments of a decoration as written, each without whitespace: split where a separator stands outside every
/// bracket and quote, so a decoration that passes several is held one argument at a time.
#[must_use]
pub fn arguments(text: PartText<'_>) -> Vec<AttributeText> {
    let text = text.0;
    let mut found = Vec::new();
    let mut current = String::new();
    let mut open = Vec::new();
    let mut quoted = false;
    let mut escaped = false;
    for character in text.chars() {
        if quoted {
            current.push(character);
            if !escaped && character == '"' {
                quoted = false;
            }
            escaped = !escaped && character == '\\';
            continue;
        }
        let opening = BRACKETS.iter().find(|(opening, _)| *opening == character);
        match (character, opening) {
            ('"', _) => {
                quoted = true;
                current.push(character);
            }
            (_, Some((_, close))) => {
                open.push(*close);
                current.push(character);
            }
            _ if open.last() == Some(&character) => {
                let _: Option<char> = open.pop();
                current.push(character);
            }
            _ if character == ARGUMENT_SEPARATOR && open.is_empty() => {
                found.push(std::mem::take(&mut current));
            }
            _ if character.is_whitespace() => {}
            _ => current.push(character),
        }
    }
    found.push(current);
    found
        .into_iter()
        .filter(|each| !each.is_empty())
        .map(|each| AttributeText(each.into()))
        .collect()
}

/// The attributes of the declaration `id` and the paths of its decorations: each decoration one argument at a time
/// (`path(argument)`, the path alone when it passes none), its visibility, and its type when it is a value.
fn attributes(
    id: &RowNode,
    typing: Typing,
    decorations: &BTreeMap<&RowNode, Vec<&DataRow>>,
    visibilities: &BTreeMap<&RowNode, Vec<&DataRow>>,
    annotations: &BTreeMap<&RowNode, Vec<&DataRow>>,
) -> Held {
    let mut held = Held::default();
    for decoration in decorations.get(id).into_iter().flatten() {
        let path = squeezed(PartText(&decoration.key.0));
        let passed = arguments(PartText(&decoration.text.0));
        if passed.is_empty() {
            let _: bool = held.attributes.insert(path.clone());
        }
        for argument in passed {
            let _: bool = held
                .attributes
                .insert(AttributeText(format!("{}({})", path.0, argument.0).into()));
        }
        let _: bool = held.paths.insert(DeclarationName(path.0));
    }
    for visibility in visibilities.get(id).into_iter().flatten() {
        let _: bool = held
            .attributes
            .insert(squeezed(PartText(&visibility.text.0)));
    }
    if typing == Typing::Typed {
        for annotation in annotations.get(id).into_iter().flatten() {
            let _: bool = held.attributes.insert(AttributeText(
                format!(": {}", squeezed(PartText(&annotation.text.0)).0).into(),
            ));
        }
    }
    held
}

/// The names of the declarations whose scopes hold `span`, outermost first, beside `own`'s own scope; none when a scope
/// that holds it is a body or is named by no declaration.
fn owners(
    own: &RowNode,
    span: RowPlace,
    scopes: &[&DataRow],
    defined: &BTreeMap<&RowNode, &DataRow>,
) -> Option<Vec<DeclarationName>> {
    let RowPlace::At(span) = span else {
        return Some(Vec::new());
    };
    let by_child = scopes
        .iter()
        .map(|scope| (&scope.id, *scope))
        .collect::<BTreeMap<_, _>>();
    let innermost = scopes
        .iter()
        .filter(|scope| scope.id != *own && scope.id != scope.owner)
        .filter_map(|scope| match scope.span {
            RowPlace::At(outer) if holding(outer, span) == Holding::Holds => Some((scope, outer)),
            RowPlace::At(_) | RowPlace::Unplaced => None,
        })
        .min_by_key(|(_, outer)| outer.end.0 - outer.start.0)
        .map(|(scope, _)| *scope);
    let mut names = Vec::new();
    let mut current = innermost;
    let mut seen = BTreeSet::new();
    while let Some(scope) = current {
        if scope.id == scope.owner || !seen.insert(&scope.id) {
            break;
        }
        let declared = defined.get(&scope.id)?;
        if BODIES.contains(&&*declared.kind.0) {
            return None;
        }
        names.push(DeclarationName(declared.key.0.clone()));
        current = by_child.get(&scope.owner).copied();
    }
    names.reverse();
    Some(names)
}

/// The declarations a file of code makes, read from its rows: each declaration not in a body, by its refinement (its
/// shared kind when it has none) and its name, within the declarations whose scopes hold it, with its decorations, its
/// visibility and, for a value, its type; and the file itself, with the decorations of its own scope.
#[must_use]
pub fn declarations(facts: &Facts<'_>) -> Vec<Declaration> {
    let defined = facts
        .defines
        .iter()
        .map(|row| (&row.id, *row))
        .collect::<BTreeMap<_, _>>();
    let decorations = by_owner(&facts.decorates);
    let visibilities = by_owner(&facts.visibility);
    let annotations = by_owner(&facts.annotates);
    let mut found = Vec::new();
    for row in facts
        .defines
        .iter()
        .filter(|row| !PARTS.contains(&&*row.kind.0))
    {
        let Some(owners) = owners(&row.id, row.span, &facts.scopes, &defined) else {
            continue;
        };
        let typing = if TYPED.contains(&&*row.kind.0) {
            Typing::Typed
        } else {
            Typing::Untyped
        };
        let Held {
            attributes: held,
            paths,
        } = attributes(&row.id, typing, &decorations, &visibilities, &annotations);
        found.push(Declaration {
            kind: DeclarationKind(if row.text.0.is_empty() {
                row.kind.0.clone()
            } else {
                row.text.0.clone()
            }),
            name: DeclarationName(row.key.0.clone()),
            owners,
            attributes: held,
            // A module its content decorates is held to those decorations: one the file adds beside them can disable it
            // (a condition no build meets), so it is a mismatch. Any other declaration may carry more than its content.
            decorations: if paths.is_empty() || !HELD.contains(&&*row.kind.0) {
                Decorations::Open
            } else {
                Decorations::Named(paths)
            },
        });
    }
    for root in facts.scopes.iter().filter(|scope| scope.id == scope.owner) {
        let held = attributes(
            &root.id,
            Typing::Untyped,
            &decorations,
            &visibilities,
            &annotations,
        )
        .attributes;
        found.push(Declaration {
            kind: DeclarationKind::default(),
            name: DeclarationName::default(),
            owners: Vec::new(),
            attributes: held,
            decorations: Decorations::Open,
        });
    }
    found.sort();
    found.dedup();
    found
}
