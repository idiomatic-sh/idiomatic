//! The complete semantic catalog declares which input populations drive row evaluation.
use super::{service::modes, types::InputShape};
use ryo_values::domain::plans::types::OperationKind;

#[test]
fn every_row_contract_names_exactly_its_required_driving_inputs() {
    for kind in OperationKind::ALL {
        let (streaming, _, _, _, _) = modes(*kind);
        if let InputShape::Row { primary } = streaming.shape {
            let expected = if *kind == OperationKind::Union {
                3
            } else {
                1 << primary.0
            };
            assert_eq!(streaming.row_inputs.0, expected, "{kind:?}");
            assert_ne!(streaming.row_inputs.0, 0);
        } else {
            assert_eq!(streaming.row_inputs.0, 0, "{kind:?}");
        }
    }
}
