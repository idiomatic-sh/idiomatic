//! The `fragment` service's cases, over the small tree: the lowered steps of a plan are made and their rows passed on.

use crate::domain::lowering::types::Counting;
use crate::domain::table_texts::types::SourceView;
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::{Relation, Value as Cell};
use ryo_values::domain::plan_parts::types::LiteralText;

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fragment::service::{counts, emit, emit_counts, make};
use crate::domain::fragment::types::FragmentRun;
use crate::domain::lowering::service::lower;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::positions::service_test::{SMALL_SOURCE, SmallTree, rows_over, small_tree};
use crate::domain::positions::types::{LoweredRoom, Positions, Readiness};
use crate::domain::replay::types::Collect;

use crate::domain::table_scratch::types::OperatorScratch;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Comparison;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::QueryLiteral;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep, StepScope};

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Passes on the rows of a select of the tree.
    Words,
    /// Counts the rows of a select of the tree by their kind.
    Counted,
}

/// What a row of the cases does.
type Input = Scenario;
/// The cells passed on, shown.
type Value = &'static str;
type Failure = OperatingError;

fn plan_of(steps: Vec<Operation>) -> Plan {
    let last = steps.len().saturating_sub(1);
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: Vec::new(),
        steps: steps
            .into_iter()
            .map(|operation| PlanStep {
                operation,
                scope: StepScope::PerFile,
            })
            .collect(),
        emit: Emit {
            step: StepId(u32::try_from(last).unwrap_or(0)),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: Vec::new(),
        comparison_origins: Vec::new(),
    }
}

fn words() -> Operation {
    Operation::Select {
        from: PlanInput::Relation(Relation::SyntaxTree),
        comparisons: vec![Comparison {
            test: Test::Equal,
            left: Operand::Column(ColumnIndex(1)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText("word".into()))),
            origin: None,
        }],
        kept: vec![ColumnIndex(0), ColumnIndex(1)],
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let SmallTree {
        elements,
        mut symbols,
    } = small_tree();
    let mut steps = vec![words()];
    if matches!(scenario, Scenario::Counted) {
        steps.push(Operation::GroupCount {
            from: PlanInput::Step(StepId(0)),
            keys: vec![ColumnIndex(1)],
            distinct: None,
        });
    }
    let plan = plan_of(steps);
    let lowering = lower(&plan);
    let mut room = LoweredRoom {
        ready: vec![Readiness::NotMade; plan.steps.len()],
        made: vec![Positions::default(); plan.steps.len()],
        ..LoweredRoom::default()
    };
    let mut running = FragmentRun {
        plan: &plan,
        lowering: &lowering,
        tree: rows_over(&elements),
        room: &mut room,
        classes: &[],
    };
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(SMALL_SOURCE),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let semantics = crate::application::file_dag::execution::PrimitiveSemanticsBridge {
        port: crate::outbound::operators::factory::test_port(),
    };
    let mut table = Table::default();
    let last = StepId(u32::try_from(plan.steps.len() - 1).unwrap_or(0));
    if counts(&running, last) == Counting::Counts {
        emit_counts(
            &semantics,
            &mut running,
            last,
            &mut context,
            &mut Collect(&mut table),
        )?;
    } else {
        make(&semantics, &mut running, last, &mut context)?;
        emit(&mut running, last, &mut context, &mut Collect(&mut table))?;
    }
    let width = table.kinds.len().max(1);
    // The answer is leaked: a case compares it with text that lives for the whole run.
    Ok(Box::leak(
        table
            .cells
            .chunks(width)
            .map(|row| {
                row.iter()
                    .map(|cell| match cell {
                        Cell::Node(node) => node.0.to_string(),
                        Cell::Number(Integer(number)) => number.to_string(),
                        Cell::Symbol(_)
                        | Cell::Name(_)
                        | Cell::File(_)
                        | Cell::Span(_)
                        | Cell::Flag(_) => "_".to_owned(),
                    })
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .collect::<Vec<_>>()
            .join(" ")
            .into_boxed_str(),
    ))
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_rows_of_a_lowered_select_are_passed_on_as_values"),
        word: Word::Succeeds,
        input: || Scenario::Words,
        expect: Expect::Value("1 _ 3 _"),
    },
    Case {
        name: CaseName("a_count_of_lowered_rows_is_passed_on_with_the_keys"),
        word: Word::Succeeds,
        input: || Scenario::Counted,
        expect: Expect::Value("_ 2"),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

use ryo_values::domain::rows::types::Table;
