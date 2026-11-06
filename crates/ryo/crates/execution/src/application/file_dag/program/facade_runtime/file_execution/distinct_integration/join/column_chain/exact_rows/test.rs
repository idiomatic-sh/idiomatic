//! A selected column cut keeps its exact rows, frontier and capability, and refuses at its boundaries.
use super::super::super::capability::fixtures::Witness;
use super::super::super::*;
use super::super::fixtures::{expected, plan, sources};
use std::sync::atomic::Ordering;
#[test]
fn selected_cut_join_filter_project_keeps_exact_rows_frontier_and_required_capability() {
    let mut symbols = Symbols::default();
    let providers = sources(&mut symbols, 64, true);
    let port = Arc::new(Witness::default());
    let candidate = execute(
        plan(),
        providers,
        &mut symbols,
        b"",
        false,
        port.clone(),
        Faults::default(),
    )
    .unwrap();
    assert!(
        candidate.column_prepared,
        "real compiler must install this exact column chain"
    );
    assert_eq!(candidate.result.unwrap().cells, expected(64));
    assert!(
        port.columns.load(Ordering::Relaxed) > 0,
        "actual column-cell semantic call, not just prepared metadata"
    );
    let frontier = &candidate.frontiers[0];
    assert_eq!(frontier.count, RowCount(64));
    assert_eq!(
        frontier.cells,
        (0..64)
            .flat_map(|at| [
                Cell::Node(node_id(FileId(0), Position(at))),
                Cell::File(FileId(0))
            ])
            .collect::<Vec<_>>()
    );
    // An authored additional reader is the genuine pre-work decline control.
    let mut ordinary = plan();
    ordinary.frontier.push(PlanInput::Step(StepId(1)));
    let providers = sources(&mut symbols, 64, true);
    let ordinary_port = Arc::new(Witness::default());
    let baseline = execute(
        ordinary,
        providers,
        &mut symbols,
        b"",
        false,
        ordinary_port.clone(),
        Faults::default(),
    )
    .unwrap();
    assert!(
        !baseline.column_prepared,
        "observed Join cannot be replaced by a private column continuation"
    );
    assert_eq!(baseline.result.unwrap().cells, expected(64));
    assert_eq!(&baseline.frontiers[0], frontier);
    assert_eq!(ordinary_port.columns.load(Ordering::Relaxed), 0);
    assert!(ordinary_port.calls.load(Ordering::Relaxed) > 0);
    // This extra consumer changes observable publication work. It proves a real
    // decline and output parity, not a matched-workload percentage reduction.

    let providers = sources(&mut symbols, 64, true);
    let refusing = Arc::new(Witness {
        refuse: true,
        only_columns: true,
        ..Witness::default()
    });
    let result = execute(
        plan(),
        providers,
        &mut symbols,
        b"",
        false,
        refusing.clone(),
        Faults::default(),
    )
    .unwrap();
    assert_eq!(
        refusing.columns.load(Ordering::Relaxed),
        1,
        "refusal before first output, without optional fallback"
    );
    assert!(matches!(
        result.result,
        Err(OperatingError::Table(
            ryo_values::domain::rows::errors::TableModelError::NoSuchColumn { index: 60001 }
        ))
    ));
}
#[test]
fn no_hits_missing_provider_and_partial_empty_sources_keep_refusal_and_frontier_boundaries() {
    let mut symbols = Symbols::default();
    let no_hits = execute(
        plan(),
        sources(&mut symbols, 1, false),
        &mut symbols,
        b"",
        false,
        real_port(),
        Faults::default(),
    )
    .unwrap();
    assert!(no_hits.result.unwrap().cells.is_empty());
    assert_eq!(no_hits.frontiers[0].count, RowCount(1));
    let absent = execute(
        plan(),
        sources(&mut symbols, 1, true),
        &mut symbols,
        b"",
        false,
        real_port(),
        Faults {
            left: Some(OperatingError::NotRead),
            ..Faults::default()
        },
    );
    let absent = absent.unwrap();
    assert!(matches!(absent.result, Err(OperatingError::NotRead)));
    assert!(matches!(
        absent.frontier_errors.as_slice(),
        [OperatingError::NotRead]
    ));
    let partial = execute(
        plan(),
        sources(&mut symbols, 0, true),
        &mut symbols,
        b"",
        false,
        real_port(),
        Faults {
            partials: vec![crate::domain::coverage::types::Partial {
                relation: Relation::SemanticChild,
                cause: ryo_values::domain::stored_facts::types::PartialCause("incomplete".into()),
                lacking: crate::domain::coverage::types::Lacking::Any,
                at: None,
            }],
            ..Faults::default()
        },
    )
    .unwrap();
    assert!(partial.result.unwrap().cells.is_empty());
    assert!(partial.frontiers[0].cells.is_empty());
}
