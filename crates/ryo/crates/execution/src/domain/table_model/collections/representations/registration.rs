//! The physical catalog selects one opaque classified value.
use super::types::{
    RepresentationKind, registered_representation::value::RegisteredRepresentation,
};
pub(crate) fn registered(kind: RepresentationKind) -> RegisteredRepresentation {
    RegisteredRepresentation::new(kind)
}
