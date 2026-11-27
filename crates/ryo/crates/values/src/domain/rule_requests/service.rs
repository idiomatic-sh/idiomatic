//! The `rule_requests` service: free functions named by verb over the rules of a run, with no IO and no ports.

use super::errors::RuleRequestsError;
use super::types::RuleSpec;
use crate::domain::query_bindings::types::ParameterBinding;
use crate::domain::query_plans::types::Plan;
use crate::domain::runs::types::RuleId;



