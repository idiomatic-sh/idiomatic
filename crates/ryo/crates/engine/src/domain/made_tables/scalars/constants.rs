//! Words reserved by the packed table representation.
use ryo_language::domain::facts::types::CellWord;
/// The reserved word for a missing cell, including a not-applicable column value.
pub const MISSING_CELL: CellWord = CellWord(u64::MAX);
