//! The `preflight` service: free functions named by verb over the domain's types, with no IO and no ports.

use super::super::types::*;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::InputShape;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::InputNumber;
use ryo_values::domain::plans::types::Operation;


