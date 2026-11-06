//! One selection rule keeps one route per region; the other candidates are dropped, never queued.
use super::{Candidates, Route, RouteKind};
use crate::domain::executable_program::types::{OrderedExistence, RegionId};
use crate::domain::lowering::types::{LoweredShape, RowWidth};
use crate::domain::ordered_existence::types::{Extremum, NumberRequirement};
use ryo_values::domain::plan_parts::types::{ColumnIndex, StepId};
use std::sync::Arc;

fn ordered() -> OrderedExistence {
    OrderedExistence {
        source: RegionId(0),
        number: NumberRequirement {
            source: StepId(0),
            column: ColumnIndex(0),
            width: 1,
            origins: Vec::new(),
        },
        partition: Box::new([]),
        order: ColumnIndex(0),
        extremum: Extremum::Maximum,
        output: Box::new([]),
    }
}
fn native() -> Arc<LoweredShape> {
    Arc::new(LoweredShape {
        width: RowWidth(1),
        columns: Vec::new(),
    })
}

#[test]
fn a_region_no_family_offered_is_generic() {
    let route = Candidates::default().route();
    assert_eq!(route.kind(), RouteKind::Generic);
    assert!(route.ordered_existence().is_none());
    assert!(route.column().is_none());
    assert!(route.selection().is_none());
    assert!(route.native_positions().is_none());
}

#[test]
fn native_positions_alone_is_the_native_route() {
    let route = Candidates {
        native: Some(native()),
        ..Candidates::default()
    }
    .route();
    assert_eq!(route.kind(), RouteKind::NativePositions);
    assert!(route.native_positions().is_some());
}

#[test]
fn ordered_existence_wins_over_native_positions_and_nothing_else_stays_reachable() {
    let route = Candidates {
        ordered: Some(ordered()),
        native: Some(native()),
        ..Candidates::default()
    }
    .route();
    assert_eq!(route.kind(), RouteKind::OrderedExistence);
    assert!(route.ordered_existence().is_some());
    assert!(route.native_positions().is_none());
    assert!(route.column().is_none());
    assert!(route.selection().is_none());
}

#[test]
fn the_default_route_is_generic() {
    assert_eq!(Route::default().kind(), RouteKind::Generic);
}
