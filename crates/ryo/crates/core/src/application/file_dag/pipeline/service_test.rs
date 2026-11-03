//! The `pipeline` service's cases: it walks a plan through the operators, the per-file steps on one
//! file's rows and the cross-file steps on the rows the files handed over.

use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::relations::service::to_bits;
use ryo_language::domain::relations::types::{
    CellKind, FileId, Name, Offset, PlaceNumber, Relation, TextHash, Value as Cell,
};
use ryo_language::domain::symbols::service::intern;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::RepeatedRows;

use super::service::evaluate_file;
use crate::application::file_dag::cross_execution::evaluate_cross;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::coverage::types::Partials;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::service::{emitted, pool_for};

use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::{Frontier, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use ryo_language::domain::relations::types::{ColumnsView, FactColumn};
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
    /// A plan whose steps are all per-file, on a file that has the relation it reads.
    PerFile,
    /// The same plan, on a file without the relation.
    NotRead,
    /// A plan with a cross-file step: the file hands over its rows, and two files' rows meet.
    CrossFile,
    /// One pool made for two plans serves each in turn, and the first again.
    OnePoolForSeveralPlans,
}

/// What a row of the cases does.
type Input = Scenario;
/// How many rows the plan answers.
type Value = usize;
type Failure = OperatingError;

pub(crate) const SOURCE: &[u8] = b"mainother";

fn column(index: u32) -> ColumnIndex {
    ColumnIndex(index)
}

/// What the columns of the functions hold.
pub(crate) const KINDS: &[CellKind] = &[CellKind::Symbol, CellKind::File];

/// A file's functions as the store keeps them: two named `main` and one `other`, each in a file of
/// its own number.
pub(crate) fn functions(symbols: &mut Symbols) -> Vec<FactColumn> {
    let main = intern(symbols, ByteSlice(SOURCE), Offset(0), Offset(4));
    let other = intern(symbols, ByteSlice(SOURCE), Offset(4), Offset(9));
    let names = [main, main, other]
        .map(|name| to_bits(Cell::Symbol(name)).0)
        .to_vec();
    let files = [10, 11, 11]
        .map(|file| to_bits(Cell::File(FileId(file))).0)
        .to_vec();
    vec![FactColumn(names), FactColumn(files)]
}

/// A plan that keeps the rows whose first column is `main`, then projects the second column.
pub(crate) fn plan() -> Plan {
    let select = PlanStep {
        operation: Operation::Select {
            from: PlanInput::Relation(Relation::Function),
            comparisons: vec![Comparison {
                test: Test::Equal,
                left: Operand::Column(column(0)),
                right: Operand::Literal(QueryLiteral::Text(LiteralText("main".into()))),
                origin: None,
            }],
            kept: Vec::new(),
        },
        scope: StepScope::PerFile,
    };
    let project = PlanStep {
        operation: Operation::Project {
            from: PlanInput::Step(StepId(0)),
            columns: vec![column(1)],
            distinct: RepeatedRows::Dropped,
        },
        scope: StepScope::PerFile,
    };
    Plan {
        parameters: Vec::new(),
        steps: vec![select, project],
        emit: Emit {
            step: StepId(1),
            subject: column(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        frontier: Vec::new(),
        comparison_origins: Vec::new(),
    }
}

/// A plan that counts the names the per-file select keeps, across files: the count groups by a name,
/// which says nothing of the file, so it is a cross-file step and the select's rows are its frontier.
pub(crate) fn counting_plan() -> Plan {
    let mut plan = plan();
    plan.steps.truncate(1);
    plan.steps.push(PlanStep {
        operation: Operation::GroupCount {
            from: PlanInput::Step(StepId(0)),
            keys: vec![column(0)],
            distinct: None,
        },
        scope: StepScope::CrossFile,
    });
    plan.emit.step = StepId(1);
    plan.frontier = vec![PlanInput::Step(StepId(0))];
    plan
}

/// A name that left its file, known by its hash.
fn named(hash: u64) -> Cell {
    Cell::Name(Name {
        hash: TextHash(hash),
        place: PlaceNumber(0),
    })
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut symbols = Symbols::default();
    let columns = (!matches!(scenario, Scenario::NotRead)).then(|| functions(&mut symbols));
    let relations = |relation: Relation| {
        columns
            .as_ref()
            .filter(|_| relation == Relation::Function)
            .map(|columns| ColumnsView {
                kinds: KINDS,
                columns,
            })
    };
    let (mut memo, mut scratch) = (Vec::new(), OperatorScratch::default());
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(SOURCE),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let mut prepared = crate::domain::operator::types::PreparedPool::default();
    match scenario {
        Scenario::PerFile | Scenario::NotRead => {
            let (plan, mut pool) = (plan(), pool_for(std::slice::from_ref(&plan())));
            evaluate_file(
                &plan,
                &relations,
                None,
                &mut context,
                &mut pool,
                &mut prepared,
                &[],
            )?;
            Ok(emitted(&plan, &mut pool).map_or(0, |table| table.count.0))
        }
        Scenario::OnePoolForSeveralPlans => {
            let (per_file, counting) = (plan(), counting_plan());
            let mut pool = pool_for(&[counting.clone(), per_file.clone()]);
            let mut answers = Vec::new();
            for each in [&per_file, &counting, &per_file] {
                evaluate_file(
                    each,
                    &relations,
                    None,
                    &mut context,
                    &mut pool,
                    &mut prepared,
                    &[],
                )?;
                answers.push(emitted(each, &mut pool).map_or(0, |table| table.count.0));
            }
            Ok(usize::from(
                pool.steps.len() == counting.steps.len().max(per_file.steps.len())
                    && answers == [2, 0, 2]
                    && pool.frontier.first().is_some_and(|table| table.count.0 > 0),
            ))
        }
        Scenario::CrossFile => {
            let counting = counting_plan();
            let mut pool = pool_for(std::slice::from_ref(&counting));
            evaluate_file(
                &counting,
                &relations,
                None,
                &mut context,
                &mut pool,
                &mut prepared,
                &[],
            )?;
            let kept = pool.frontier.first().map_or(0, |table| table.count.0);
            // Two files hand over the name `main`, and a third another name.
            let mut table = Table {
                kinds: vec![CellKind::Symbol],
                ..Table::default()
            };
            for hash in [1, 1, 2] {
                push_row(&mut table, &[named(hash)]);
            }
            let mut cross_symbols = Symbols::default();
            let mut cross = OperatorContext {
                prepared_kinds: None,
                prepared_matchers: None,
                bindings: &[],
                symbols: (&mut cross_symbols).into(),
                source: SourceView(&[]),
                paths: &[],
                arenas: &[],
                memo: &mut Vec::new(),
                scratch: &mut OperatorScratch::default(),
            };
            let counted = evaluate_cross(
                &counting,
                vec![Frontier {
                    input: PlanInput::Step(StepId(0)),
                    table,
                }],
                &Partials::default(),
                &mut cross,
                &mut [],
                &[],
            )?;
            if emitted(&counting, &mut pool).is_some() {
                return Err(OperatingError::NotRun);
            }
            Ok(kept * 10 + counted.count.0)
        }
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_plan_runs_its_per_file_steps_in_order"),
        word: Word::Succeeds,
        input: || Scenario::PerFile,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_cross_file_step_reads_only_what_the_file_handed_over"),
        word: Word::Succeeds,
        input: || Scenario::CrossFile,
        expect: Expect::Value(22),
    },
    Case {
        name: CaseName("one_pool_made_for_several_plans_serves_each_in_turn"),
        word: Word::Succeeds,
        input: || Scenario::OnePoolForSeveralPlans,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_relation_nobody_read_is_refused"),
        word: Word::Refuses,
        input: || Scenario::NotRead,
        expect: Expect::Error(OperatingError::NotRead),
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
