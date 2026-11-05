//! Public request data and opaque prepared-program handle.
pub use crate::domain::prepared_request::types::{Compiled, ObservationIdentity};
pub use crate::outbound::derived::interface::DerivedPort;
pub use crate::outbound::operators::interface::OperatorsPort;
use std::sync::Arc;
pub struct PreparedRequest {
    pub compiled: Compiled,
    pub rules: Vec<crate::domain::rules::types::RuleSpec>,
    pub digests: Vec<ryo_values::domain::stored_facts::types::Digest>,
    pub hosted: Vec<ryo_values::domain::hosted::types::HostRelation>,
    pub observations: ObservationIdentity,
}
pub struct PreparedProgram {
    pub(super) inner: Arc<crate::application::file_dag::program::facade::PreparedProgram>,
}

pub use crate::application::file_dag::frontiers::types::{
    BoundInput, BoundMember, PreparedFrontiers, PublicationSlot,
};

pub mod facts {
    pub use crate::outbound::facts::errors::FactsError;
    pub use crate::outbound::facts::interface::{FactsFinish, FactsPort, FactsWorker};
    pub use crate::outbound::facts::types::{
        BucketSource, ColumnsMut, KeptBucket, ResidentStability, WordsToKeep,
    };
}
pub mod digest {
    pub use crate::outbound::digest::interface::DigestPort;
    pub use crate::outbound::digest::types::{DigestBytes, DigestWords};
}
pub mod matching {
    pub use crate::outbound::matching::errors::MatchingError;
    pub use crate::outbound::matching::interface::MatchingPort;
}
