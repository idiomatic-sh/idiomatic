//! The `physical` service: free functions named by verb over the domain's types, with no IO and no ports.

use super::super::types::RegexIdentity;
use super::types::{PhysicalAuthorization, PhysicalBinding, PhysicalRoute};
use crate::domain::operating::errors::OperatingError;
use ryo_operators::domain::evaluation::types::Lent;
use ryo_operators::domain::ordered_existence::types::PartitionExtremum;
use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_operators::types::OperationKind;


