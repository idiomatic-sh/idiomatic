//! The scenario `read_failure_demand_is_limited_to_governing_rules` of the cases of this file.

use super::*;

pub(super) fn read_failure_demand_is_limited_to_governing_rules() -> Result<Value, Failure> {
    use crate::domain::reading::service::read_failure_demanded;
    use ryo_execution::inbound::use_cases::metadata::interface::{Needs, RelationRead};
    let wanted = [Relation::FilesystemReadFailure];
    let needs = [
        Needs::default(),
        Needs {
            reads: vec![RelationRead {
                relation: Relation::FilesystemReadFailure,
                kinds: vec![],
            }],
            ..Needs::default()
        },
    ];
    assert_eq!(read_failure_demanded(&wanted, &Mask(vec![1]), &needs, crate::domain::reading::types::RunRuleCount(2)), crate::domain::reading::types::ReadFailureDemand::NotDemanded);
    assert_eq!(read_failure_demanded(&wanted, &Mask(vec![2]), &needs, crate::domain::reading::types::RunRuleCount(2)), crate::domain::reading::types::ReadFailureDemand::Demanded);
    assert_eq!(read_failure_demanded(&wanted, &Mask(vec![1]), &[], crate::domain::reading::types::RunRuleCount(2)), crate::domain::reading::types::ReadFailureDemand::Demanded);
    Ok(1)
}
