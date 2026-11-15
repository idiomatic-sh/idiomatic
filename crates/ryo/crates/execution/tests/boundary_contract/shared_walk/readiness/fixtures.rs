//! The support of the `readiness` cases, which the interpreter's twin cases also read.
use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
use ryo_values::domain::{
    hosted::types::{HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow},
    plan_parts::types::{HostedNumber, Input, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    row_texts::types::Cell,
    rows::types::{RowCount, Table},
};

pub(crate) fn readiness_chain() -> Vec<PlanStep> {
    let mut steps = vec![PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Hosted {
            relation: HostedNumber(0),
        },
    }];
    for (left, right) in [(0, 0), (1, 0), (1, 2)] {
        steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Union {
                left: Input::Step(StepId(left)),
                right: Input::Step(StepId(right)),
            },
        });
    }
    for _ in 0..96 {
        let prior = StepId(u32::try_from(steps.len() - 1).unwrap());
        steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Union {
                left: Input::Step(prior),
                right: Input::Step(prior),
            },
        });
    }
    steps
}

pub(crate) fn readiness_hosted() -> HostRelation {
    HostRelation {
        name: HostName("numbers".into()),
        columns: vec![HostColumn {
            name: HostColumnName("number".into()),
            kind: HostKind::Number,
        }],
        rows: [3, 1, 3, 2]
            .into_iter()
            .map(|n| HostRow(vec![Cell::Number(Integer(n))]))
            .collect(),
    }
}

pub(crate) fn readiness_expected() -> Table {
    Table {
        kinds: vec![CellKind::Number],
        count: RowCount(3),
        cells: [3, 1, 2]
            .into_iter()
            .map(|n| Value::Number(Integer(n)))
            .collect(),
    }
}
