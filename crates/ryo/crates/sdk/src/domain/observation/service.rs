//! Private conversion of SDK-owned observation declarations into planner inputs.
use super::types::{BoundObservationInput, ObservationColumn, ObservationKind, ObservationScope};

/// Preserve every declared semantic kind at the SDK boundary.
pub(crate) fn columns(
    columns: Vec<ObservationColumn>,
) -> Vec<ryo_values::domain::observations::types::ObservationColumn> {
    use ryo_relations::domain::relations::types::CellKind;
    columns
        .into_iter()
        .map(
            |column| ryo_values::domain::observations::types::ObservationColumn {
                name: ryo_values::domain::observations::types::ObservationColumnName(column.name.0),
                kind: match column.kind {
                    ObservationKind::Node => CellKind::Node,
                    ObservationKind::Symbol => CellKind::Symbol,
                    ObservationKind::File => CellKind::File,
                    ObservationKind::Span => CellKind::Span,
                    ObservationKind::Number => CellKind::Number,
                    ObservationKind::Flag => CellKind::Flag,
                },
            },
        )
        .collect()
}

/// Carry the same explicit context through validation and compilation.
pub(crate) fn declarations(
    inputs: Vec<BoundObservationInput>,
) -> Vec<ryo_values::domain::observations::types::BoundObservationInput> {
    inputs
        .into_iter()
        .map(
            |input| ryo_values::domain::observations::types::BoundObservationInput {
                name: ryo_values::domain::observations::types::ObservationBindingName(input.name.0),
                columns: columns(input.columns),
                scope: match input.scope {
                    ObservationScope::PerFile => {
                        ryo_values::domain::plan_parts::types::StepScope::PerFile
                    }
                    ObservationScope::CrossFile => {
                        ryo_values::domain::plan_parts::types::StepScope::CrossFile
                    }
                },
            },
        )
        .collect()
}
