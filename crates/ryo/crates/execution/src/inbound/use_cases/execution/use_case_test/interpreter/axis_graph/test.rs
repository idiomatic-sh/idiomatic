//! The interpreter's own work over the axis requests: one canonical retained graph for the stored edges, one shared walk for the
//! selected ones, none for the hosted ones, and no work on replay.
use super::super::super::super::fixtures;
use super::super::super::super::fixtures::requests::{Authored, Preparation};
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
/// One population's run: each rule's first answer (`None` when its root did not complete), the work it did, and
/// what its replay answered and did.
struct Run {
    answers: Vec<Option<Table>>,
    work: Work,
    replays: Vec<Option<Table>>,
    replay_work: Work,
    foreign_reads: usize,
}
/// The canonical graph's builds, reuses and early stops, and the shared walk's passes, source rows and predicate rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Work {
    graph: [u64; 3],
    walk: [u64; 3],
}
impl Work {
    fn of(
        counts: ryo_execution_store::application::file_dag::derived::owner::fixtures::resources::Snapshot,
        walks: ryo_execution_domain::domain::shared_walk::types::WalkCounts,
    ) -> Self {
        Self {
            graph: [
                counts.graph_builds,
                counts.graph_reuses,
                counts.graph_early_stops,
            ],
            walk: [walks.passes.0, walks.source_rows.0, walks.predicate_rows.0],
        }
    }
}
fn complete(outcome: RootOutcome) -> Option<Table> {
    if let RootOutcome::Complete(table) = outcome {
        Some(table)
    } else {
        None
    }
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
    let request = fixtures::requests::execution().execution(&prepared, &corpus, None);
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
    let work = Work::of(file.resource_fixture_counts(), file.fixture_walk_counts());
    let mut replays = vec![];
    for i in 0..5 {
        let result = file.rule(
            RuleIndex(i),
            crate::inbound::use_cases::execution::fixtures::requests::emitted(),
            &mut symbols,
        )?;
        replays.push(complete(result.emit));
    }
    let replay_work = Work::of(file.resource_fixture_counts(), file.fixture_walk_counts());
    file.finish()?;
    request.finish();
    Ok(Run {
        answers,
        work,
        replays,
        replay_work,
        foreign_reads: foreign_reads.get(),
    })
}
/// The edges an axis request reads: hosted, the stored relation itself, or selected (projected) from it.
#[derive(Clone, Copy, Debug)]
enum Edges {
    Hosted,
    Stored,
    Selected,
}
/// What one population's five rules did and answered.
#[derive(Debug, PartialEq, Eq)]
struct Observed {
    /// The stored relation's cell kinds the fake serves its columns as.
    schema: Cow<'static, [CellKind]>,
    /// The canonical graph's builds, reuses and early stops, and the shared walk's passes, source rows and predicate rows.
    work: Work,
    /// Every root completed, and replayed its exact output.
    replayed: bool,
    /// Delivery retries do no source, predicate or graph work.
    replay_is_free: bool,
    /// All five directions answer the hosted reference's exact ordered bags.
    same_as_hosted: bool,
    /// Reads of any relation other than the stored graph.
    foreign_reads: usize,
}
type Input = Edges;
type Value = Observed;
type Failure = OperatingError;
fn subject(edges: Input) -> Result<Value, Failure> {
    let hosted = execute(false, false)?;
    let run = match edges {
        Edges::Hosted => execute(false, false)?,
        Edges::Stored => execute(true, false)?,
        Edges::Selected => execute(true, true)?,
    };
    Ok(Observed {
        schema: kinds(Relation::LanguageTarget).to_vec().into(),
        work: run.work,
        replayed: run.answers.iter().all(Option::is_some) && run.replays == run.answers,
        replay_is_free: run.replay_work == run.work,
        same_as_hosted: run.answers == hosted.answers,
        foreign_reads: run.foreign_reads,
    })
}
const fn observed(graph: [u64; 3], walk: [u64; 3]) -> Expect<Value, Failure> {
    Expect::Value(Observed {
        schema: Cow::Borrowed(&[CellKind::File, CellKind::Number, CellKind::Number]),
        work: Work { graph, walk },
        replayed: true,
        replay_is_free: true,
        same_as_hosted: true,
        foreign_reads: 0,
    })
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("axis_requests_build_one_graph_or_share_one_walk_and_replay_without_work"),
        word: Word::Succeeds,
        input: || Edges::Stored,
        expect: observed([1, 4, 3], [0, 0, 0]),
    },
    Case {
        name: CaseName("selected_edges_share_one_walk_and_build_no_graph"),
        word: Word::Succeeds,
        input: || Edges::Selected,
        expect: observed([0, 0, 0], [1, 7, 7]),
    },
    Case {
        name: CaseName("hosted_edges_build_no_graph_and_walk_nothing"),
        word: Word::Succeeds,
        input: || Edges::Hosted,
        expect: observed([0, 0, 0], [0, 0, 0]),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
