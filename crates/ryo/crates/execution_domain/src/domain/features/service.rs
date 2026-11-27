//! The `features` service: free functions over a plan and the rows its steps were fed, with no IO and no ports.

use ryo_relations::domain::relations::service::all;

use super::types::{FeatureRows, StepWork};
use crate::domain::operator::types::RowsFed;
use ryo_values::domain::plan_parts::types::Direction;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::row_texts::types::RowTotal;
use ryo_values::domain::run_metadata::types::Feature;
use ryo_values::domain::run_metadata::types::{FeatureSource, RuleRunMetadata, StepMetadata};





