//! First-half views of the provider's additional walk relations.
use super::{fact_catalog, first::WalkFact, pair_summary::Source};
use ryo::languages::{
    CellKind, CellWord, FactAnswer, FileWords, FilledColumn, FilledColumns, Wanted,
};
use ryo_python::fixtures::columns::ColumnFile;

use CellKind::{File, Node};

#[path = "pair_predecessor.rs"]
pub(super) mod predecessor;
pub(super) const FACTS: &[WalkFact] = &[
    WalkFact {
        name: ColumnFile::BINDING_READ_SCHEMA.name,
        columns: ColumnFile::BINDING_READ_SCHEMA.columns,
        exact: true,
    },
    WalkFact {
        name: ColumnFile::ASSIGNMENT_SOURCE_SCHEMA.name,
        columns: ColumnFile::ASSIGNMENT_SOURCE_SCHEMA.columns,
        exact: true,
    },
    WalkFact {
        name: ColumnFile::NAMESPACE_SITES_SCHEMA.name,
        columns: ColumnFile::NAMESPACE_SITES_SCHEMA.columns,
        exact: true,
    },
    WalkFact {
        name: ColumnFile::BINDING_SITES_SCHEMA.name,
        columns: ColumnFile::BINDING_SITES_SCHEMA.columns,
        exact: true,
    },
    WalkFact {
        name: fact_catalog::PREDECESSOR_REL,
        columns: predecessor::COLUMNS,
        exact: true,
    },
    // Package identity can remove a relative import. Its indexed occurrence
    // is necessary but cannot by itself certify an exclusion or an output.
    WalkFact {
        name: fact_catalog::RELATIVE_IMPORT_REL,
        columns: &[("occurrence", Node), ("file", File)],
        exact: false,
    },
];

pub(super) fn read(
    file: &mut ColumnFile<'_, '_>,
    number: usize,
    source: Source,
    words: &mut FileWords,
    out: &mut FilledColumns,
) -> bool {
    let name = match source {
        Source::Catalog(rel) => {
            return matches!(
                file.fill(rel, Wanted::Whole, words, out),
                FactAnswer::Answered(_)
            );
        }
        Source::Walk(name) => name,
    };
    let schema = FACTS
        .iter()
        .find(|f| f.name == name)
        .expect("declared walk fact");
    out.0 = (0..schema.columns.len())
        .map(|_| FilledColumn(Vec::new()))
        .collect();
    match name {
        fact_catalog::BINDING_SITES_REL | fact_catalog::ASSIGNMENT_SOURCE_REL |
        fact_catalog::NAMESPACE_SITES_REL | fact_catalog::BINDING_READ_REL => file.visit_site_relation(
            name, number as u64,
            |s| ryo::languages::facts::intern(words, ryo_relations::domain::terms::types::NameText(s)).0,
            |row| {
                for (col, &cell) in out.0.iter_mut().zip(row) {
                    col.0.push(CellWord(cell));
                }
            },
        ).is_ok(),
        fact_catalog::PREDECESSOR_REL => predecessor::read(file, number, words, out),
        fact_catalog::RELATIVE_IMPORT_REL => {
            for (node, _, _, _) in file.relative_imports() {
                out.0[0].0.push(CellWord(u64::from(node)));
                out.0[1].0.push(CellWord(number as u64));
            }
            true
        }
        _ => unreachable!("declared walk fact has a reader"),
    }
}

/// The walk declares the cost of accessing a family. Import and directive rows
/// are file sets; ordered predecessor selection also needs the resolution input.
pub(super) fn family(name: &str) -> super::pair_summary::cost::Family {
    use super::pair_summary::cost::Family::*;
    match name {
        fact_catalog::PREDECESSOR_REL => Schedule,
        "static-import" | "comment" | "filesystem.executable" => Sets,
        super::provider_packaging::MODULE_IDENTITY_REL => Project,
        _ => Fact,
    }
}
