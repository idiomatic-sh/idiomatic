//! A captured file producer admitted to the real named project publication owner.
pub(super) mod captured;
pub(super) mod scenario;
use super::super::super::super::service::*;
use super::super::super::super::*;
use super::super::super::fixtures::*;
use super::super::fixtures::{FinalConsumer, ProjectionOrder, shared_plan};
use crate::application::file_dag::execution::service::CommonOperatorEngine;
use crate::application::file_dag::frontiers::fixtures::prepared_frontiers;
use crate::application::file_dag::frontiers::types::{BoundInput, BoundMember};
use crate::application::file_dag::program::admission::types::RequiredPartitions;
use crate::application::file_dag::program::facade_runtime::project_execution::{
    service::ProjectRun, service::open_project,
};
use crate::application::file_dag::program::facade_runtime::project_execution::{
    service::evaluate_roots, types::ProjectRoot,
};
use crate::application::file_dag::program::facade_runtime::service::ProjectFrontiers;
use crate::application::file_dag::program::facade_runtime::service::PublicationScope;
use crate::application::file_dag::program::facade_runtime::service::{
    publication_owner, required_partitions,
};
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::rule_outcome::types::ProjectBatch;
use crate::application::file_dag::program::runtime::types::IndexLiveness;
use crate::application::file_dag::program::runtime::types::{
    AttemptCheckpoint, ConsumerReleasePoint,
};
use ryo_execution_domain::domain::coverage::types::Partials;
use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;
use ryo_execution_store::application::file_dag::derived::owner::witness::service::{
    BackingIdentity, BackingLiveness,
};
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, FileId, Integer, Relation, Value as Cell},
};
use ryo_values::domain::hosted::types::{
    HostColumn, HostColumnName, HostKind, HostName, HostRelation,
};
use ryo_values::domain::observations::types::{
    BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationColumn,
    ObservationColumnName, ObservationName, ObservationProducer, PlanObservation,
};
use ryo_values::domain::plan_parts::types::ObservationBindingId;
use ryo_values::domain::plan_parts::types::{HostedNumber, InputNumber};
use ryo_values::domain::records::types::ObservationReadinessLayout;
use ryo_values::domain::rows::errors::RowsError;
use ryo_values::domain::stored_facts::types::{Bucket, KeptWords};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{Input as PlanInput, StepId, StepScope},
    plans::types::{Operation, PlanStep},
    rows::types::{RowCount, Table},
    runs::types::RuleId,
    stored_facts::types::Digest,
};
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub(super) enum Exit {
    Complete,
    SemanticFailure,
    RefusedBinding,
    CancelledDelivery,
}

#[derive(Clone, Copy)]
pub(super) enum BoundSupply {
    Restored,
    Refused,
}
