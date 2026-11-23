//! First-half views of the provider's additional walk relations.
use super::{fact_catalog, first::WalkFact, pair_summary::Source};
use ryo::languages::{
    CellKind, CellWord, FactAnswer, FileWords, FilledColumn, FilledColumns, Wanted,
};
use ryo_python::fixtures::columns::{BindingSite, ColumnFile};

use CellKind::{File, Node, Number};

#[path = "pair_predecessor.rs"]
pub(super) mod predecessor;
#[path = "pair_sites.rs"]
pub(super) mod sites;

// Columns of the actual binding-site answer are shared with the second-half adapter.
pub(super) const SITES: &[(&str, CellKind)] = &[
    ("binding", Node),
    ("stores", Number),
    ("removals", Number),
    ("later", Number),
    ("statement", Node),
    ("series-end", Node),
    ("next", Node),
    ("file", File),
];
pub(super) const FACTS: &[WalkFact] = &[
    WalkFact {
        name: fact_catalog::BINDING_READ_REL,
        columns: sites::READ_WITNESS,
        exact: true,
    },
    WalkFact {
        name: fact_catalog::ASSIGNMENT_SOURCE_REL,
        columns: sites::ASSIGNMENT,
        exact: true,
    },
    WalkFact {
        name: fact_catalog::NAMESPACE_SITES_REL,
        columns: sites::NAMESPACE,
        exact: true,
    },
    WalkFact {
        name: fact_catalog::BINDING_SITES_REL,
        columns: SITES,
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

pub(super) fn site_cells(site: BindingSite, file: usize) -> [u64; 8] {
    [
        u64::from(site.binding),
        site.stores as u64,
        site.removals as u64,
        site.later as u64,
        site.statement.map_or(super::finish::NULL, u64::from),
        u64::from(site.series_end),
        site.next.map_or(super::finish::NULL, u64::from),
        file as u64,
    ]
}
/// The walk-site iterators expose table positions. Encode their node
/// columns at this boundary in the same file-qualified form as catalog rows.
fn write_positions(out: &mut FilledColumns, schema: &[(&str, CellKind)], row: &[u64], file: ryo::languages::FileId) {
    for ((column, (_, kind)), &cell) in out.0.iter_mut().zip(schema).zip(row) {
        let word = if *kind == Node && cell != super::finish::NULL {
            let at = ryo::languages::Position(u32::try_from(cell).expect("walk node position"));
            ryo_relations::domain::relations::service::node_id(file, at).0
        } else {
            cell
        };
        column.0.push(CellWord(word));
    }
}

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
    let file_id = ryo::languages::FileId(number as u32);
    match name {
        fact_catalog::BINDING_SITES_REL => file.visit_binding_sites(|site| {
            write_positions(out, schema.columns, &site_cells(site, number), file_id);
        }),
        fact_catalog::ASSIGNMENT_SOURCE_REL | fact_catalog::NAMESPACE_SITES_REL | fact_catalog::BINDING_READ_REL => sites::visit(
            name,
            file,
            number as u64,
            |s| {
                ryo::languages::facts::intern(
                    words,
                    ryo_relations::domain::terms::types::NameText(s),
                )
                .0
            },
            |row| write_positions(out, schema.columns, row, file_id),
        ),
        fact_catalog::PREDECESSOR_REL => predecessor::read(file, number, words, out),
        fact_catalog::RELATIVE_IMPORT_REL => {
            for (node, _, _, _) in file.relative_imports() {
                write_positions(out, schema.columns, &[u64::from(node), number as u64], file_id);
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
