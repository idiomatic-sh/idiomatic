//! Borrowed ordered existence contracts; no engine authority.
/// Strict comparison direction for admitted whole-number columns only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartitionExtremum {
    Maximum,
    Minimum,
}

/// Finite immutable rows already demanded and admitted by the existing engine owner.
/// Repeated reads return identical keys/numbers; equality is exact Value equality.
/// No provider, owner, scheduler or cache is visible to the semantic kernel.
pub trait OrderedExistenceDriver {
    fn len(&self) -> usize;
    fn key_cell(
        &mut self,
        row: usize,
        column: ryo_values::domain::plan_parts::types::ColumnIndex,
    ) -> Result<ryo_relations::domain::relations::types::Value, EqualityJoinError>;
    /// Total non-null whole-number evidence is required before this driver is constructed.
    fn number(
        &mut self,
        row: usize,
        column: ryo_values::domain::plan_parts::types::ColumnIndex,
    ) -> Result<ryo_relations::domain::relations::types::Integer, EqualityJoinError>;
    /// Deliver a dominated source row and an actual same-partition dominating witness.
    /// The engine can reconstruct the original logical Join coordinates for the unchanged Project.
    /// First interruption is sticky, including every subsequent callback.
    fn emit_dominated(&mut self, row: usize, witness: usize) -> Result<(), EqualityJoinError>;
}

/// Per-step scratch supplied and owned by the engine; no completed population is retained.
#[derive(Debug, Default)]
pub struct OrderedExistenceScratch {
    pub groups: std::collections::HashMap<
        Vec<ryo_relations::domain::relations::types::Value>,
        ExtremumWitness,
    >,
    pub key: Vec<ryo_relations::domain::relations::types::Value>,
}

#[derive(Clone, Copy, Debug)]
pub struct ExtremumWitness {
    pub value: ryo_relations::domain::relations::types::Integer,
    pub row: usize,
}

use crate::domain::equality_join::errors::EqualityJoinError;
