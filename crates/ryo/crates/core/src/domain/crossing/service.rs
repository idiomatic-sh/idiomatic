//! The `crossing` service: free functions named by verb over the cross-file steps of one rule, with no IO and no ports.

use super::types::{CrossingInput, Evaluated, Placeability};
use crate::domain::answering::types::Todo;
use crate::domain::coverage::types::{FilePartial, Partials, RuleIndex};
use crate::domain::finish::service::carry;
use crate::domain::finish::types::{Arenas, Resolving};
use crate::domain::hosted::service::table_of;
use crate::domain::hosted::types::NumberedFiles;
use crate::domain::operating::types::OperatorContext;

use crate::domain::operator::types::RowsFed;

use crate::domain::rules::errors::RulesError;
use crate::domain::rules::service::failed;
use crate::domain::table_model::types::Frontier;
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use crate::domain::trust::service::matters;
use crate::domain::trust::types::Bearing;
use ryo_relations::domain::relations::types::CellKind;

use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::plans::types::{Emit, Operation, Plan};

/// Whether an emitted finding can have a place in a file: a stretch, a node or a name among its columns.
#[must_use]
pub(crate) fn placeable(emit: &Emit, table: &Table) -> Placeability {
    let places = emit
        .key
        .iter()
        .chain(std::iter::once(&plan.emit.subject))
        .any(|column| {
            table
                .kinds
                .get(usize::try_from(column.0).unwrap_or(usize::MAX))
                .is_some_and(|kind| {
                    matches!(kind, CellKind::Span | CellKind::Node | CellKind::Symbol)
                })
        });
    if places {
        Placeability::Placeable
    } else {
        Placeability::Unplaceable
    }
}

/// The files that govern a rule, in which a relation its cross-file steps trust the absence of rows of is read in part.
#[must_use]
pub(crate) fn partials_for(crossing: &CrossingInput<'_>, rule: RuleIndex) -> Partials {
    let trusted = crossing
        .needs
        .get(rule.0)
        .map_or(&[][..], |needs| needs.cross.as_slice());
    partials_for_reads(crossing, rule, trusted)
}

/// Publication completeness uses actual root reads, not only absence-sensitive diagnostic Needs.
pub(crate) fn partials_for_reads(
    crossing: &CrossingInput<'_>,
    rule: RuleIndex,
    trusted: &[crate::domain::trust::types::RelationRead],
) -> Partials {
    let matter = |partial: &&crate::domain::coverage::types::Partial| {
        trusted
            .iter()
            .any(|read| matters(read, partial) == Bearing::Matters)
    };
    Partials {
        entries: crossing
            .project
            .iter()
            .filter(matter)
            .map(|partial| FilePartial {
                file: partial.at,
                partial: partial.clone(),
            })
            .chain(
                crossing
                    .covered
                    .iter()
                    .filter(|each| each.rules.contains(&rule))
                    .flat_map(|each| {
                        each.partial
                            .iter()
                            .filter(matter)
                            .map(move |partial| FilePartial {
                                file: Some(each.file),
                                partial: partial.clone(),
                            })
                    }),
            )
            .collect(),
        unavailable_files: crossing
            .ids
            .get(rule.0)
            .into_iter()
            .flat_map(|id| crossing.skipped.iter().filter(move |skip| skip.rule == *id))
            .filter_map(|skip| {
                crossing
                    .paths
                    .iter()
                    .position(|path| *path == skip.path)
                    .and_then(|place| u32::try_from(place).ok())
                    .map(ryo_relations::domain::relations::types::FileId)
            })
            .collect(),
    }
}

/// The rows the host declared, made once for the finish with their text in the names of the finish; none when the plan reads none.
pub(crate) fn hosted_tables(
    crossing: &CrossingInput<'_>,
    plan: &Plan,
    cross: &mut Symbols,
) -> Vec<Table> {
    let reads_hosted = plan
        .steps
        .iter()
        .any(|step| matches!(step.operation, Operation::Hosted { .. }));
    if !reads_hosted {
        return Vec::new();
    }
    let numbered: Vec<FilePath> = crossing
        .files
        .iter()
        .map(|file| ryo_values::domain::files::service::input_path(file).clone())
        .collect();
    crossing
        .hosted
        .iter()
        .map(|relation| {
            let mut text = Symbols::default();
            let mut table = table_of(relation, NumberedFiles(&numbered), &mut text);
            carry(&mut table, &text, cross);
            table
        })
        .collect()
}

use ryo_values::domain::rows::types::Table;
