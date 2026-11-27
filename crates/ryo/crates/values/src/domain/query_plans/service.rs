//! The `query_plans` service: free functions named by verb over a whole compiled query, with no IO and no ports: the
//! relations it reads, the order its steps are read in, their digests, and the plan the new engine receives of it, each
//! join that holds a reach a `Closure` step and a plain `Join` again.

use super::types::{NodeBytes, NodeDigest, NodeDigests, Plan, RuleReads};
use crate::domain::plan_parts::types::{
    ColumnIndex, Input, ReachLookup, RepeatedRows, StepId, StepScope};
use crate::domain::plans::service::{inputs_of, named};
use crate::domain::plans::types::{Operation, PlanStep};
use crate::domain::query_bindings::types::RuleBindings;
use ryo_relations::domain::relations::types::Count;
use std::borrow::Cow;








