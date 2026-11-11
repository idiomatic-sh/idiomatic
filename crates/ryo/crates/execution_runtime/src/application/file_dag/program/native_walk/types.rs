//! The types of the `native_walk` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use super::super::runtime::types::Submission;
use super::lease::service::PermittedWalk;
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::runtime::service::KernelPermit;
use rustc_hash::FxHashMap;
use ryo_execution_domain::domain::{coverage::types::RuleIndex, shared_walk::types::WalkCounts};
use ryo_execution_domain::domain::{
    operating::errors::OperatingError,
    operator::types::OperatorContext,
    shared_walk::types::{WalkWork, native_batch::value::NativeBatch},
    step_classes::types::ClassPlace,
    table_model::types::Rows,
};
use ryo_execution_store::application::file_dag::derived::population::service::PopulationId;
use ryo_relations::domain::relations::types::Relation;
use std::{
    cell::{Cell, Ref, RefCell},
    collections::BTreeSet,
};

/// Physical delivery must decline before opening any shared predicate work.
#[derive(Clone, Copy)]
pub(crate) enum WalkDelivery {
    Rows,
}

/// Entry-point calls a rule has yet to make.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Deliveries(pub(crate) usize);

/// What retirement decides for one batch.
pub(super) enum Fate {
    Keep,
    Release,
}

/// Batches a file walk still holds.
#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RetainedBatches(pub(crate) usize);
