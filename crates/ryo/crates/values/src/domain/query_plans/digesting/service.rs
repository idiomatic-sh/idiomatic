//! The service of the `digesting` part: one step written as the bytes its digest is taken of, with each input as that
//! input's digest and each parameter as the value it is bound to. Free functions with no IO and no ports.

use crate::domain::plan_parts::types::{Input, Operand, ParameterId, PlanComparison, StepId};
use crate::domain::plans::service::each_input;
use crate::domain::plans::types::{Operation, PlanStep};
use crate::domain::query_bindings::types::{ParameterBinding, RuleBindings};
use crate::domain::query_plans::types::{NodeBytes, NodeDigest, Plan};
use crate::domain::written_rows::types::RowsSource;
use ryo_relations::domain::relations::service::name;
use std::hash::Hash;







