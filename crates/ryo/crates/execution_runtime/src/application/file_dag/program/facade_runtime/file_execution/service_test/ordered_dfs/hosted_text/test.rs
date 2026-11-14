//! Hosted text vertices and one file owner reset the traversal across closures and graphs.
use super::super::super::super::service::*;
use super::super::super::fixtures::*;
use super::super::fixtures::{column, execute_request, host, hosted_plan, numbers, operation};
use crate::application::file_dag::execution::service::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::fixtures::ExecutionObservation;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::CaptureIntent;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::EmitDemand;
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
#[cfg(test)]
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_domain::domain::{
    observation_schedule::types::RuleSchedule,
    prepared_request::types::{Compiled, ObservationIdentity},
};
use ryo_execution_store::application::file_dag::derived::relation_owner::types::ScopePartials;
use ryo_execution_store::application::file_dag::relations::types::ReuseEligibility;
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_relations::domain::relations::types::{CellKind, FileId, Integer, Value as Cell};
use ryo_values::domain::hosted::types::{
    HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow,
};
use ryo_values::domain::row_texts::types::Cell as HostCell;
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{Input as PlanInput, StepId},
    plans::types::Operation,
    runs::types::RuleId,
    stored_facts::types::Digest,
};
use std::borrow::Cow;
use std::{rc::Rc, sync::Arc};
/// The hosted graph a row walks.
#[derive(Clone, Copy, Debug)]
enum Graph {
    /// Text roots `(vertex, context)` and text edges `(from, to, order)` walked by partition key.
    Text {
        roots: &'static [[&'static str; 2]],
        edges: &'static [(&'static str, &'static str, i64)],
    },
    /// One file owner running four rules in turn: a walk of 1 -> 2 -> 3, the same edges as a
    /// closure, an empty graph, and a different graph 9 -> 8.
    SharedOwner,
}

/// What the walk observed.
#[derive(Debug, PartialEq)]
enum Observed {
    /// The text walk: its rows as their five texts joined by spaces and the ordinal, and the
    /// traversal's expanded and examined counts.
    Texts {
        count: usize,
        kinds: Cow<'static, [CellKind]>,
        rows: Cow<'static, [(Cow<'static, str>, i64)]>,
        expanded: usize,
        examined: usize,
        /// The owner's working bytes once the file finished.
        retained: usize,
    },
    /// The shared owner: each rule's row count, the first and different graphs' sorted rows,
    /// whether the first walk held an adjacency, the empty rule's adjacency, frame and visited
    /// lengths, and the owner's working bytes once the file finished.
    Owner {
        counts: Cow<'static, [usize]>,
        first: Cow<'static, [Cow<'static, [i64]>]>,
        different: Cow<'static, [Cow<'static, [i64]>]>,
        first_adjacency_held: bool,
        empty_state: [usize; 3],
        working_bytes: usize,
    },
}

type Input = Graph;
type Value = Observed;
type Failure = OperatingError;

const fn text_row(texts: &'static str, ordinal: i64) -> (Cow<'static, str>, i64) {
    (Cow::Borrowed(texts), ordinal)
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("text_vertices_and_partition_keys_use_the_actual_hosted_text_owner"),
        word: Word::Succeeds,
        input: || Graph::Text {
            roots: &[["a", "ctx"], ["a", "ctx"], ["b", "ctx"]],
            edges: &[("a", "b", 0), ("b", "c", 0), ("b", "a", 1)],
        },
        expect: Expect::Value(Observed::Texts {
            count: 6,
            kinds: Cow::Borrowed(&[
                CellKind::Symbol,
                CellKind::Symbol,
                CellKind::Symbol,
                CellKind::Symbol,
                CellKind::Symbol,
                CellKind::Number,
            ]),
            rows: Cow::Borrowed(&[
                text_row("ctx a a b edge", 0),
                text_row("ctx a b c edge", 1),
                text_row("ctx a b a edge", 2),
                text_row("ctx b b c edge", 0),
                text_row("ctx b b a edge", 1),
                text_row("ctx b a b edge", 2),
            ]),
            expanded: 6,
            examined: 6,
            retained: 0,
        }),
    },
    Case {
        name: CaseName("one_file_owner_resets_dfs_across_closure_empty_and_different_graph"),
        word: Word::Succeeds,
        input: || Graph::SharedOwner,
        expect: Expect::Value(Observed::Owner {
            counts: Cow::Borrowed(&[2, 3, 0, 1]),
            first: Cow::Borrowed(&[
                Cow::Borrowed(&[1, 1, 2, 12, 0]),
                Cow::Borrowed(&[1, 2, 3, 23, 1]),
            ]),
            different: Cow::Borrowed(&[Cow::Borrowed(&[9, 9, 8, 98, 0])]),
            first_adjacency_held: true,
            empty_state: [0, 0, 0],
            working_bytes: 0,
        }),
    },
];

/// A table's number rows of width five, sorted.
fn sorted(table: &ryo_values::domain::rows::types::Table) -> Cow<'static, [Cow<'static, [i64]>]> {
    let mut rows: Vec<Cow<'static, [i64]>> = table
        .cells
        .chunks_exact(5)
        .map(|row| {
            row.iter()
                .map(|cell| {
                    let Cell::Number(Integer(value)) = cell else {
                        panic!("a number cell, not {cell:?}");
                    };
                    *value
                })
                .collect()
        })
        .collect();
    rows.sort_unstable();
    rows.into()
}

/// Walks the graph and observes it.
fn subject(graph: Input) -> Result<Value, Failure> {
    match graph {
        Graph::Text { roots, edges } => text_walk(roots, edges),
        Graph::SharedOwner => shared_owner(),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// Walks the hosted text graph and reads each row back as its texts and ordinal.
fn text_walk(roots: &[[&str; 2]], edges: &[(&str, &str, i64)]) -> Result<Observed, OperatingError> {
    let text = |s: &str| HostCell::Text(s.into());
    let number = |n| HostCell::Number(Integer(n));
    let host_rows = |role: &str, kinds: Vec<HostKind>, rows: Vec<Vec<HostCell>>| HostRelation {
        name: HostName(format!("dfs-text-{role}").into()),
        columns: kinds
            .into_iter()
            .enumerate()
            .map(|(at, kind)| HostColumn {
                name: HostColumnName(format!("c{at}").into()),
                kind,
            })
            .collect(),
        rows: rows.into_iter().map(HostRow).collect(),
    };
    let hosts = [
        host_rows(
            "roots",
            vec![HostKind::Text; 2],
            roots
                .iter()
                .map(|[vertex, context]| vec![text(vertex), text(context)])
                .collect(),
        ),
        host_rows(
            "edges",
            vec![
                HostKind::Text,
                HostKind::Text,
                HostKind::Number,
                HostKind::Text,
                HostKind::Text,
            ],
            edges
                .iter()
                .map(|(from, to, order)| {
                    vec![
                        text(from),
                        text(to),
                        number(*order),
                        text("edge"),
                        text("ctx"),
                    ]
                })
                .collect(),
        ),
    ];
    let mut symbols = Symbols::default();
    let mut observed = ExecutionObservation::default();
    let ran = execute_request(
        hosted_plan(operation(&[1], &[4]), 0, 1),
        &hosts,
        &[],
        &mut symbols,
        b"",
        false,
        &mut observed,
    )?;
    let out = ran.answer?;
    let texts = ryo_operators::domain::table_texts::types::Texts {
        symbols: &symbols,
        source: ryo_values::domain::files::types::SourceView(b""),
        paths: &[],
        arenas: &[],
    };
    let mut rows = Vec::new();
    for row in out.cells.chunks_exact(6) {
        let strings = row[..5]
            .iter()
            .map(|cell| {
                let ryo_operators::domain::table_texts::types::Datum::Text(text) =
                    ryo_operators::domain::table_texts::service::datum(*cell, &texts)
                else {
                    panic!("owned text")
                };
                text.0.to_owned()
            })
            .collect::<Vec<_>>()
            .join(" ");
        let Cell::Number(ordinal) = row[5] else {
            panic!("ordinal")
        };
        rows.push((Cow::Owned(strings), ordinal.0));
    }
    Ok(Observed::Texts {
        count: out.count.0,
        kinds: out.kinds.into(),
        rows: rows.into(),
        expanded: observed.dfs_expanded,
        examined: observed.dfs_examined,
        retained: ran.retained.0,
    })
}

/// Runs the first, closure, empty and different-graph rules on one file owner.
fn shared_owner() -> Result<Observed, OperatingError> {
    let hosts = [
        host(&numbers(1, &[&[1]]), "first-roots"),
        host(
            &numbers(4, &[&[1, 2, 0, 12], &[2, 3, 0, 23]]),
            "first-edges",
        ),
        host(&numbers(1, &[]), "empty-roots"),
        host(&numbers(4, &[]), "empty-edges"),
        host(&numbers(1, &[&[9]]), "different-roots"),
        host(&numbers(4, &[&[9, 8, 0, 98]]), "different-edges"),
    ];
    let first = hosted_plan(operation(&[], &[]), 0, 1);
    let mut closure = first.clone();
    closure.steps[2].operation = Operation::Closure {
        edges: PlanInput::Step(StepId(1)),
        keys: vec![],
        source: column(0),
        destination: column(1),
    };
    let plans = vec![
        first,
        closure,
        hosted_plan(operation(&[], &[]), 2, 3),
        hosted_plan(operation(&[], &[]), 4, 5),
    ];
    let rules = ["first", "closure", "empty", "different"]
        .map(|name| {
            let mut value = rule();
            value.id = RuleId(format!("dfs-lifecycle-{name}").into());
            value
        })
        .into_iter()
        .collect::<Vec<_>>();
    let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            plans,
            bindings: vec![vec![]; 4],
            observations: vec![vec![]; 4],
            schedule: RuleSchedule::Independent,
        },
        &rules,
        &[Digest([0; 16]); 4],
        &hosts,
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )?;
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(Arc::new(
            crate::application::file_dag::fixtures::operators(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("dfs-lifecycle.py".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
    let mut symbols = Symbols::default();
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }; 4],
        services,
        ryo_values::domain::files::types::SourceView(b""),
        ReuseEligibility::Eligible,
        &mut symbols,
        None,
        None,
    )?;
    let read = |_| Err::<Rows<'_>, _>(OperatingError::NotRead);
    let mut outputs = Vec::new();
    let mut states = Vec::new();
    for index in 0..4 {
        let RootOutcome::Complete(table) = file
            .rule(
                RuleIndex(index),
                RuleDemand {
                    emit: EmitDemand::Enabled,
                    observations: ObservationDemand::NotRequired,
                },
                &read,
                &mut symbols,
            )?
            .emit
        else {
            return Err(OperatingError::NotRun);
        };
        let state = file.route_observation(RuleIndex(index), StepId(2))?;
        states.push([
            state.dfs_adjacency_len,
            state.dfs_frames_len,
            state.dfs_visited_len,
        ]);
        outputs.push(table);
    }
    let owner = Rc::clone(&file.owner);
    // The finished file's frontier readiness is not this case's observation.
    let _readiness = file.finish()?;
    Ok(Observed::Owner {
        counts: outputs.iter().map(|table| table.count.0).collect(),
        first: sorted(&outputs[0]),
        different: sorted(&outputs[3]),
        first_adjacency_held: states[0][0] > 0,
        empty_state: states[2],
        working_bytes: owner.derived().working_bytes().0,
    })
}
