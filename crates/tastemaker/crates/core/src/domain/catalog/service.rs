//! The `catalog` service: free functions named by verb that write what the engine's catalog says as sections of Markdown
//! lines, a line per item, with no IO and no ports.

use super::constants::{
    COLUMNS, EXAMPLE, FIELDS, LANGUAGES, OPTIONAL, OPTIONS, READS, REQUIRED, RYOQL, SEPARATOR,
    WALKTHROUGH, WORKED, WORKED_QUERIES, WRITING,
};
use super::types::{
    CatalogEntry, CatalogName, OperatorPage, Production, Reading, RelationPage, Requirement,
};
use crate::domain::explanations::types::{Section, SectionLine, SectionTitle};

/// Names as one backticked list: `a`, `b`.
fn listed(names: &[CatalogName]) -> SectionLine {
    SectionLine(
        names
            .iter()
            .map(|name| format!("`{}`", name.0))
            .collect::<Vec<_>>()
            .join(SEPARATOR)
            .into(),
    )
}

/// A section of `title` holding `lines`.
fn section(title: SectionTitle, lines: Vec<SectionLine>) -> Section {
    Section { title, lines }
}

/// Relations or operators, a line each: `- `name`: what it is`.
#[must_use]
pub fn entry_lines(entries: &[CatalogEntry]) -> Vec<SectionLine> {
    entries
        .iter()
        .map(|entry| SectionLine(format!("- `{}`: {}", entry.name.0, entry.summary.0).into()))
        .collect()
}

/// A relation: what it holds, a line per column, a line per linked language.
#[must_use]
pub fn relation_sections(page: &RelationPage) -> Vec<Section> {
    let columns = page
        .columns
        .iter()
        .map(|column| {
            SectionLine(
                if column.words.is_empty() {
                    format!("- `{}` {}", column.name.0, column.kind.0)
                } else {
                    format!(
                        "- `{}` {}, one of {}",
                        column.name.0,
                        column.kind.0,
                        listed(&column.words).0
                    )
                }
                .into(),
            )
        })
        .collect();
    let languages = page
        .languages
        .iter()
        .map(|language| {
            let reading = match &language.reading {
                Reading::Reads => READS.to_owned(),
                Reading::Absent(reason) => format!("absent ({})", reason.0),
            };
            SectionLine(
                if language.unknowns.is_empty() {
                    format!("- {}: {reading}", language.name.0)
                } else {
                    format!(
                        "- {}: {reading}; unknown values {}",
                        language.name.0,
                        listed(&language.unknowns).0
                    )
                }
                .into(),
            )
        })
        .collect();
    let mut sections = vec![
        section(
            SectionTitle(page.entry.name.0.clone()),
            vec![SectionLine(page.entry.summary.0.clone())],
        ),
        section(SectionTitle(COLUMNS.into()), columns),
    ];
    // A build that links no language says nothing of languages, rather than an empty heading.
    if !page.languages.is_empty() {
        sections.push(section(SectionTitle(LANGUAGES.into()), languages));
    }
    sections
}

/// An operator: what it does, a line per field and per choice, and its example.
#[must_use]
pub fn operator_sections(page: &OperatorPage) -> Vec<Section> {
    let fields = page
        .fields
        .iter()
        .map(|field| {
            let requirement = match field.requirement {
                Requirement::Required => REQUIRED,
                Requirement::Optional => OPTIONAL,
            };
            SectionLine(format!("- `{}` {requirement}: {}", field.name.0, field.summary.0).into())
        })
        .collect();
    let mut sections = vec![
        section(
            SectionTitle(page.entry.name.0.clone()),
            vec![SectionLine(page.entry.summary.0.clone())],
        ),
        section(SectionTitle(FIELDS.into()), fields),
    ];
    if !page.options.is_empty() {
        sections.push(section(
            SectionTitle(OPTIONS.into()),
            page.options
                .iter()
                .map(|option| SectionLine(format!("- {}", option.0).into()))
                .collect(),
        ));
    }
    sections.push(section(
        SectionTitle(EXAMPLE.into()),
        vec![SectionLine(format!("`{}`", page.example.0).into())],
    ));
    sections
}

/// How a query is written, four steps and the worked queries, then the grammar, a line per production: its name, its form
/// and an example the parser reads.
#[must_use]
pub fn grammar_sections(productions: &[Production]) -> Vec<Section> {
    let worked = WORKED_QUERIES
        .iter()
        .map(|(checks, query)| SectionLine(format!("{checks}\n\n```text\n{query}\n```\n").into()))
        .collect();
    vec![
        section(
            SectionTitle(WRITING.into()),
            WALKTHROUGH
                .iter()
                .map(|step| SectionLine((*step).into()))
                .collect(),
        ),
        section(SectionTitle(WORKED.into()), worked),
        section(
            SectionTitle(RYOQL.into()),
            productions
                .iter()
                .map(|production| {
                    SectionLine(
                        format!(
                            "- {}: `{}`, as in `{}`",
                            production.name.0, production.form.0, production.example.0
                        )
                        .into(),
                    )
                })
                .collect(),
        ),
    ]
}
