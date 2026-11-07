//! The scenario `unstable_same_shape_resident_populations_remain_distinct` of the cases of this file.

use super::*;

pub(super) fn unstable_same_shape_resident_populations_remain_distinct() -> Result<Value, Failure> {
    use ryo_execution::inbound::use_cases::execution::types::facts::FactsWorker as _;
    use ryo_relations::domain::relations::types::FactColumn;
    let facts = UnstableResidentFacts {
        inner: FactsFake::default().worker().expect("inner"),
        alternatives: [
            vec![
                FactColumn(vec![1]),
                FactColumn(vec![0]),
                FactColumn(vec![0]),
                FactColumn(vec![4]),
            ],
            vec![
                FactColumn(vec![2]),
                FactColumn(vec![0]),
                FactColumn(vec![0]),
                FactColumn(vec![4]),
            ],
        ],
        reads: std::sync::atomic::AtomicUsize::new(0),
    };
    let first = facts
        .rows(Relation::Function)
        .expect("first resident population")
        .columns
        .first()
        .expect("first column")
        .0
        .clone();
    assert_eq!(
        facts.resident_stability(),
        ryo_execution::inbound::use_cases::execution::types::facts::ResidentStability::Unstable
    );
    let second = facts
        .rows(Relation::Function)
        .expect("changed population")
        .columns
        .first()
        .expect("first column")
        .0
        .clone();
    assert_ne!(
        first, second,
        "same relation/schema/count is insufficient without an audited stability promise"
    );
    Ok(1)
}
