//! The canonical stored graph, selected stored edges and hosted ordinary traversal publish identical ordered bags.
use super::super::super::fixtures;
use super::super::super::fixtures::requests::{Authored, Preparation};
use crate::inbound::use_cases::execution::interface::CaptureIntent;
use crate::inbound::use_cases::execution::interface::DiagnosticDelivery;
use crate::inbound::use_cases::execution::interface::ExecutionUseCase;
use crate::inbound::use_cases::execution::interface::FileRuleIntent;
use crate::inbound::use_cases::execution::interface::ObservationDemand;
use crate::inbound::use_cases::execution::interface::OperatingError;
use crate::inbound::use_cases::execution::interface::RuleIndex;
use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_execution_runtime::application::file_dag::program::file_session::interface::ProgramFileSessionPort as _;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
use ryo_execution_runtime::application::file_dag::program::request_execution::interface::ProgramRequestExecutionPort as _;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_relations::domain::relations::{
    service::{kinds, to_bits},
    types::{CellKind, ColumnsView, FactColumn, FileId, Integer, Relation, Value as Cell},
};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
        plan_parts::types::{ColumnIndex, Direction, HostedNumber, Input as Source, StepId},
        plans::types::{Operation, Plan},
        query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
        rows::types::Table,
        rule_requests::types::{RuleParameters, RuleSpec},
        runs::types::{RuleApplicability, RuleId},
        stored_facts::types::Digest,
    },
};
use std::borrow::Cow;
const AXES: [&str; 5] = ["axis-0", "axis-1", "axis-2", "axis-3", "axis-4"];
const DIRECTIONS: [Direction; 5] = [
    Direction::Parent,
    Direction::Child,
    Direction::Ancestor,
    Direction::Descendant,
    Direction::NearestAncestor,
];
const EDGES: &[&[i64]] = &[
    &[1, 9],
    &[1, 2],
    &[1, 9],
    &[9, 4],
    &[2, 4],
    &[4, 1],
    &[90, 91],
];
/// The edges of `stored` and not `projected` are the stored relation itself, the population the canonical retained
/// graph is built over; `projected` selects the stored relation with duplicated and reordered columns (owned rows,
/// traversed as ordinary rows); otherwise the edges are hosted.
fn plan(direction: Direction, stored: bool, projected: bool) -> Plan {
    let axis = |edges, domain, parent, child| Operation::Axis {
        from: Source::Step(StepId(0)),
        edges,
        domain: Source::Step(domain),
        direction,
        id: ColumnIndex(0),
        parent: ColumnIndex(parent),
        child: ColumnIndex(child),
        domain_id: ColumnIndex(0),
        domain_filter: vec![],
        domain_view: vec![],
        output: vec![],
    };
    let starts = Operation::Hosted {
        relation: HostedNumber(0),
    };
    let domain = Operation::Hosted {
        relation: HostedNumber(1),
    };
    if stored && !projected {
        return inputs::plan(vec![
            starts,
            domain,
            axis(Source::Relation(Relation::LanguageTarget), StepId(1), 1, 2),
        ]);
    }
    inputs::plan(vec![
        starts,
        if stored {
            Operation::Select {
                from: Source::Relation(Relation::LanguageTarget),
                comparisons: vec![],
                kept: vec![ColumnIndex(2), ColumnIndex(1), ColumnIndex(2)],
            }
        } else {
            Operation::Hosted {
                relation: HostedNumber(2),
            }
        },
        domain,
        axis(
            Source::Step(StepId(1)),
            StepId(2),
            u32::from(projected),
            u32::from(!projected),
        ),
    ])
}
/// One run of the five rules: each rule's first answer (`None` when its root did not complete) and its replay.
struct Run {
    answers: Vec<Option<Table>>,
    replays: Vec<Option<Table>>,
    foreign_reads: usize,
}
fn execute(stored: bool, projected: bool) -> Result<Run, OperatingError> {
    let engine = Preparation::real();
    let plans = DIRECTIONS
        .into_iter()
        .map(|d| plan(d, stored, projected))
        .collect();
    let prepared = engine.prepare(Authored {
        plans: plans,
        bindings: vec![vec![]; 5],
        rules: AXES
            .iter()
            .map(|axis| RuleSpec {
                id: RuleId((*axis).into()),
                query: QueryBundle {
                    root: QueryDocumentSource {
                        id: QuerySourceId((*axis).into()),
                        text: "".into(),
                    },
                    modules: Default::default(),
                },
                applicability: RuleApplicability::Always,
                given: vec![],
                observations: vec![],
                spelled: RuleParameters("".into()),
                include: vec![],
                exclude: vec![],
            })
            .collect(),
        digests: vec![Digest([0; 16]); 5],
        hosted: vec![
            fixtures::requests::numbers("starts", 1, &[&[1], &[1], &[4], &[999]]),
            fixtures::requests::numbers("domain", 1, &[&[1], &[2], &[9], &[9], &[4], &[91]]),
            fixtures::requests::numbers("edges", 2, EDGES),
        ],
        observations: ObservationIdentity::Independent,
    })?;
    let columns = [
        FactColumn(vec![to_bits(Cell::File(FileId(0))).0; 7]),
        FactColumn(
            EDGES
                .iter()
                .map(|row| to_bits(Cell::Number(Integer(row[0]))).0)
                .collect(),
        ),
        FactColumn(
            EDGES
                .iter()
                .map(|row| to_bits(Cell::Number(Integer(row[1]))).0)
                .collect(),
        ),
    ];
    let foreign_reads = std::cell::Cell::new(0);
    let read = |relation| {
        if relation != Relation::LanguageTarget {
            foreign_reads.set(foreign_reads.get() + 1);
        }
        Ok(SourceRows::Stored(ColumnsView {
            kinds: kinds(relation),
            columns: &columns,
        }))
    };
    let corpus = [CorpusInput::Available(CorpusEntry {
        path: ryo_language::domain::files::types::FilePath("axis.py".into()),
        executable: ExecutablePermission::Unknown,
        size: FileSize(0),
        modified: Nanoseconds(0),
    })];
    let request = fixtures::requests::execution().execution(&prepared, &corpus, None.into());
    let mut symbols = Symbols::default();
    let elements = Elements::default();
    let intents = vec![
        FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled
        };
        5
    ];
    let mut file = request.open_file(
        FileInput {
            language: SourceLanguage::Recognized,
            resident_stability: ResidentStability::Stable,
            file: FileId(0),
            evaluation: &[],
            publication: &[],
            intents: &intents,
            source: ryo_values::domain::files::types::SourceView(b""),
            elements: &elements,
            read: &read,
            semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
            kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
        },
        &mut symbols,
    )?;
    let mut answers = vec![];
    for i in 0..5 {
        let result = file.rule(
            RuleIndex(i),
            crate::inbound::use_cases::execution::fixtures::requests::emitted(),
            &mut symbols,
        )?;
        answers.push(complete(result.emit));
    }
    let mut replays = vec![];
    for i in 0..5 {
        let result = file.rule(
            RuleIndex(i),
            crate::inbound::use_cases::execution::fixtures::requests::emitted(),
            &mut symbols,
        )?;
        replays.push(complete(result.emit));
    }
    file.finish()?;
    request.finish();
    Ok(Run {
        answers,
        replays,
        foreign_reads: foreign_reads.get(),
    })
}
fn complete(outcome: RootOutcome) -> Option<Table> {
    if let RootOutcome::Complete(table) = outcome {
        Some(table)
    } else {
        None
    }
}
/// What the three populations answered across the five directions.
#[derive(Debug, PartialEq, Eq)]
struct Parity {
    /// The stored relation's cell kinds the fake serves its columns as.
    schema: Cow<'static, [CellKind]>,
    /// The ordinary run's row count per direction, `None` where a root did not complete.
    ordinary_rows: [Option<usize>; 5],
    /// Whether every root of a run (ordinary, stored, projected) completed and replayed its exact output.
    replayed: [bool; 3],
    /// Duplicate and reordered selected columns retain the same graph meaning.
    projected_is_ordinary: bool,
    /// All five directions preserve exact BFS/direct order and duplicate bags.
    stored_is_ordinary: bool,
    /// Reads of any relation other than the stored graph, across the three runs.
    foreign_reads: usize,
}
type Input = ();
type Value = Parity;
type Failure = OperatingError;
fn subject((): Input) -> Result<Value, Failure> {
    let ordinary = execute(false, false)?;
    let stored = execute(true, false)?;
    let projected = execute(true, true)?;
    let replayed =
        |run: &Run| run.answers.iter().all(Option::is_some) && run.replays == run.answers;
    let mut ordinary_rows = [None; 5];
    for (rows, answer) in ordinary_rows.iter_mut().zip(&ordinary.answers) {
        *rows = answer.as_ref().map(|table| table.count.0);
    }
    Ok(Parity {
        schema: kinds(Relation::LanguageTarget).to_vec().into(),
        ordinary_rows,
        replayed: [replayed(&ordinary), replayed(&stored), replayed(&projected)],
        projected_is_ordinary: projected.answers == ordinary.answers,
        stored_is_ordinary: stored.answers == ordinary.answers,
        foreign_reads: ordinary.foreign_reads + stored.foreign_reads + projected.foreign_reads,
    })
}
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("compact_axes_equal_ordinary_bfs_and_direct_bags_across_rules"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(Parity {
        schema: Cow::Borrowed(&[CellKind::File, CellKind::Number, CellKind::Number]),
        ordinary_rows: [Some(4), Some(7), Some(12), Some(12), Some(3)],
        replayed: [true; 3],
        projected_is_ordinary: true,
        stored_is_ordinary: true,
        foreign_reads: 0,
    }),
}];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
