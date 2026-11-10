//! The execution store crate's derived workload.
//!
//! The derived hex's adapters back to back: the semantics the factory builds, and the port over them, each prepare an equality request,
//! build its payload from the rows and view it, so the faster is chosen by measurement.

#![expect(
    clippy::expect_used,
    reason = "a bench that cannot set up its own input has nothing to measure: the panic is the failure"
)]

use ryo_derived::domain::contract::types::{
    BuildInputs, BuildOutcome, BuildWorkspace, DerivedView,
};
use ryo_derived::domain::request::types::DerivedRequest;
use ryo_execution_store::outbound::derived::factory::{semantics, use_case};
use ryo_execution_store::outbound::derived::interface::DerivedPort;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, Table};

/// How many rows the request is built over.
const ROWS: usize = 10_000;

/// The derived adapters the factory builds.
#[derive(Clone, Copy, Debug)]
enum Variant {
    /// The adapter over the portable implementations of the three derived use cases.
    Semantics,
    /// The port over the given implementation.
    UseCase,
}

impl Variant {
    /// Every adapter, run on the same rows.
    const ALL: [Self; 2] = [Self::Semantics, Self::UseCase];

    /// The adapter this variant is, built through its factory function.
    fn build(self) -> Box<dyn DerivedPort> {
        match self {
            Self::Semantics => Box::new(semantics()),
            Self::UseCase => Box::new(use_case(semantics())),
        }
    }
}

/// The port prepares an equality request on the one column, builds its payload from the rows and views it as its index.
#[divan::bench(args = Variant::ALL)]
fn prepare_build_and_view(bencher: divan::Bencher<'_, '_>, variant: Variant) {
    let adapter = variant.build();
    let rows = Table {
        kinds: vec![CellKind::Number],
        cells: (0..ROWS)
            .map(|cell| Value::Number(Integer(i64::try_from(cell).expect("a small row number"))))
            .collect(),
        count: RowCount(ROWS),
    };
    bencher.bench_local(|| {
        let prepared = adapter.prepare(
            DerivedRequest::Equality {
                columns: vec![ColumnIndex(0)],
            },
            vec![vec![CellKind::Number]],
        );
        prepared.validate().expect("the request is valid");
        let mut room = BuildWorkspace::default();
        let BuildOutcome::Built(payload) = adapter
            .build(&prepared, BuildInputs::Rows(&rows), &mut room, None)
            .expect("the payload is built")
        else {
            panic!("the equality payload is built");
        };
        let DerivedView::Equality(index) = adapter
            .view(&prepared, &payload)
            .expect("the payload is viewed")
        else {
            panic!("the equality payload is viewed as its index");
        };
        index.rows.len()
    });
}
