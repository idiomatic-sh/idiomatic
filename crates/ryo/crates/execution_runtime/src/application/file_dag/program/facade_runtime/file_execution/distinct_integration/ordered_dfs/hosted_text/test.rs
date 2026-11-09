//! Hosted text vertices and one file owner reset the traversal across closures and graphs.
use super::super::super::*;
use super::super::fixtures::{
    column, execute_request, expected, host, hosted_plan, numbers, operation,
};
use ryo_execution_domain::domain::operating::fixtures::ExecutionObservation;
use ryo_values::domain::hosted::types::{
    HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow,
};
use ryo_values::domain::row_texts::types::Cell as HostCell;
#[test]
fn text_vertices_and_partition_keys_use_the_actual_hosted_text_owner() {
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
            vec![
                vec![text("a"), text("ctx")],
                vec![text("a"), text("ctx")],
                vec![text("b"), text("ctx")],
            ],
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
            vec![
                vec![text("a"), text("b"), number(0), text("edge"), text("ctx")],
                vec![text("b"), text("c"), number(0), text("edge"), text("ctx")],
                vec![text("b"), text("a"), number(1), text("edge"), text("ctx")],
            ],
        ),
    ];
    let mut symbols = Symbols::default();
    let mut observed = ExecutionObservation::default();
    let out = execute_request(
        hosted_plan(operation(&[1], &[4]), 0, 1),
        &hosts,
        &[],
        &mut symbols,
        b"",
        false,
        &mut observed,
    )
    .unwrap();
    assert_eq!(out.count, RowCount(6));
    assert_eq!(
        out.kinds,
        [
            CellKind::Symbol,
            CellKind::Symbol,
            CellKind::Symbol,
            CellKind::Symbol,
            CellKind::Symbol,
            CellKind::Number
        ]
    );
    let texts = ryo_execution_domain::domain::table_texts::types::Texts {
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
                let ryo_execution_domain::domain::table_texts::types::Datum::Text(text) =
                    ryo_execution_domain::domain::table_texts::service::datum(*cell, &texts)
                else {
                    panic!("owned text")
                };
                text.0.to_owned()
            })
            .collect::<Vec<_>>();
        let Cell::Number(ordinal) = row[5] else {
            panic!("ordinal")
        };
        rows.push((strings, ordinal.0));
    }
    assert_eq!(
        rows,
        vec![
            (
                vec!["ctx", "a", "a", "b", "edge"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                0
            ),
            (
                vec!["ctx", "a", "b", "c", "edge"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                1
            ),
            (
                vec!["ctx", "a", "b", "a", "edge"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                2
            ),
            (
                vec!["ctx", "b", "b", "c", "edge"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                0
            ),
            (
                vec!["ctx", "b", "b", "a", "edge"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                1
            ),
            (
                vec!["ctx", "b", "a", "b", "edge"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                2
            )
        ]
    );
    assert_eq!(observed.dfs_expanded, 6);
    assert_eq!(observed.dfs_examined, 6);
}
#[test]
fn one_file_owner_resets_dfs_across_closure_empty_and_different_graph() {
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
    )
    .unwrap();
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
        &|_| Err(ryo_execution_domain::domain::operating::errors::OperatingError::NotRead))
    .unwrap();
    let read = |_| Err::<Rows<'_>, _>(OperatingError::NotRead);
    let mut outputs = Vec::new();
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
            )
            .unwrap()
            .emit
        else {
            panic!("complete independent rule")
        };
        let state = file.route_observation(RuleIndex(index), StepId(2)).unwrap();
        if index == 0 {
            assert!(state.dfs_adjacency_len > 0);
        }
        if index == 2 {
            assert_eq!(state.dfs_adjacency_len, 0);
            assert_eq!(state.dfs_frames_len, 0);
            assert_eq!(state.dfs_visited_len, 0);
        }
        outputs.push(table);
    }
    expected(&outputs[0], 5, &[&[1, 1, 2, 12, 0], &[1, 2, 3, 23, 1]]);
    assert_eq!(outputs[1].count, RowCount(3));
    assert_eq!(outputs[2].count, RowCount(0));
    expected(&outputs[3], 5, &[&[9, 9, 8, 98, 0]]);
    let owner = Rc::clone(&file.owner);
    file.finish().unwrap();
    assert_eq!(owner.derived().working_bytes(), 0);
}
