//! One request of two rules whose second has a step beyond its operator contract answers the second rule's error through the
//! public file owner while the first rule delivers its rows: preparation succeeds, and the error is the second rule's alone.
use super::super::super::{fixtures, interface::*, types::*};
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, FileId, Flag, Integer, Offset, Position, Relation, Span, Value as CellValue},
};
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{ColumnIndex, Input as PlanInput, RepeatedRows, StepId},
    plans::types::Operation,
    rows::types::{RowCount, Table},
};

type Input = fn();
type Value = ();
type Failure = OperatingError;

fn sources() -> Table {
    let kinds = kinds(Relation::Function).to_vec();
    let mut cells = Vec::new();
    for row in 1..=2 {
        cells.extend(kinds.iter().map(|kind| match kind {
            CellKind::Node => CellValue::Node(ryo_relations::domain::relations::service::node_id(
                FileId(0),
                Position(row),
            )),
            CellKind::Symbol => CellValue::Symbol(Symbol(0)),
            CellKind::File => CellValue::File(FileId(0)),
            CellKind::Span => CellValue::Span(Span {
                start: Offset(0),
                end: Offset(0),
            }),
            CellKind::Number => CellValue::Number(Integer(0)),
            CellKind::Flag => CellValue::Flag(Flag::Lowered),
        }));
    }
    Table {
        kinds,
        cells,
        count: RowCount(2),
    }
}

fn an_errored_rule_answers_its_error_while_its_sibling_delivers_rows() {
    let select = || Operation::Select {
        from: PlanInput::Relation(Relation::Function),
        comparisons: vec![],
        kept: vec![ColumnIndex(0)],
    };
    let sound = fixtures::plan(vec![select()]);
    // The step after the emit step names a column that is not there: the rule's origin is errored although its emit step is sound.
    let mut errored = fixtures::plan(vec![
        select(),
        Operation::Project {
            from: PlanInput::Step(StepId(0)),
            columns: vec![ColumnIndex(99)],
            distinct: RepeatedRows::Kept,
        },
    ]);
    errored.emit.step = StepId(0);
    let (engine, prepared) = fixtures::prepared_many(vec![sound, errored], vec![vec![], vec![]], vec![])
        .expect("an errored rule does not refuse the request: it is refused on its own roots");
    let population = sources();
    let read = |relation| match relation {
        Relation::Function => Ok(SourceRows::Computed(&population)),
        _ => Err(OperatingError::NotRead),
    };
    let corpus = [CorpusInput::Available(CorpusEntry {
        path: ryo_language::domain::files::types::FilePath("errored-rule.py".into()),
        executable: ExecutablePermission::Unknown,
        size: FileSize(0),
        modified: Nanoseconds(0),
    })];
    let request = engine.begin(&prepared, &corpus);
    let elements = Elements::default();
    let mut symbols = Symbols::default();
    let run = FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    };
    let intents = [run, run];
    let mut file = request
        .open_file(
            FileInput {
                language: SourceLanguage::Recognized,
                resident_stability: facts::ResidentStability::Stable,
                file: FileId(0),
                evaluation: &[],
                publication: &[],
                intents: &intents,
                source: b"",
                elements: &elements,
                read: &read,
                semantic: None,
                kinds: None,
            },
            &mut symbols,
        )
        .expect("the file owner opens over both rules");
    let demand = || RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::NotRequired,
    };
    let sibling = file
        .rule(RuleIndex(0), demand(), &mut symbols)
        .expect("the sound rule answers");
    match sibling.emit {
        RootOutcome::Complete(rows) => assert_eq!(rows.count, RowCount(2), "the sound rule's rows"),
        _ => panic!("the sound rule delivers its rows beside an errored sibling"),
    }
    let refused = file.rule(RuleIndex(1), demand(), &mut symbols);
    let delivered = match refused {
        Err(error) => Some(error),
        Ok(answer) => match answer.emit {
            RootOutcome::Failed(error) => Some(error),
            _ => None,
        },
    };
    assert!(
        delivered.is_some(),
        "the errored rule reports its error and never delivers rows"
    );
    file.finish().expect("the file owner finishes");
    request.finish();
}
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("an_errored_rule_answers_its_error_while_its_sibling_delivers_rows"),
    word: Word::Succeeds,
    input: || an_errored_rule_answers_its_error_while_its_sibling_delivers_rows,
    expect: Expect::Value(()),
}];
fn subject(input: Input) -> Result<Value, Failure> {
    input();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
