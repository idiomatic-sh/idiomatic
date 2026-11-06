//! Independent descriptor derivation cases; no relation rows or symbol table is available.

use crate::domain::stored_selection::fixtures::plan;
use crate::domain::stored_selection::types::program::value::SelectionProgram;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, Input as Source, Operand, ParameterId, RepeatedRows, StepId, Test,
    WholeNumber,
};
use ryo_values::domain::plans::types::{Operation, PlanStep};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::query_bindings::types::ParameterBinding;

#[test]
fn exact_numeric_comparison_retains_ordinary_execution_until_selector_parity_is_proven() {
    for test in [Test::NumericGreaterThanOrEqual, Test::IntegerSuccessor] {
        let mut numeric = plan();
        let Operation::Select { comparisons, .. } = &mut numeric.steps[0].operation else {
            panic!("fixture starts with a select");
        };
        comparisons[0].test = test;
        let prepared = SelectionProgram::new(&numeric, &[]).prepare();
        assert!(prepared.steps.iter().all(Option::is_none));
    }
}

/// One statically derivable descriptor or required ordinary-execution boundary.
#[derive(Clone, Copy)]
enum Scenario {
    /// Chain projections and bound operands compose onto physical columns.
    Remapped,
    /// A terminal distinct projection derives, then prevents later descriptor composition.
    Distinct,
    /// Regex evaluation stays with the owning runtime comparison.
    Regex,
    /// Missing bindings retain the normal failure path.
    MissingBinding,
    /// Non-scalar table bindings are outside pure predicate admission.
    Tables,
    /// An empty projection is distinct from identity projection.
    ZeroColumns,
}
/// The derivation boundary to inspect.
type Input = Scenario;
/// One when the prepared metadata satisfies its contract.
type Value = usize;
/// Descriptor preparation refuses eligibility rather than producing an error.
type Failure = ();

/// Derive without any provider rows and check the physical map and referenced bindings.
fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut plan = plan();
    plan.steps[1].operation = Operation::Project {
        from: Source::Step(StepId(0)),
        columns: vec![ColumnIndex(1), ColumnIndex(1)],
        distinct: if matches!(scenario, Scenario::Distinct) {
            RepeatedRows::Dropped
        } else {
            RepeatedRows::Kept
        },
    };
    if matches!(scenario, Scenario::ZeroColumns) {
        plan.steps[1].operation = Operation::Project {
            from: Source::Step(StepId(0)),
            columns: Vec::new(),
            distinct: RepeatedRows::Kept,
        };
    } else {
        plan.steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: Source::Step(StepId(1)),
                kept: vec![ColumnIndex(1), ColumnIndex(0)],
                comparisons: vec![Comparison {
                    test: if matches!(scenario, Scenario::Regex) {
                        Test::Matches
                    } else {
                        Test::Equal
                    },
                    left: Operand::Column(ColumnIndex(1)),
                    right: Operand::Parameter(ParameterId(1)),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
            },
        });
    }
    let mut bindings = vec![
        ParameterBinding::Number(WholeNumber(999)),
        ParameterBinding::Number(WholeNumber(17)),
    ];
    if matches!(scenario, Scenario::MissingBinding) {
        bindings.clear();
    }
    if matches!(scenario, Scenario::Tables) {
        bindings[1] = ParameterBinding::Tables(Vec::new());
    }
    let compiled = SelectionProgram::new(&plan, &bindings).prepare();
    assert!(compiled.steps[0].is_some());
    if matches!(scenario, Scenario::Distinct) {
        let terminal = compiled.steps[1]
            .as_ref()
            .expect("terminal distinct selector");
        assert!(terminal.distinct);
        assert_eq!(terminal.columns, [ColumnIndex(1), ColumnIndex(1)]);
        let mut projected = plan.clone();
        projected.steps.last_mut().unwrap().operation = Operation::Project {
            from: Source::Step(StepId(1)),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Kept,
        };
        assert!(
            SelectionProgram::new(&projected, &bindings)
                .prepare()
                .steps
                .last()
                .unwrap()
                .is_none()
        );
        projected.steps.truncate(2);
        projected.steps[1].operation = Operation::Project {
            from: Source::Step(StepId(0)),
            columns: Vec::new(),
            distinct: RepeatedRows::Dropped,
        };
        let zero = SelectionProgram::new(&projected, &bindings).prepare();
        let zero = zero.steps[1].as_ref().unwrap();
        assert!(zero.distinct && zero.columns.is_empty());
        projected.steps[1].operation = Operation::Project {
            from: Source::Step(StepId(0)),
            columns: vec![ColumnIndex(u32::MAX)],
            distinct: RepeatedRows::Dropped,
        };
        assert!(SelectionProgram::new(&projected, &bindings).prepare().steps[1].is_none());
    }
    if matches!(
        scenario,
        Scenario::Distinct | Scenario::Regex | Scenario::MissingBinding | Scenario::Tables
    ) {
        assert!(compiled.steps.last().unwrap().is_none());
    } else {
        let derived = compiled.steps.last().unwrap().as_ref().unwrap();
        if matches!(scenario, Scenario::ZeroColumns) {
            assert!(derived.columns.is_empty());
        } else {
            assert_eq!(derived.columns, [ColumnIndex(1), ColumnIndex(1)]);
            assert_eq!(
                derived.identity.comparisons[1].left,
                Operand::Column(ColumnIndex(1))
            );
            assert_eq!(
                derived.identity.comparisons[1].right,
                Operand::Parameter(ParameterId(0))
            );
            assert_eq!(
                derived.identity.bindings,
                [ParameterBinding::Number(WholeNumber(17))]
            );
        }
    }
    Ok(1)
}

/// Static composition and owner-local fallback cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("select_chains_compose_duplicate_column_maps_and_referenced_bindings"),
        word: Word::Succeeds,
        input: || Scenario::Remapped,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("terminal_distinct_derives_without_flattening_descendants"),
        word: Word::Succeeds,
        input: || Scenario::Distinct,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("regex_retains_owner_local_execution_and_error_origin"),
        word: Word::Succeeds,
        input: || Scenario::Regex,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("missing_binding_retains_normal_failure_path"),
        word: Word::Succeeds,
        input: || Scenario::MissingBinding,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("unsupported_binding_retains_normal_failure_path"),
        word: Word::Succeeds,
        input: || Scenario::Tables,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("zero_column_projection_is_not_identity_projection"),
        word: Word::Succeeds,
        input: || Scenario::ZeroColumns,
        expect: Expect::Value(1),
    },
];

/// Execute every registered metadata derivation case.
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
