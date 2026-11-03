//! The `crossing` service's cases, on values alone.
use crate::application::file_dag::crossing::cross_rule;

use crate::application::file_dag::settings::CrossingInput;
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::domain::answering::types::Todo;
use crate::domain::coverage::types::{Covered, FilePartial, Lacking, Partial, RuleIndex};
use crate::domain::crossing::service::{partials_for, placeable};
use crate::domain::crossing::types::Placeability;
use crate::domain::frontiers::service::decode_input;
use crate::domain::rules::errors::RulesError;
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::{Frontier, ParameterBinding, Table};
use crate::domain::table_words::service::{encode_table, pack};
use crate::domain::table_words::types::WordSlice;
use crate::domain::trust::service::files_for;
use crate::domain::trust::types::{Needs, RelationRead};
use ryo_relations::domain::relations::types::{
    CellKind, FileId, Name, NamePlace, Offset, Origin, PlaceNumber, Relation, TextHash,
    Value as Cell, Words, WorkerNumber,
};
use ryo_language::domain::symbols::service::places_mut;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::runs::types::{RuleId, SkipReason, Skipped};
use ryo_values::domain::stored_facts::types::PartialCause;

use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as PlanInput, RepeatedRows, StepId,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep, StepScope};
use ryo_values::domain::stored_facts::types::{ArenaBytes, KeptWords};

/// What a row of the cases asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// Findings whose columns do and do not say where they are.
    Placeable,
    /// Files and the project that read a relation in part which a rule trusts.
    Partials,
    /// The names two files handed over, counted.
    CrossCounts,
    /// A rule the plans lack.
    CrossNoRule,
    /// Two names of one hash spelled two ways.
    CrossSpellings,
    /// A cross-file step that reads what nobody handed over.
    CrossNothingHanded,
}

/// What a row of the cases asks of the service.
type Input = Scenario;
/// Up to three numbers a row answers; a place a row has no number for holds zero.
type Value = [usize; 3];
type Failure = RulesError;

/// What the cross-file steps of every rule read, owned.
struct World {
    plans: Vec<Plan>,
    bindings: Vec<Vec<ParameterBinding>>,
    ids: Vec<RuleId>,
    needs: Vec<Needs>,
    arenas: Vec<ArenaBytes>,
    covered: Vec<Covered>,
    project: Vec<Partial>,
    skipped: Vec<Skipped>,
    paths: Vec<ryo_language::domain::files::types::FilePath>,
}

/// A plan that counts across files the names a per-file step handed over, and emits the column `subject` of the counts.
fn counting_plan(subject: u32, key: Vec<ColumnIndex>) -> Plan {
    Plan {
        parameters: Vec::new(),
        steps: vec![
            PlanStep {
                operation: Operation::Project {
                    from: PlanInput::Relation(Relation::Function),
                    columns: vec![ColumnIndex(0)],
                    distinct: RepeatedRows::Dropped,
                },
                scope: StepScope::PerFile,
            },
            PlanStep {
                operation: Operation::GroupCount {
                    from: PlanInput::Step(StepId(0)),
                    keys: vec![ColumnIndex(0)],
                    distinct: None,
                },
                scope: StepScope::CrossFile,
            },
        ],
        emit: Emit {
            step: StepId(1),
            subject: ColumnIndex(subject),
            key,
            headings: Vec::new(),
        },
        frontier: vec![PlanInput::Step(StepId(0))],
        comparison_origins: Vec::new(),
    }
}

fn world(plan: Plan) -> World {
    World {
        plans: vec![plan],
        bindings: vec![Vec::new()],
        ids: vec![RuleId("rule".into())],
        needs: vec![Needs::default()],
        arenas: Vec::new(),
        covered: Vec::new(),
        project: Vec::new(),
        skipped: Vec::new(),
        paths: Vec::new(),
    }
}

fn crossing(world: &World) -> CrossingInput<'_> {
    CrossingInput {
        executor: &crate::application::file_dag::execution::CommonOperatorEngine,
        operators: crate::application::file_dag::testing::request_fixture(
            &world.plans,
            &world.bindings,
            &[],
        ),
        domain: crate::domain::crossing::types::CrossingInput {
            unknown: &[],
            plans: &world.plans,
            bindings: &world.bindings,
            ids: &world.ids,
            needs: &world.needs,
            paths: &world.paths,
            arenas: &world.arenas,
            files: &[],
            covered: &world.covered,
            skipped: &world.skipped,
            project: &world.project,
            hosted: &[],
            answering: &crate::domain::render::types::Answering::Findings,
        },
    }
}

#[test]
fn rejected_syntax_blocks_cross_file_complements_without_becoming_a_partial_diagnostic() {
    let mut world = world(counting_plan(0, Vec::new()));
    let path = ryo_language::domain::files::types::FilePath("rejected.py".into());
    world.paths.push(path.clone());
    world.needs[0].cross.push(RelationRead {
        relation: Relation::Function,
        kinds: Vec::new(),
    });
    world.skipped.push(Skipped {
        rule: world.ids[0].clone(),
        path,
        reason: SkipReason::SyntaxRejected,
    });

    let input = crossing(&world);
    let partials = partials_for(&input, RuleIndex(0));
    assert!(
        partials.entries.is_empty(),
        "no provider partial was invented"
    );
    assert_eq!(
        files_for(&partials, &world.needs[0].cross),
        [Some(FileId(0))],
        "the skipped file remains unavailable to aggregate absence checks"
    );
}

fn todo(rule: usize) -> Todo {
    Todo {
        rule: RuleIndex(rule),
        id: RuleId("rule".into()),
    }
}

fn partial(relation: Relation) -> Partial {
    Partial {
        relation,
        cause: PartialCause("a construct the reader cannot represent".into()),
        lacking: Lacking::Any,
        at: None,
    }
}

fn name(hash: u64, place: u32) -> Cell {
    Cell::Name(Name {
        hash: TextHash(hash),
        place: PlaceNumber(place),
    })
}

/// The words of a table of one column of names, one row for each of the given places and hashes.
fn words_of(names: &[(u64, u32)]) -> KeptWords {
    let mut table = Table {
        kinds: vec![CellKind::Symbol],
        ..Table::default()
    };
    for (hash, place) in names {
        push_row(&mut table, &[name(*hash, *place)]);
    }
    let mut words = Words::default();
    encode_table(&table, &[], &mut words);
    KeptWords(pack(WordSlice(&words.0)))
}

/// The rows the files handed to the first input of a counting plan: two files handed over the name hashed 1, a third another name.
fn handed_over(cross: &mut Symbols) -> Vec<Frontier> {
    let table = decode_input(&words_of(&[(1, 0), (1, 0), (2, 0)]), cross);
    vec![Frontier {
        input: PlanInput::Step(StepId(0)),
        table,
    }]
}

/// The file of `partial`, as a number: the repository as a whole is the number past every file, as the store writes it; 0
/// when there is no such partial.
fn file_number(partial: Option<&FilePartial>) -> usize {
    partial.map_or(0, |each| {
        usize::try_from(each.file.map_or(u32::MAX, |file| file.0)).unwrap_or(0)
    })
}

/// The partials of a rule that reads what the project wrote: the files that cover it, in order.
fn partials_found() -> Value {
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
    let found = partials_for(&crossing(&rules), RuleIndex(0)).entries;
    [
        found.len(),
        file_number(found.first()),
        file_number(found.get(1)),
    ]
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(match scenario {
        Scenario::Placeable => {
            let table = Table {
                kinds: vec![CellKind::Symbol, CellKind::Number],
                ..Table::default()
            };
            let says = |emitted, key| {
                usize::from(
                    placeable(&counting_plan(emitted, key), &table) == Placeability::Placeable,
                )
            };
            [
                says(0, Vec::new()),
                says(1, Vec::new()),
                says(1, vec![ColumnIndex(0)]),
            ]
        }
        Scenario::Partials => partials_found(),
        Scenario::CrossCounts => {
            let rules = world(counting_plan(1, Vec::new()));
            let mut cross = Symbols::default();
            let frontier = handed_over(&mut cross);
            let done = cross_rule(&crossing(&rules), &todo(0), Vec::from(frontier), &mut cross)?;
            [done.table.count.0, done.fed.len(), done.table.cells.len()]
        }
        Scenario::CrossNoRule => {
            let rules = world(counting_plan(1, Vec::new()));
            let done = cross_rule(
                &crossing(&rules),
                &todo(3),
                Vec::new(),
                &mut Symbols::default(),
            )?;
            [done.table.count.0, done.fed.len(), done.table.cells.len()]
        }
        Scenario::CrossSpellings => {
            let mut rules = world(counting_plan(1, Vec::new()));
            rules.arenas = vec![ArenaBytes(b"ab".to_vec())];
            let mut cross = Symbols::default();
            for (start, end) in [(0, 1), (1, 2)] {
                places_mut(&mut cross).push(NamePlace {
                    origin: Origin::Derived(WorkerNumber(0)),
                    file: FileId(0),
                    start: Offset(start),
                    end: Offset(end),
                });
            }
            let table = decode_input(&words_of(&[(1, 0), (1, 1)]), &mut Symbols::default());
            let frontier = [Frontier {
                input: PlanInput::Step(StepId(0)),
                table,
            }];
            let done = cross_rule(&crossing(&rules), &todo(0), Vec::from(frontier), &mut cross)?;
            [done.table.count.0, 0, 0]
        }
        Scenario::CrossNothingHanded => {
            let rules = world(counting_plan(1, Vec::new()));
            let done = cross_rule(
                &crossing(&rules),
                &todo(0),
                Vec::new(),
                &mut Symbols::default(),
            )?;
            [done.table.count.0, 0, 0]
        }
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_finding_is_placeable_by_a_name_or_a_key_that_says_where_it_is"),
        word: Word::Succeeds,
        input: || Scenario::Placeable,
        expect: Expect::Value([1, 0, 1]),
    },
    Case {
        name: CaseName(
            "the_partial_reads_of_the_project_and_the_files_that_govern_a_rule_bear_on_it",
        ),
        word: Word::Succeeds,
        input: || Scenario::Partials,
        expect: Expect::Value([2, 4_294_967_295, 2]),
    },
    Case {
        name: CaseName("the_names_two_files_handed_over_are_counted_into_two_findings"),
        word: Word::Succeeds,
        input: || Scenario::CrossCounts,
        expect: Expect::Value([2, 2, 4]),
    },
    Case {
        name: CaseName("a_rule_the_plans_lack_crosses_to_nothing"),
        word: Word::Succeeds,
        input: || Scenario::CrossNoRule,
        expect: Expect::Value([0, 0, 0]),
    },
    Case {
        name: CaseName("one_hash_spelled_two_ways_fails_the_rule"),
        word: Word::Refuses,
        input: || Scenario::CrossSpellings,
        expect: Expect::Refusal(|error| {
            if matches!(error, RulesError::Failed { .. }) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_step_that_reads_what_nobody_handed_over_fails_the_rule"),
        word: Word::Refuses,
        input: || Scenario::CrossNothingHanded,
        expect: Expect::Refusal(|error| {
            if matches!(error, RulesError::Failed { .. }) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
