//! Canonical stored graph acquisition and hosted ordinary traversal publish identical ordered bags.
use super::super::super::fixtures::{Authored, Preparation};
use crate::outbound::facts::types::ResidentStability;
use crate::inbound::use_cases::execution::use_case::ExecutionUseCaseImpl;
use crate::inbound::use_cases::execution::interface::ExecutionUseCase;
use super::super::super::fixtures;
use crate::application::file_dag::program::file_session::types::FileInput;
use crate::application::file_dag::program::file_session::types::SourceLanguage;
use crate::application::file_dag::program::file_session::types::SourceRows;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::inbound::use_cases::execution::interface::CaptureIntent;
use crate::inbound::use_cases::execution::interface::DiagnosticDelivery;
use crate::inbound::use_cases::execution::interface::EmitDemand;
use crate::inbound::use_cases::execution::interface::FileRuleIntent;
use crate::inbound::use_cases::execution::interface::ObservationDemand;
use crate::inbound::use_cases::execution::interface::OperatingError;
use crate::inbound::use_cases::execution::interface::RuleDemand;
use crate::inbound::use_cases::execution::interface::RuleIndex;
use crate::domain::prepared_request::types::ObservationIdentity;
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
use std::sync::Arc;
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
fn plan(direction: Direction, stored: bool, projected: bool) -> Plan {
    fixtures::plan(vec![
        Operation::Hosted {
            relation: HostedNumber(0),
        },
        if stored {
            Operation::Select {
                from: Source::Relation(Relation::LanguageTarget),
                comparisons: vec![],
                kept: if projected {
                    vec![ColumnIndex(2), ColumnIndex(1), ColumnIndex(2)]
                } else {
                    vec![ColumnIndex(1), ColumnIndex(2)]
                },
            }
        } else {
            Operation::Hosted {
                relation: HostedNumber(2),
            }
        },
        Operation::Hosted {
            relation: HostedNumber(1),
        },
        Operation::Axis {
            from: Source::Step(StepId(0)),
            edges: Source::Step(StepId(1)),
            domain: Source::Step(StepId(2)),
            direction,
            id: ColumnIndex(0),
            parent: ColumnIndex(u32::from(projected)),
            child: ColumnIndex(u32::from(!projected)),
            domain_id: ColumnIndex(0),
            domain_filter: vec![],
            domain_view: vec![],
            output: vec![],
        },
    ])
}
fn execute(stored: bool, projected: bool) -> Result<Vec<Table>, OperatingError> {
    let engine = Preparation::new(
        Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())),
        ),
        Arc::new(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new()),
    );
    let plans = DIRECTIONS
        .into_iter()
        .map(|d| plan(d, stored, projected))
        .collect();
    let prepared = engine.prepare(Authored { plans: plans, bindings: vec![vec![]; 5], rules: AXES
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
            .collect(), digests: vec![Digest([0; 16]); 5], hosted: vec![
            fixtures::numbers("starts", 1, &[&[1], &[1], &[4], &[999]]),
            fixtures::numbers("domain", 1, &[&[1], &[2], &[9], &[9], &[4], &[91]]),
            fixtures::numbers("edges", 2, EDGES),
        ], observations: ObservationIdentity::Independent })?;
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
    assert_eq!(
        kinds(Relation::LanguageTarget),
        [CellKind::File, CellKind::Number, CellKind::Number]
    );
    let read = |relation| {
        assert_eq!(relation, Relation::LanguageTarget);
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
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus);
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
            source: b"",
            elements: &elements,
            read: &read,
            semantic: None,
            kinds: None,
        },
        &mut symbols,
    )?;
    let mut answers = vec![];
    for i in 0..5 {
        let result = file.rule(
            RuleIndex(i),
            RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            &mut symbols,
        )?;
        let RootOutcome::Complete(table) = result.emit else {
            panic!("actual axis owner must complete")
        };
        answers.push(table);
    }
    let counts = file.inner.resource_fixture_counts();
    if stored {
        assert_eq!(counts.graph_builds, 1);
        assert_eq!(counts.graph_reuses, 4);
        assert!(
            counts.graph_early_stops > 0,
            "cyclic nearest preserves early stop"
        );
    } else {
        assert_eq!(
            counts.graph_builds, 0,
            "hosted ordinary reference has no canonical retained graph"
        );
    }
    let walks = file.walk_counts();
    if stored {
        assert_eq!(
            walks.passes.0, 1,
            "all five stored consumers share one actual walk"
        );
        assert_eq!(
            walks.source_rows.0, 7,
            "the seven-edge provider population is scanned once"
        );
        assert_eq!(
            walks.predicate_rows.0, 7,
            "one shared predicate class visits every source row once"
        );
    } else {
        assert_eq!(
            walks.passes.0, 0,
            "the hosted reference performs no resident shared walk"
        );
    }
    for (i, expected) in answers.iter().enumerate() {
        let result = file.rule(
            RuleIndex(i),
            RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            &mut symbols,
        )?;
        let RootOutcome::Complete(table) = result.emit else {
            panic!("a completed axis root must replay its exact output")
        };
        assert_eq!(&table, expected);
    }
    assert_eq!(
        file.walk_counts(),
        walks,
        "delivery retries do no source or predicate work"
    );
    let repeated = file.inner.resource_fixture_counts();
    assert_eq!(repeated.graph_builds, counts.graph_builds);
    assert_eq!(repeated.graph_reuses, counts.graph_reuses);
    assert_eq!(repeated.graph_early_stops, counts.graph_early_stops);
    file.finish()?;
    request.finish();
    Ok(answers)
}
type Input = ();
type Value = ();
type Failure = OperatingError;
fn subject((): Input) -> Result<Value, Failure> {
    let ordinary = execute(false, false)?;
    let stored = execute(true, false)?;
    let projected = execute(true, true)?;
    assert_eq!(
        projected, ordinary,
        "duplicate/reordered selected columns retain the same graph meaning"
    );
    assert_eq!(
        stored, ordinary,
        "all five directions preserve exact BFS/direct order and duplicate bags"
    );
    Ok(())
}
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("compact_axes_equal_ordinary_bfs_and_direct_bags_across_rules"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
