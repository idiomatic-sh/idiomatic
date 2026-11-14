//! The `context` service: free functions named by verb over the domain's types, with no IO and no ports.

use super::super::types::{PrimitiveLease, PrimitiveViews, PrimitiveWorkspace, WorkCounters};
use super::super::views::types::PredicateProgram;
use crate::domain::operating::types::PrimitiveSemantics;
use ryo_language::domain::symbols::types::QuerySymbols;

