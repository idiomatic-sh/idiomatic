//! File-generation freshness under one immutable prepared request and service set.
use super::super::super::super::service::*;
use super::super::direct::fixtures::{request, schema_column, simple};
use super::super::*;
use crate::application::file_dag::execution::service::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime::file_execution::service_test::fixtures::*;
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
use crate::cases::{Case, CaseName, Expect, Word, run};
use fixtures::execution::*;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::CaptureIntent;
use ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery;
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
use ryo_execution_domain::domain::table_model::types::RepresentationKind;
use ryo_execution_domain::domain::{
    observation_schedule::types::RuleSchedule,
    prepared_request::types::{Compiled, ObservationIdentity},
};
use ryo_execution_store::application::file_dag::derived::owner::fixtures::resources::RecordedDerived;
use ryo_execution_store::application::file_dag::derived::relation_owner::types::ScopePartials;
use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_relations::domain::relations::types::{CellKind, FileId, Integer, Relation, Value as Cell};
use ryo_values::domain::rows::errors::RowsError;
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{ColumnIndex, Side, StepId},
    stored_facts::types::Digest,
};
use std::borrow::Cow;
use std::cell::Cell as Counter;
use std::{rc::Rc, sync::Arc};
/// Runs one prepared request over three generations of the same file, a generation whose right
/// side is refused, and a malformed key on an empty primary.
fn reopen(keys: &[Generation]) -> Reopened {
    let left = schema_column(Relation::Entry, CellKind::Number, 0);
    let right = schema_column(Relation::SequenceElement, CellKind::Number, 0);
    let payload = schema_column(Relation::SequenceElement, CellKind::Number, 1);
    let recorded = Arc::new(RecordedDerived::default());
    let derived: Arc<dyn ryo_execution_store::outbound::derived::interface::DerivedPort> =
        recorded.clone();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            plans: std::iter::IntoIterator::into_iter(vec![request(
                &[left],
                &[right],
                &[(Side::Right, payload)],
            )])
            .map(std::sync::Arc::new)
            .collect(),
            bindings: vec![vec![]],
            observations: vec![vec![]],
            schedule: RuleSchedule::Independent,
        },
        &[rule()],
        &[Digest([0; 16])],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )
    .unwrap();
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(real_port())),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("same-prepared-file.py".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
    let mut symbols = Symbols::default();
    let mut total_builds = 0;
    let mut generations = Vec::new();
    for &Generation {
        keys,
        values,
        refuse_after,
    } in keys
    {
        let builds_before = recorded.builds().len();
        let providers = simple(keys, values, RepresentationKind::Stored);
        let reads = [Counter::new(0), Counter::new(0)];
        let read = |relation| {
            let side = providers
                .iter()
                .position(|p| p.relation == relation)
                .ok_or(OperatingError::NotRead)?;
            reads[side].set(reads[side].get() + 1);
            Ok(providers[side].rows())
        };
        let mut file = open_file(
            &prepared,
            FileId(0),
            ScopePartials::File {
                evaluation: &[],
                publication: &[],
            },
            &[FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Forwarded,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::Disabled,
            }],
            Arc::clone(&services),
            ryo_values::domain::files::types::SourceView(b""),
            ReuseEligibility::Eligible,
            &mut symbols,
            None,
            None,
        )
        .unwrap();
        let weak = Rc::downgrade(&file.owner);
        let mut prefix = vec![];
        let result = file.emit_rows(RuleIndex(0), &read, &mut symbols, &mut |row| {
            if refuse_after == Some(prefix.len()) {
                return Err(OperatingError::WrongBinding);
            }
            prefix.push(row.to_vec());
            Ok(())
        });
        let work = file.route_observation(RuleIndex(0), StepId(0)).unwrap();
        let counts = file.owner.derived().counts();
        total_builds += counts.0.0;
        let right_reads = reads[1].get();
        let payloads = recorded.builds().len() - builds_before;
        let owner = Rc::clone(&file.owner);
        file.finish().unwrap();
        let bytes = (
            owner.derived().working_bytes().0,
            owner.derived().owned_bytes().0,
        );
        drop(owner);
        generations.push(Opened {
            refused: result.err(),
            rows: Cow::Owned(prefix.concat()),
            index: (work.index_rows, work.index_cells, work.index_capacity),
            acquired: work.index_representation,
            total_builds,
            old_index_hits: counts.1.0,
            right_reads,
            payloads,
            bytes,
            owner_retained: weak.upgrade().is_some(),
        });
    }
    let calls = recorded.builds();
    let equality_on_the_right = calls.iter().all(|request| {
        matches!(request,
        ryo_derived::domain::request::types::DerivedRequest::Equality { columns }
        if columns == &[ColumnIndex(right)])
    });
    let providers = simple(&[], &[(1, 99)], RepresentationKind::Stored);
    let right_refusals = Counter::new(0);
    let read = |relation| {
        if relation == providers[0].relation {
            Ok(providers[0].rows())
        } else {
            right_refusals.set(right_refusals.get() + 1);
            Err(OperatingError::NotRead)
        }
    };
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Forwarded,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }],
        Arc::clone(&services),
        ryo_values::domain::files::types::SourceView(b""),
        ReuseEligibility::Eligible,
        &mut symbols,
        None,
        None,
    )
    .unwrap();
    let mut rows = vec![];
    let refused = file
        .emit_rows(RuleIndex(0), &read, &mut symbols, &mut |row| {
            rows.push(row.to_vec());
            Ok(())
        })
        .err();
    let unsupplied = Unsupplied {
        refused,
        right_refusals: right_refusals.get(),
        rows: rows.len(),
        builds: file.owner.derived().counts().0.0,
        payloads: recorded.builds().len(),
    };
    file.finish().unwrap();
    // The required malformed-key declaration still refuses an empty primary,
    // before any equality payload can be acquired.
    let mut invalid = request(&[left], &[99], &[(Side::Right, payload)]);
    invalid.emit.step = StepId(0);
    let actual = execute(
        invalid,
        simple(&[], &[(1, 99)], RepresentationKind::Stored),
        &mut symbols,
        b"",
        false,
        real_port(),
        Faults::default(),
    )
    .unwrap();
    Reopened {
        generations: generations.try_into().unwrap(),
        payloads: calls.len(),
        equality_on_the_right,
        unsupplied,
        malformed: (
            actual.result.err(),
            actual.derived_builds.len(),
            actual.held,
        ),
    }
}

/// One generation of the file: its right keys, its right (key, payload) rows, and the delivered
/// row after which the sink refuses.
#[derive(Clone, Copy, Debug)]
struct Generation {
    keys: &'static [i64],
    values: &'static [(i64, i64)],
    refuse_after: Option<usize>,
}

/// What one generation answered, after its file finished.
#[derive(Debug, PartialEq)]
struct Opened {
    /// The emit's refusal.
    refused: Option<OperatingError>,
    /// The rows delivered, one cell each.
    rows: Cow<'static, [Cell]>,
    /// The Join index's rows, cells and capacity.
    index: (usize, usize, usize),
    /// The Join index's representation.
    acquired: Option<RepresentationKind>,
    /// Constructions so far under the one request and service set.
    total_builds: u64,
    /// Hits on an earlier population's index.
    old_index_hits: u64,
    /// Reads of the right side.
    right_reads: usize,
    /// Derived payloads built in this generation.
    payloads: usize,
    /// The owner's working and owned bytes after finish.
    bytes: (usize, usize),
    /// Whether the finished generation's execution owner is still alive.
    owner_retained: bool,
}

/// A generation whose right side is refused.
#[derive(Debug, PartialEq)]
struct Unsupplied {
    refused: Option<OperatingError>,
    right_refusals: usize,
    rows: usize,
    builds: u64,
    /// Derived payloads built over every generation.
    payloads: usize,
}

/// The whole reopened file.
#[derive(Debug, PartialEq)]
struct Reopened {
    generations: [Opened; 3],
    /// Derived payloads built over the three generations.
    payloads: usize,
    /// Whether every payload is an equality index on the right key.
    equality_on_the_right: bool,
    unsupplied: Unsupplied,
    /// The malformed key's refusal on an empty primary, the derived builds it made, and what its graph owner held
    /// once the file finished.
    malformed: (Option<OperatingError>, usize, Held),
}

/// The three generations: a delivery refused after one row, an empty primary, a changed payload.
const GENERATIONS: &[Generation] = &[
    Generation {
        keys: &[1],
        values: &[(1, 10), (1, 11)],
        refuse_after: Some(1),
    },
    Generation {
        keys: &[],
        values: &[(1, 99)],
        refuse_after: None,
    },
    Generation {
        keys: &[1],
        values: &[(1, 99)],
        refuse_after: None,
    },
];

type Input = &'static [Generation];
type Value = Reopened;
type Failure = String;

/// A finished generation that left no bytes, no old-index hits and no retained owner.
const fn opened(
    refused: Option<OperatingError>,
    rows: &'static [Cell],
    index_rows: usize,
    acquired: Option<RepresentationKind>,
    (total_builds, right_reads, payloads): (u64, usize, usize),
) -> Opened {
    Opened {
        refused,
        rows: Cow::Borrowed(rows),
        index: (index_rows, 0, 0),
        acquired,
        total_builds,
        old_index_hits: 0,
        right_reads,
        payloads,
        bytes: (0, 0),
        owner_retained: false,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName(
        "one_prepared_request_reopens_same_file_after_delivery_failure_empty_skip_and_changed_payload",
    ),
    word: Word::Succeeds,
    input: || GENERATIONS,
    expect: Expect::Value(Reopened {
        generations: [
            opened(
                Some(OperatingError::WrongBinding),
                &[Cell::Number(Integer(10))],
                2,
                Some(RepresentationKind::Stored),
                (1, 3, 1),
            ),
            opened(None, &[], 0, None, (1, 1, 0)),
            opened(
                None,
                &[Cell::Number(Integer(99))],
                1,
                Some(RepresentationKind::Stored),
                (2, 3, 1),
            ),
        ],
        payloads: 2,
        equality_on_the_right: true,
        unsupplied: Unsupplied {
            refused: Some(OperatingError::NotRead),
            right_refusals: 1,
            rows: 0,
            builds: 0,
            payloads: 2,
        },
        malformed: (
            Some(OperatingError::Table(RowsError::NoSuchColumn { index: 99 })),
            0,
            NOTHING_HELD,
        ),
    }),
}];

/// Reopens the file for each generation.
fn subject(generations: Input) -> Result<Value, Failure> {
    Ok(reopen(generations))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
