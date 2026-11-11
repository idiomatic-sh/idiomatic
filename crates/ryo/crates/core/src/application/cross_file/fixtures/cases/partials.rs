//! Which findings can say where they are, and which partial reads bear on a rule.
use crate::application::cross_file::fixtures::counting_plan;
use crate::application::cross_file::fixtures::crossing;
use crate::application::cross_file::fixtures::world;
use crate::domain::crossing::service::partials_for;
use crate::domain::crossing::service::placeable;
use crate::domain::crossing::types::Placeability;
use ryo_execution::inbound::use_cases::metadata::interface::Covered;
use ryo_execution::inbound::use_cases::metadata::interface::FilePartial;
use ryo_execution::inbound::use_cases::metadata::interface::Lacking;
use ryo_execution::inbound::use_cases::metadata::interface::Needs;
use ryo_execution::inbound::use_cases::metadata::interface::Partial;
use ryo_execution::inbound::use_cases::metadata::interface::PartialPlace;
use ryo_execution::inbound::use_cases::metadata::interface::RelationRead;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_execution::inbound::use_cases::metadata::use_case::MetadataUseCaseImpl;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::stored_facts::types::PartialCause;

fn partial(relation: Relation) -> Partial {
    Partial {
        relation,
        cause: PartialCause("a construct the reader cannot represent".into()),
        lacking: Lacking::Any,
        at: ryo_execution::inbound::use_cases::metadata::interface::PartialPlace::Repository,
    }
}

/// The file of `partial`, as a number: the repository as a whole is the number past every file, as the store writes it; 0
/// when there is no such partial.
fn file_number(partial: Option<&FilePartial>) -> usize {
    partial.map_or(0, |each| {
        usize::try_from(match each.file {
            PartialPlace::File(file) => file.0,
            PartialPlace::Repository => u32::MAX,
        })
        .unwrap_or(0)
    })
}

/// The partials of a rule that reads what the project wrote: the files that cover it, in order.
pub(crate) fn partials_found() -> [usize; 3] {
    let mut rules = world(counting_plan(1, Vec::new()));
    rules.needs = vec![Needs {
        cross: vec![RelationRead {
            relation: Relation::Function,
            kinds: Vec::new(),
        }],
        ..Needs::default()
    }];
    rules.project = vec![
        partial(Relation::Function),
        partial(Relation::PackageDependency),
    ];
    rules.covered = vec![
        Covered {
            file: FileId(2),
            rules: vec![RuleIndex(0)],
            partial: vec![partial(Relation::Function)],
        },
        Covered {
            file: FileId(3),
            rules: vec![RuleIndex(1)],
            partial: vec![partial(Relation::Function)],
        },
    ];
    let found = partials_for(&MetadataUseCaseImpl, &crossing(&rules), RuleIndex(0)).entries;
    [
        found.len(),
        file_number(found.first()),
        file_number(found.get(1)),
    ]
}

pub(crate) fn placing_outcomes() -> [usize; 3] {
    let table = Table {
        kinds: vec![CellKind::Symbol, CellKind::Number],
        ..Table::default()
    };
    let says = |emitted, key| {
        usize::from(placeable(&counting_plan(emitted, key).emit, &table) == Placeability::Placeable)
    };
    [
        says(0, Vec::new()),
        says(1, Vec::new()),
        says(1, vec![ColumnIndex(0)]),
    ]
}
