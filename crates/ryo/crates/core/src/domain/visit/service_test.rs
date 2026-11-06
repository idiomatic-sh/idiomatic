//! The `visit` service's cases, on values alone.

use ryo_relations::domain::relations::types::Relation;

use super::errors::VisitError;
use super::service::{lines_in, roles_of, walked, wanted, wants};
use super::types::{BorrowedBytes, Noting, RoleRead};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::{Comparison, Operand, Test};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use ryo_values::domain::plan_parts::types::StepScope;

/// What a row asks: the relations a plan reads when the catalog is this, or the ones among these a visit walks.
#[derive(Clone)]
enum Question {
    /// The catalog the visit is planned against.
    Catalog(Vec<Relation>),
    /// The relations wanted.
    Wanted(Vec<Relation>),
    /// The bytes of a file, whose lines are counted.
    Bytes(&'static [u8]),
    /// A plan that reads the edges of the tree in this way.
    Edges(Reading),
    /// The same consumer operations over the explicit semantic projection.
    SemanticEdges(Reading),
    /// What the visit wants of a plan that reads this relation, when the catalog lists these.
    Wants(Relation, Vec<Relation>, Aspect),
}

/// Which part of what a visit wants a row asks about.
#[derive(Clone, Copy)]
enum Aspect {
    /// How many relations the plans read.
    Wanted,
    /// How many of them the files are walked for.
    Walked,
    /// Whether the languages write their notes: one if they do.
    Noted,
}

/// How a plan reads the edges of the tree.
#[derive(Clone, Copy)]
enum Reading {
    /// It filters by field but returns only a node ID.
    FiltersField,
    /// It keeps the `field` column.
    KeepsField,
    /// It keeps the parent and the child.
    KeepsParentAndChild,
    /// It keeps every column.
    KeepsAll,
    /// It walks them as the edges of a hierarchy.
    WalksThem,
}
/// What a row asks.
type Input = Question;
/// How many relations the visit builds.
type Value = usize;
type Failure = VisitError;

fn plan_reading_function() -> Plan {
    plan_reading(Relation::Function)
}

fn plan_reading(relation: Relation) -> Plan {
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: Vec::new(),
        steps: vec![PlanStep {
            operation: Operation::Select {
                from: PlanInput::Relation(relation),
                comparisons: Vec::new(),
                kept: Vec::new(),
            },
            scope: StepScope::PerFile,
        }],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: Vec::new(),
        comparison_origins: Vec::new(),
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "semantic field predicates demand role computation even when field is not emitted",
        ),
        word: Word::Succeeds,
        input: || Question::SemanticEdges(Reading::FiltersField),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("native field predicates demand the same metadata"),
        word: Word::Succeeds,
        input: || Question::Edges(Reading::FiltersField),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("semantic projection of field demands role computation"),
        word: Word::Succeeds,
        input: || Question::SemanticEdges(Reading::KeepsField),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("semantic identity-only projection does not demand roles"),
        word: Word::Succeeds,
        input: || Question::SemanticEdges(Reading::KeepsParentAndChild),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_read_relation_is_wanted_once"),
        word: Word::Succeeds,
        input: || Question::Catalog(vec![Relation::Function]),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_edges_of_the_tree_are_read_from_the_tree_and_not_walked"),
        word: Word::Succeeds,
        input: || {
            Question::Wanted(vec![
                Relation::Function,
                Relation::SyntaxChild,
                Relation::SyntaxTree,
                Relation::Reference,
            ])
        },
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("each_line_feed_ends_a_line_and_a_last_line_with_none_counts"),
        word: Word::Succeeds,
        input: || Question::Bytes(b"a\n\nb"),
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("a_file_ending_in_a_line_feed_has_no_line_after_it"),
        word: Word::Succeeds,
        input: || Question::Bytes(b"a\nb\n"),
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("an_empty_file_has_no_lines"),
        word: Word::Succeeds,
        input: || Question::Bytes(b""),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_plan_that_keeps_the_field_column_asks_for_the_roles"),
        word: Word::Succeeds,
        input: || Input::Edges(Reading::KeepsField),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_plan_that_keeps_every_column_of_the_edges_asks_for_the_roles"),
        word: Word::Succeeds,
        input: || Input::Edges(Reading::KeepsAll),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_plan_that_keeps_the_parent_and_the_child_asks_for_none"),
        word: Word::Succeeds,
        input: || Input::Edges(Reading::KeepsParentAndChild),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_plan_that_walks_the_edges_asks_for_none"),
        word: Word::Succeeds,
        input: || Input::Edges(Reading::WalksThem),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_plan_that_reads_a_relation_wants_it"),
        word: Word::Succeeds,
        input: || Question::Wants(Relation::Function, vec![Relation::Function], Aspect::Wanted),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_relation_the_files_are_walked_for_is_walked"),
        word: Word::Succeeds,
        input: || Question::Wants(Relation::Function, vec![Relation::Function], Aspect::Walked),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_edges_of_the_tree_are_wanted_and_not_walked"),
        word: Word::Succeeds,
        input: || {
            Question::Wants(
                Relation::SyntaxChild,
                vec![Relation::SyntaxChild],
                Aspect::Walked,
            )
        },
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("notes_are_wanted_when_a_plan_reads_the_source_dependencies"),
        word: Word::Succeeds,
        input: || {
            Question::Wants(
                Relation::SourceDependency,
                vec![Relation::SourceDependency],
                Aspect::Noted,
            )
        },
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("namespace_membership_alone_demands_import_notes"),
        word: Word::Succeeds,
        input: || {
            Question::Wants(
                Relation::ProjectNamespaceMembership,
                vec![Relation::ProjectNamespaceMembership],
                Aspect::Noted,
            )
        },
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("notes_are_not_wanted_when_no_plan_reads_what_they_are_for"),
        word: Word::Succeeds,
        input: || Question::Wants(Relation::Function, vec![Relation::Function], Aspect::Noted),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_plan_reading_a_relation_the_catalog_lacks_is_refused_by_the_visit_wants"),
        word: Word::Refuses,
        input: || Question::Wants(Relation::Function, Vec::new(), Aspect::Wanted),
        expect: Expect::Error(VisitError::UnknownRelation {
            relation: Relation::Function,
        }),
    },
    Case {
        name: CaseName("a_relation_the_catalog_lacks_is_refused"),
        word: Word::Refuses,
        input: || Question::Catalog(Vec::new()),
        expect: Expect::Error(VisitError::UnknownRelation {
            relation: Relation::Function,
        }),
    },
];

/// A plan with one step that reads the edges of the tree as `reading` says.
fn plan_reading_edges(reading: Reading, relation: Relation) -> Plan {
    let edges = PlanInput::Relation(relation);
    let operation = match reading {
        Reading::FiltersField => Operation::Select {
            from: edges,
            comparisons: vec![Comparison {
                test: Test::Equal,
                left: Operand::Column(ColumnIndex(3)),
                right: Operand::Column(ColumnIndex(3)),
                origin: None,
            }],
            kept: vec![ColumnIndex(1)],
        },
        Reading::KeepsField => Operation::Project {
            from: edges,
            columns: vec![ColumnIndex(0), ColumnIndex(3)],
            distinct: RepeatedRows::Dropped,
        },
        Reading::KeepsParentAndChild => Operation::Project {
            from: edges,
            columns: vec![ColumnIndex(0), ColumnIndex(1)],
            distinct: RepeatedRows::Dropped,
        },
        Reading::KeepsAll => Operation::Select {
            from: edges,
            comparisons: Vec::new(),
            kept: Vec::new(),
        },
        Reading::WalksThem => Operation::Closure {
            edges,
            keys: Vec::new(),
            source: ColumnIndex(0),
            destination: ColumnIndex(1),
        },
    };
    Plan {
        steps: vec![PlanStep {
            operation,
            scope: StepScope::PerFile,
        }],
        ..plan_reading_function()
    }
}

fn subject(input: Question) -> Result<Value, Failure> {
    match input {
        Question::Catalog(catalog) => {
            let plan = plan_reading_function();
            wanted(&[&plan, &plan], &catalog).map(|wanted| wanted.len())
        }
        Question::Wanted(wanted) => Ok(walked(&wanted).len()),
        Question::Edges(reading) => Ok(usize::from(
            roles_of(&[&plan_reading_edges(reading, Relation::SyntaxChild)]) == RoleRead::Reads,
        )),
        Question::SemanticEdges(reading) => Ok(usize::from(
            roles_of(&[&plan_reading_edges(reading, Relation::SemanticChild)]) == RoleRead::Reads,
        )),
        Question::Wants(read, catalog, aspect) => {
            let wants = wants(&[plan_reading(read)], &catalog)?;
            Ok(match aspect {
                Aspect::Wanted => wants.wanted.len(),
                Aspect::Walked => wants.walked.len(),
                Aspect::Noted => usize::from(wants.noted == Noting::Wanted),
            })
        }
        Question::Bytes(bytes) => {
            Ok(usize::try_from(lines_in(BorrowedBytes(bytes)).0).unwrap_or(usize::MAX))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
