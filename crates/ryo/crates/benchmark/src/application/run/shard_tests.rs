//! Shard coordinate refusal contracts.
use super::CaseShard;

#[test]
fn shard_coordinates_are_zero_based_and_bounded() {
    assert!(matches!(
        CaseShard::new(0, 3),
        Ok(CaseShard { index: 0, count: 3 })
    ));
    assert!(matches!(
        CaseShard::new(2, 3),
        Ok(CaseShard { index: 2, count: 3 })
    ));
    let _verified_1 = CaseShard::new(3, 3).expect_err("expected refusal");
    let _verified_2 = CaseShard::new(0, 0).expect_err("expected refusal");
}
