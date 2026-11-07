//! The test data of `linter lint` that fixes observations.

use crate::domain::observation::types::ObservationBindingName;
use crate::domain::observation::types::ObservationColumn;
use crate::domain::observation::types::ObservationColumnName;
use crate::domain::observation::types::ObservationKind;
use crate::domain::observation::types::ObservationName;
use crate::resources::linter::lint::types::ObservationBinding;
use crate::resources::linter::lint::types::ObservationProducer;
use crate::resources::linter::lint::types::RuleId;
use super::shared_calls::shared;

/// The binding of an observation named `incoming` to the output `shared` of `producer`.
pub(crate) fn incoming(columns: Vec<ObservationColumn>, producer: RuleId) -> ObservationBinding {
    ObservationBinding {
        name: ObservationBindingName("incoming".into()),
        columns,
        producers: vec![ObservationProducer::Output {
            rule: producer,
            output: ObservationName("shared".into()),
        }],
    }
}

/// A column of an observation.
pub(crate) fn column(name: &str, kind: ObservationKind) -> ObservationColumn {
    ObservationColumn {
        name: ObservationColumnName(name.into()),
        kind,
    }
}
