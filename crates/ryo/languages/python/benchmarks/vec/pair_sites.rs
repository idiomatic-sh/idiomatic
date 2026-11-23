//! Row projections of the provider's assignment sources and namespace bounds.
use super::{CellKind, ColumnFile, fact_catalog};
use CellKind::{File, Node, Number, Symbol};

pub(crate) const ASSIGNMENT: &[(&str, CellKind)] = &[
    ("target", Node),
    ("value", Node),
    ("operation", Symbol),
    ("association", Symbol),
    ("file", File),
];
pub(crate) const NAMESPACE: &[(&str, CellKind)] = &[
    ("scope", Node),
    ("name", Symbol),
    ("first", Number),
    ("first-mutation", Number),
    ("first-written", Number),
    ("file", File),
];

pub(crate) fn visit(
    name: &str,
    source: &mut ColumnFile<'_, '_>,
    file: u64,
    mut symbol: impl FnMut(&str) -> u64,
    mut row: impl FnMut(&[u64]),
) -> bool {
    match name {
        fact_catalog::ASSIGNMENT_SOURCE_REL => {
            source.visit_assignment_sources(|target, value, operation, association| {
                row(&[
                    u64::from(target),
                    u64::from(value),
                    symbol(operation),
                    symbol(association),
                    file,
                ]);
            });
            true
        }
        fact_catalog::NAMESPACE_SITES_REL => {
            source.visit_namespace_sites(|scope, name, first, mutation, written| {
                let null = super::super::finish::NULL;
                row(&[
                    u64::from(scope),
                    symbol(name),
                    first,
                    mutation.unwrap_or(null),
                    written.map_or(null, u64::from),
                    file,
                ]);
            })
        }
        _ => unreachable!("declared site relation"),
    }
}
