//! Owned declarations retain the original ordered-map and duplicate-key semantics.
use super::value::RuleObservationDeclarations;
#[test]
fn collection_and_mutation_preserve_btree_order_and_last_duplicate_value() {
    let a = ryo::linter::RuleId::try_from("A001").unwrap();
    let b = ryo::linter::RuleId::try_from("B001").unwrap();
    let mut map = [(b.clone(), vec![]), (a.clone(), vec![])]
        .into_iter()
        .collect::<RuleObservationDeclarations>();
    assert_eq!(map.keys().collect::<Vec<_>>(), vec![&a, &b]);
    assert!(map.insert(a.clone(), vec![]).is_some());
    assert_eq!(map.len(), 2);
    let owned = map.into_iter().map(|(id, _)| id).collect::<Vec<_>>();
    assert_eq!(owned, vec![a, b]);
}
