//! The `operator` service's cases: the dispatch of an operation to its operator, what an operation reads, and the cross-file
//! steps on the rows the files handed over. The per-file steps are the `pipeline`'s.

use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{
    CellKind, FileId, Name, PlaceNumber, Relation, TextHash, Value as Cell,
};

use crate::application::file_dag::cross_execution::{compute, evaluate_cross};
use crate::application::file_dag::testing::TestOperator as _;
use crate::domain::coverage::types::Partials;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::collections::operators::project::item::ProjectItem;
use crate::domain::table_model::types::{Frontier, Rows};
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep, StepScope};
use ryo_values::domain::rows::service::push_row;

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Computes a project of one column through the dispatch.
    ProjectsThroughTheDispatch,
    /// Counts across files the names two files handed over.
    CountsHandedOverRows,
    /// Gives a project the operation of a union.
    ProjectGetsAUnion,
    /// Gives a project no input.
    ProjectGetsNoInput,
    /// Extends with a column where a value is meant.
    ExtendsWithAColumn,
    /// Runs a cross-file step that reads a per-file step nobody handed over.
    ReadsAStepNobodyHandedOver,
    /// Counts the relations the operators' examples name that the catalog does not have.
    ExamplesNameRelations,
}

/// What a row of the cases does.
type Input = Scenario;
/// A count: the rows made, the inputs read, or 1 for the family asked.
type Value = usize;
type Failure = OperatingError;

/// A table of files, one column, `rows` rows.
fn files(rows: u32) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::File],
        ..Table::default()
    };
    for file in 0..rows {
        push_row(&mut table, &[Cell::File(FileId(file))]);
    }
    table
}

/// A name that left its file, known by its hash.
fn named(hash: u64) -> Cell {
    Cell::Name(Name {
        hash: TextHash(hash),
        place: PlaceNumber(0),
    })
}

fn project() -> Operation {
    Operation::Project {
        from: PlanInput::Relation(Relation::Function),
        columns: vec![ColumnIndex(0)],
        distinct: RepeatedRows::Dropped,
    }
}

fn union() -> Operation {
    Operation::Union {
        left: PlanInput::Relation(Relation::Function),
        right: PlanInput::Relation(Relation::Function),
    }
}

/// A plan that counts across files the names a per-file step handed over.
fn counting_plan(handed_over: bool) -> Plan {
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: Vec::new(),
        steps: vec![
            PlanStep {
                operation: project(),
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
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: if handed_over {
            vec![PlanInput::Step(StepId(0))]
        } else {
            Vec::new()
        },
        comparison_origins: Vec::new(),
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut symbols = Symbols::default();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
        memo: &mut Vec::new(),
        scratch: &mut OperatorScratch::default(),
    };
    let from = files(2);
    let mut out = Table::default();
    match scenario {
        Scenario::ProjectsThroughTheDispatch => {
            compute(&project(), &[Rows::Computed(&from)], &mut context, &mut out)?;
            Ok(out.count.0)
        }
        Scenario::CountsHandedOverRows => {
            // Two files hand over the name `main`, and a third another name.
            let mut table = Table {
                kinds: vec![CellKind::Symbol],
                ..Table::default()
            };
            for hash in [1, 1, 2] {
                push_row(&mut table, &[named(hash)]);
            }
            let counted = evaluate_cross(
                &counting_plan(true),
                vec![Frontier {
                    input: PlanInput::Step(StepId(0)),
                    table,
                }],
                &Partials::default(),
                &mut context,
                &mut [],
                &[],
            )?;
            Ok(counted.count.0)
        }
        Scenario::ProjectGetsAUnion => {
            ProjectItem.compute(&union(), &[Rows::Computed(&from)], &mut context, &mut out)?;
            Ok(out.count.0)
        }
        Scenario::ProjectGetsNoInput => {
            compute(&project(), &[], &mut context, &mut out)?;
            Ok(out.count.0)
        }
        Scenario::ExtendsWithAColumn => {
            let extend = Operation::Extend {
                from: PlanInput::Relation(Relation::Function),
                values: vec![Operand::Column(ColumnIndex(0))],
            };
            compute(&extend, &[Rows::Computed(&from)], &mut context, &mut out)?;
            Ok(out.count.0)
        }
        Scenario::ReadsAStepNobodyHandedOver => {
            let counted = evaluate_cross(
                &counting_plan(false),
                Vec::new(),
                &Partials::default(),
                &mut context,
                &mut [],
                &[],
            )?;
            Ok(counted.count.0)
        }
        Scenario::ExamplesNameRelations => Ok(examples_naming_no_relation()),
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_dispatch_computes_with_the_operator_the_operation_names"),
        word: Word::Succeeds,
        input: || Scenario::ProjectsThroughTheDispatch,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("every_relation_an_operator_example_names_is_in_the_catalog"),
        word: Word::Succeeds,
        input: || Scenario::ExamplesNameRelations,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_cross_file_step_reads_only_what_the_file_handed_over"),
        word: Word::Succeeds,
        input: || Scenario::CountsHandedOverRows,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("an_operator_refuses_an_operation_that_is_not_its_own"),
        word: Word::Refuses,
        input: || Scenario::ProjectGetsAUnion,
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("an_operator_refuses_to_run_without_the_input_it_reads"),
        word: Word::Refuses,
        input: || Scenario::ProjectGetsNoInput,
        expect: Expect::Error(OperatingError::MissingInput { position: 1 }),
    },
    Case {
        name: CaseName("a_column_is_not_a_value_to_append"),
        word: Word::Refuses,
        input: || Scenario::ExtendsWithAColumn,
        expect: Expect::Error(OperatingError::WrongBinding),
    },
    Case {
        name: CaseName("a_cross_file_step_cannot_read_a_per_file_step_nobody_handed_over"),
        word: Word::Refuses,
        input: || Scenario::ReadsAStepNobodyHandedOver,
        expect: Expect::Error(OperatingError::NotRun),
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

/// How many of the relations the operators' examples name, a quoted name after `from`, `left`, `right`, `edges`, `relation` or
/// `domain`, are no relation of the catalog: an example runs as it is printed only when none is.
fn examples_naming_no_relation() -> Value {
    use ryo_relations::domain::relations::service::relation_named;
    use ryo_relations::domain::relations::types::NameRef;
    let fields = [
        "from",
        "from=",
        "left=",
        "right=",
        "edges=",
        "relation=",
        "domain=",
    ];
    let mut unknown = 0;
    for operator in crate::domain::operator::constants::OPERATORS {
        let example = crate::domain::operator::constants::describe(operator)
            .example
            .0;
        for quoted in example.split('"').skip(1).step_by(2) {
            let before = example
                .split(&format!("\"{quoted}\""))
                .next()
                .unwrap_or_default()
                .trim_end();
            if fields.iter().any(|field| before.ends_with(field))
                && relation_named(NameRef(quoted)).is_none()
            {
                unknown += 1;
            }
        }
    }
    unknown
}

use ryo_values::domain::rows::types::Table;
