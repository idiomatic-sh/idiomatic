//! The `blocks` service: free functions named by verb that read the blocks a document makes from its rows of the shared
//! relations and compare them in order with its exact template's, with no IO and no ports. A section is a block named by
//! its title, nested by the sections around it; a run of prose and a literal of a block of its own are blocks under the
//! section before them.

use std::collections::BTreeSet;

use super::types::{Block, Placed, SectionDepth, Titled};
use crate::domain::comments::types::ByteSpan;
use crate::domain::contracts::constants::{
    ARTICLE, LITERAL_BLOCK, ONE_LINE, PROSE_BLOCK, SECTION, SPACE,
};
use crate::domain::contracts::service::{holding, written};
use crate::domain::contracts::types::{
    AttributeText, ComparedText, DataRow, Declaration, DeclarationKind, DeclarationName,
    Decorations, Facts, Holding, RowPlace, Said, TemplateIdentity,
};

/// How many sections hold the section `own`, written at `span`, its own scope left out, and one.
fn depth(
    own: &DataRow,
    span: ByteSpan,
    sections: &[Titled<'_>],
    scopes: &[&DataRow],
) -> SectionDepth {
    SectionDepth(
        scopes
            .iter()
            .filter(|scope| {
                scope.id != own.id
                    && sections.iter().any(|section| section.row.id == scope.id)
                    && matches!(scope.span, RowPlace::At(outer) if holding(outer, span) == Holding::Holds)
            })
            .count()
            + 1,
    )
}

/// A block's attribute of its writing: a run written on one line, or what a literal is written with besides its value
/// (the line that opens it).
fn writing(text: ComparedText<'_>, placed: Placed<'_>) -> BTreeSet<AttributeText> {
    let mut attributes = BTreeSet::new();
    let whole = written(text, placed.span).0;
    match placed.block {
        Block::Section(_) => {}
        Block::Run => {
            if !whole.trim().contains('\n') {
                let _: bool = attributes.insert(AttributeText(ONE_LINE.into()));
            }
        }
        Block::Literal(value) => {
            let opener = whole
                .find(&*value.0)
                .and_then(|at| whole.get(..at))
                .unwrap_or(whole)
                .lines()
                .next()
                .unwrap_or_default()
                .split_whitespace()
                .collect::<String>();
            if !opener.is_empty() {
                let _: bool = attributes.insert(AttributeText(opener.into()));
            }
        }
    }
    attributes
}

/// The blocks a document makes, in order, read from its rows and its `text`: each section by its title and its depth,
/// each run of prose that is not a title, and each literal outside every run, under the section before it; a run marked
/// `one-line` when it is written on one line, a literal marked with the line that opens it.
#[must_use]
pub fn blocks(facts: &Facts<'_>, text: ComparedText<'_>) -> Vec<Declaration> {
    let sections = facts
        .defines
        .iter()
        .filter(|row| &*row.kind.0 == SECTION)
        .filter_map(|row| match row.span {
            RowPlace::At(span) => Some(Titled { row, span }),
            RowPlace::Unplaced => None,
        })
        .collect::<Vec<_>>();
    let runs = facts
        .prose
        .iter()
        .filter(|run| !sections.iter().any(|section| section.row.id == run.id))
        .filter_map(|run| match run.span {
            RowPlace::At(span) => Some(span),
            RowPlace::Unplaced => None,
        })
        .collect::<Vec<_>>();
    let mut placed = sections
        .iter()
        .map(|section| Placed {
            span: section.span,
            block: Block::Section(section.row),
        })
        .chain(runs.iter().map(|span| Placed {
            span: *span,
            block: Block::Run,
        }))
        .collect::<Vec<_>>();
    for literal in &facts.literals {
        if let RowPlace::At(span) = literal.span
            && !runs
                .iter()
                .chain(sections.iter().map(|section| &section.span))
                .any(|run| holding(*run, span) == Holding::Holds)
        {
            placed.push(Placed {
                span,
                block: Block::Literal(&literal.text),
            });
        }
    }
    placed.sort_by_key(|each| (each.span.start, std::cmp::Reverse(each.span.end)));
    let mut found = Vec::new();
    let mut heading: Option<DeclarationName> = None;
    for each in placed {
        let kind = match each.block {
            Block::Section(section) => {
                let SectionDepth(level) = depth(section, each.span, &sections, &facts.scopes);
                heading = Some(DeclarationName(section.key.0.clone()));
                found.push(Declaration {
                    kind: DeclarationKind(format!("{SECTION}-{level}").into()),
                    name: DeclarationName(section.key.0.clone()),
                    owners: Vec::new(),
                    attributes: BTreeSet::new(),
                    decorations: Decorations::Open,
                });
                continue;
            }
            Block::Run => PROSE_BLOCK,
            Block::Literal(_) => LITERAL_BLOCK,
        };
        found.push(Declaration {
            kind: DeclarationKind(kind.into()),
            name: DeclarationName::default(),
            owners: heading.iter().cloned().collect(),
            attributes: writing(text, each),
            decorations: Decorations::Open,
        });
    }
    found
}

/// A block as a message names it: a section by its title, any other block by its attributes and kind under its section.
fn block(declaration: &Declaration) -> Said {
    if declaration.owners.is_empty() && !declaration.name.0.is_empty() {
        return Said(format!("the section `{}`", declaration.name.0));
    }
    let mut text = String::from(ARTICLE);
    for attribute in &declaration.attributes {
        text.push_str(SPACE);
        text.push_str(&attribute.0);
    }
    text.push_str(SPACE);
    text.push_str(&declaration.kind.0);
    Said(match declaration.owners.last() {
        Some(section) => format!("{text} under `{}`", section.0),
        None => format!("{text} before any section"),
    })
}

/// Where a document's blocks `found` depart from its exact template's `expected`, in order: each expected block is met
/// by the next found one exactly like it, those skipped on the way are extra, and an expected one nothing meets is
/// missing. The template is the whole document.
#[must_use]
pub fn sequence(
    expected: &[Declaration],
    found: &[Declaration],
    identity: &TemplateIdentity,
) -> Vec<Said> {
    let mut messages = Vec::new();
    let mut rest = found;
    for wanted in expected {
        let met = rest
            .iter()
            .position(|made| made == wanted)
            .and_then(|offset| rest.split_at_checked(offset));
        if let Some((extras, from_met)) = met {
            for extra in extras {
                messages.push(format!("{} is not in the template", block(extra).0));
            }
            rest = from_met.get(1..).unwrap_or_default();
        } else {
            messages.push(format!("{} is missing", block(wanted).0));
        }
    }
    for extra in rest {
        messages.push(format!("{} is not in the template", block(extra).0));
    }
    messages
        .into_iter()
        .map(|message| {
            Said(format!(
                "{message}: the `{}` template is the whole document",
                identity.0
            ))
        })
        .collect()
}
