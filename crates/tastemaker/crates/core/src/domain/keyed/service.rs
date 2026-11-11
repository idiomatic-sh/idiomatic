//! The `keyed` service: free functions named by verb that read the declarations a keyed document (a data document's keys,
//! tables and sequences, as the shared `entry`, `mapping` and `sequence-element` relations give them) makes, and compare a
//! placed document's with its template's, each table of a sequence judged whole, with no IO and no ports.

use std::collections::{BTreeMap, BTreeSet};

use super::constants::{ASIDE, ELEMENT_CLOSE, ELEMENT_OPEN};
use super::types::{Depth, Renaming, TableMember};
use crate::domain::contracts::constants::{KEY, KEY_PATH_SEPARATOR, SCALARS, TABLES};
use crate::domain::contracts::service::compare;
use crate::domain::contracts::types::{
    AttributeText, DataRow, Declaration, DeclarationKind, DeclarationName, Decorations, Facts,
    Holding, Mismatch, RowNode,
};
use crate::domain::definitions::types::Exactness;

/// The rows of `rows`, by their node.
fn by_node<'r>(rows: &[&'r DataRow]) -> BTreeMap<&'r RowNode, &'r DataRow> {
    rows.iter().map(|row| (&row.id, *row)).collect()
}

/// The path of the data node `start` from its document: each key, and each element by its place.
fn data_path(
    start: &RowNode,
    entries: &BTreeMap<&RowNode, &DataRow>,
    elements: &BTreeMap<&RowNode, &DataRow>,
    mappings: &BTreeMap<&RowNode, &DataRow>,
) -> DeclarationName {
    let mut segments = Vec::new();
    let mut node = start;
    let mut seen = BTreeSet::new();
    while seen.insert(node) {
        if let Some(entry) = entries.get(node) {
            segments.push(entry.key.0.to_string());
            node = &entry.owner;
        } else if let Some(element) = elements.get(node) {
            segments.push(format!("{ELEMENT_OPEN}{}{ELEMENT_CLOSE}", element.key.0));
            node = &element.owner;
        } else if let Some(mapping) = mappings.get(node) {
            node = &mapping.owner;
        } else {
            break;
        }
    }
    segments.reverse();
    DeclarationName(segments.join(KEY_PATH_SEPARATOR).into())
}

/// The declarations a data document makes, read from its rows: every key and every element of a sequence, by its path
/// from the document (an element by its place); a scalar also with its written value.
#[must_use]
pub fn data_declarations(facts: &Facts<'_>) -> Vec<Declaration> {
    let entries = by_node(&facts.entries);
    let elements = by_node(&facts.elements);
    let mappings = by_node(&facts.mappings);
    let mut found = facts
        .entries
        .iter()
        .chain(&facts.elements)
        .map(|row| {
            let mut attributes = BTreeSet::new();
            if SCALARS.contains(&&*row.kind.0) {
                let _: bool = attributes.insert(AttributeText(format!("= {}", row.text.0).into()));
            }
            Declaration {
                kind: DeclarationKind(KEY.into()),
                name: data_path(&row.id, &entries, &elements, &mappings),
                owners: Vec::new(),
                attributes,
                decorations: Decorations::Open,
            }
        })
        .collect::<Vec<_>>();
    found.sort();
    found.dedup();
    found
}

/// How deep `name` is in its document.
fn depth(name: &DeclarationName) -> Depth {
    Depth(name.0.matches(KEY_PATH_SEPARATOR).count())
}

/// The paths of the elements of a data document that are tables, outermost first: each is judged as a whole.
#[must_use]
pub fn tables(facts: &Facts<'_>) -> Vec<DeclarationName> {
    let entries = by_node(&facts.entries);
    let elements = by_node(&facts.elements);
    let mappings = by_node(&facts.mappings);
    let mut found = facts
        .elements
        .iter()
        .filter(|row| TABLES.contains(&&*row.kind.0))
        .map(|row| data_path(&row.id, &entries, &elements, &mappings))
        .collect::<Vec<_>>();
    found.sort_by_key(|path| (depth(path), path.clone()));
    found.dedup();
    found
}

/// The declarations of `declarations` within the element `table`, each named from it: `id` for `tools.[0].id`.
fn within<'d>(declarations: &'d [Declaration], table: &DeclarationName) -> Vec<TableMember<'d>> {
    let prefix = format!("{}{KEY_PATH_SEPARATOR}", table.0);
    declarations
        .iter()
        .filter_map(|declaration| {
            declaration
                .name
                .0
                .strip_prefix(&*prefix)
                .map(|rest| TableMember {
                    declaration,
                    rest: DeclarationName(rest.into()),
                })
        })
        .collect()
}

/// The sequence an element's path is in: `tools` for `tools.[0]`.
fn sequence_of(table: &DeclarationName) -> DeclarationName {
    table
        .0
        .rsplit_once(KEY_PATH_SEPARATOR)
        .map_or_else(DeclarationName::default, |(sequence, _)| {
            DeclarationName(sequence.into())
        })
}

/// The declarations of the template's element `wanted` that none of `made`, each named from its element, is.
fn lacking(
    expected: &[Declaration],
    wanted: &DeclarationName,
    made: &[TableMember<'_>],
) -> Vec<DeclarationName> {
    within(expected, wanted)
        .into_iter()
        .filter(|wanted| {
            !made.iter().any(|candidate| {
                wanted.rest == candidate.rest
                    && wanted.declaration.kind == candidate.declaration.kind
                    && wanted
                        .declaration
                        .attributes
                        .is_subset(&candidate.declaration.attributes)
            })
        })
        .map(|wanted| wanted.declaration.name.clone())
        .collect()
}

/// Whether `name` is the table `table` or lies within it.
fn inside(name: &DeclarationName, table: &DeclarationName) -> Holding {
    let within = name.0 == table.0
        || name
            .0
            .strip_prefix(&*table.0)
            .is_some_and(|rest| rest.starts_with(KEY_PATH_SEPARATOR));
    if within {
        Holding::Holds
    } else {
        Holding::Apart
    }
}

/// `name` with each of `moves` applied at once: the longest `from` that is `name` or holds it gives way to its `to`.
fn moved(name: &DeclarationName, moves: &[Renaming]) -> Option<DeclarationName> {
    moves
        .iter()
        .filter(|each| inside(name, &each.from) == Holding::Holds)
        .max_by_key(|each| each.from.0.len())
        .map(|each| {
            DeclarationName(
                format!(
                    "{}{}",
                    each.to.0,
                    name.0.get(each.from.0.len()..).unwrap_or_default()
                )
                .into(),
            )
        })
}

/// The name a table of the file's no template's table meets takes, aside.
fn aside(table: &DeclarationName) -> DeclarationName {
    DeclarationName(format!("{}{ASIDE}", table.0).into())
}

/// `declarations` without those within `table`, the table itself kept: it stands for all of them.
fn whole(declarations: &mut Vec<Declaration>, table: &DeclarationName) {
    declarations.retain(|declaration| {
        declaration.name == *table || inside(&declaration.name, table) == Holding::Apart
    });
}

/// How a data document's declarations `found` differ from its template's `expected`, each table of a sequence judged
/// whole, depth by depth: a template's table is met by the first of the file's tables in the same sequence that holds
/// every declaration of it, wherever it is placed; one no table of the file holds is the table left standing that makes
/// the most of it, and is missing what that one lacks, or, when none is left standing, what none of the file's tables
/// makes; a file's table no template's meets is, for an exact template, one extra element. A sequence of values holds
/// the template's values in their order. `instance_values`, when given, names a top-level table of the document (a
/// manifest's `variables`) whose entries' values are the instance's: each entry is held by name, the template's values
/// within it are held, and the file's other values within it are not extra.
#[must_use]
pub(crate) fn compare_data(
    expected: &Facts<'_>,
    found: &Facts<'_>,
    exact: Exactness,
    instance_values: Option<&str>,
) -> Vec<Mismatch> {
    let mut wanted = data_declarations(expected);
    let mut made = data_declarations(found);
    let wanted_tables = tables(expected);
    let mut made_tables = tables(found);
    let deepest = wanted_tables
        .iter()
        .chain(&made_tables)
        .map(depth)
        .max()
        .unwrap_or_default();
    let mut gone: Vec<DeclarationName> = Vec::new();
    for level in 0..=deepest.0 {
        let level = Depth(level);
        let peers = |table: &DeclarationName, tables: &[DeclarationName]| {
            tables
                .iter()
                .filter(|candidate| {
                    depth(candidate) == level && sequence_of(candidate) == sequence_of(table)
                })
                .cloned()
                .collect::<Vec<_>>()
        };
        let mut moves = Vec::new();
        let mut taken: Vec<DeclarationName> = Vec::new();
        let mut unmet = Vec::new();
        for table in wanted_tables.iter().filter(|table| depth(table) == level) {
            if gone
                .iter()
                .any(|missing| inside(table, missing) == Holding::Holds)
            {
                continue;
            }
            let partner = peers(table, &made_tables).into_iter().find(|candidate| {
                !taken.contains(candidate)
                    && lacking(&wanted, table, &within(&made, candidate)).is_empty()
            });
            match partner {
                Some(candidate) => {
                    taken.push(candidate.clone());
                    moves.push(Renaming {
                        from: candidate,
                        to: table.clone(),
                    });
                }
                None => unmet.push(table.clone()),
            }
        }
        for table in &unmet {
            let candidates = peers(table, &made_tables);
            // The table of the file left standing in its sequence that makes the most of it is the one it is: what that
            // one lacks is missing.
            let standing = candidates
                .iter()
                .filter(|candidate| !taken.contains(candidate))
                .min_by_key(|candidate| lacking(&wanted, table, &within(&made, candidate)).len())
                .cloned();
            if let Some(candidate) = standing {
                taken.push(candidate.clone());
                moves.push(Renaming {
                    from: candidate,
                    to: table.clone(),
                });
                continue;
            }
            // None is left standing: the table is absent, and what none of the file's tables makes of it is missing.
            let rest = candidates
                .iter()
                .flat_map(|candidate| within(&made, candidate))
                .collect::<Vec<_>>();
            let lacking = lacking(&wanted, table, &rest);
            if lacking.is_empty() {
                whole(&mut wanted, table);
            } else {
                wanted.retain(|declaration| {
                    inside(&declaration.name, table) == Holding::Apart
                        || lacking.contains(&declaration.name)
                });
            }
            gone.push(table.clone());
        }
        for table in made_tables.iter().filter(|table| depth(table) == level) {
            if !taken.contains(table) {
                // A table of the file no template's table meets stands aside, as one element.
                whole(&mut made, table);
                moves.push(Renaming {
                    from: table.clone(),
                    to: aside(table),
                });
            }
        }
        for declaration in &mut made {
            if let Some(name) = moved(&declaration.name, &moves) {
                declaration.name = name;
            }
        }
        for table in &mut made_tables {
            if let Some(name) = moved(table, &moves) {
                *table = name;
            }
        }
    }
    in_order(&wanted, &mut made);
    let mut mismatches = compare(&wanted, &made, exact);
    if let Some(table) = instance_values {
        // A value within an entry of the instance's table is the instance's: only the entry's name is held exactly.
        let prefix = format!("{table}{KEY_PATH_SEPARATOR}");
        mismatches.retain(|mismatch| match mismatch {
            Mismatch::Extra(declaration) => !declaration
                .name
                .0
                .strip_prefix(&*prefix)
                .is_some_and(|within| within.contains(KEY_PATH_SEPARATOR)),
            Mismatch::Missing(_) => true,
        });
    }
    mismatches
}

/// `made` with the values of each sequence of the template's `wanted` matched in order: each value of the template is
/// met by the next value of the file's sequence written the same, which takes its place, so a file may write values
/// between them; one of the file's no template's value meets stands aside.
fn in_order(wanted: &[Declaration], made: &mut [Declaration]) {
    // A value of a sequence: an element with a written value.
    let valued = |declaration: &&Declaration| {
        declaration.name.0.ends_with(ELEMENT_CLOSE) && !declaration.attributes.is_empty()
    };
    let place = |name: &DeclarationName| {
        let index = name
            .0
            .rsplit_once(ELEMENT_OPEN)
            .map(|(_, rest)| rest.trim_end_matches(ELEMENT_CLOSE).parse::<usize>());
        // A name with no element index sorts after every indexed one.
        let Some(Ok(depth)) = index else {
            return Depth(usize::MAX);
        };
        Depth(depth)
    };
    let mut sequences: BTreeMap<DeclarationName, Vec<&Declaration>> = BTreeMap::new();
    for declaration in wanted.iter().filter(valued) {
        sequences
            .entry(sequence_of(&declaration.name))
            .or_default()
            .push(declaration);
    }
    for (sequence, values) in &mut sequences {
        values.sort_by_key(|declaration| place(&declaration.name));
        let mut written = made
            .iter()
            .enumerate()
            .filter(|(_, declaration)| {
                valued(declaration) && sequence_of(&declaration.name) == *sequence
            })
            .map(|(at, declaration)| (place(&declaration.name), at))
            .collect::<Vec<_>>();
        written.sort_unstable();
        let mut names = BTreeMap::new();
        let mut rest: &[(Depth, usize)] = &written;
        for value in values.iter() {
            let found = rest.iter().position(|(_, at)| {
                made.get(*at)
                    .is_some_and(|candidate| value.attributes.is_subset(&candidate.attributes))
            });
            if let Some((offset, (_, at))) =
                found.and_then(|offset| rest.get(offset).map(|entry| (offset, entry)))
            {
                drop(names.insert(*at, value.name.clone()));
                rest = rest.get(offset + 1..).unwrap_or_default();
            }
        }
        for (_, at) in written {
            if let Some(declaration) = made.get_mut(at) {
                declaration.name = names
                    .remove(&at)
                    .unwrap_or_else(|| aside(&declaration.name));
            }
        }
    }
}
