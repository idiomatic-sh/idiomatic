//! Representation semantics expose no input scheduler or cache owner.
use super::types::{
    BulkAccess, CellRead, PhysicalShape, RelationContract, RepresentationInput, RowRead,
};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_cells::types::RowIndex;
use ryo_language::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;

pub(crate) trait RepresentationsItem: Sync {
    /// Declare immutable metadata. Physical checks are deferred until demanded.
    fn prepare<'file>(
        &self,
        input: RepresentationInput<'file>,
    ) -> Result<RelationContract<'file>, OperatingError>;
    /// Pure structural inspection; the engine decides when to run or reuse it.
    fn inspect(&self, contract: &RelationContract<'_>) -> Result<PhysicalShape, OperatingError>;
    /// Required bulk access keeps existing column/index/tree algorithms available.
    fn bulk<'file>(
        &self,
        input: RepresentationInput<'file>,
    ) -> Result<BulkAccess<'file>, OperatingError>;
    fn cell(
        &self,
        input: RepresentationInput<'_>,
        row: RowIndex,
        column: ColumnIndex,
    ) -> Result<CellRead, OperatingError>;
    fn row<'read>(
        &self,
        input: RepresentationInput<'read>,
        row: RowIndex,
        decode: &'read mut [Value],
    ) -> Result<RowRead<'read>, OperatingError>;
}
