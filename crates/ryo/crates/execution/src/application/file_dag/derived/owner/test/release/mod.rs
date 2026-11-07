//! Release drops only the required views of the released population.
use super::*;

#[test]
fn release_drops_only_the_dependent_required_view() {
    let relation_owner = owner(EntryLimit(0));
    let owner = relation_owner.derived();
    let first_publication = relation_owner.computed(source()).unwrap();
    let second_publication = relation_owner.computed(source()).unwrap();
    let first_population = population(&relation_owner, &first_publication);
    let second_population = population(&relation_owner, &second_publication);
    owner.pin(first_population);
    owner.pin(second_population);
    let plan = plan();
    let mut scratch = DerivedScratch::default();
    let first = acquire(
        &relation_owner,
        &plan,
        &first_publication,
        ViewContext::default(),
        &mut scratch,
    );
    let second = acquire(
        &relation_owner,
        &plan,
        &second_publication,
        ViewContext::default(),
        &mut scratch,
    );
    assert!(first.storage_identity() != second.storage_identity());
    let one_view_bytes = ByteCount(owner.working_bytes().0 / 2);
    owner.release(first_population);
    assert_eq!(owner.released_entries(), 1);
    assert_eq!(owner.working_bytes(), one_view_bytes);
    let again = acquire(
        &relation_owner,
        &plan,
        &second_publication,
        ViewContext::default(),
        &mut scratch,
    );
    assert!(second.storage_identity() == again.storage_identity());
    assert_eq!(
        owner.counts(),
        BuildCounts(BuildCount(2), HitCount(1), RefusalCount(2))
    );
    owner.release(second_population);
    assert_eq!(owner.working_bytes(), 0);
    assert_eq!(owner.released_entries(), 2);
}

#[test]
fn two_population_view_survives_either_release_and_repin_without_duplicate_dependencies() {
    let relation_owner = owner(EntryLimit(0));
    let owner = relation_owner.derived();
    let mut ordered_source = source();
    ordered_source.cells = [3, 1, 2]
        .into_iter()
        .map(|value| Value::Number(Integer(value)))
        .collect();
    let publications = [
        relation_owner.computed(ordered_source.clone()).unwrap(),
        relation_owner.computed(ordered_source).unwrap(),
    ];
    let populations = [
        population(&relation_owner, &publications[0]),
        population(&relation_owner, &publications[1]),
    ];
    let plan = pair_plan();
    let mut scratch = DerivedScratch::default();
    let mut cache = indexes::new(&[], &[], ByteLimit(4096)).unwrap();
    let metadata_charge = cache.bytes;
    cache.limit = 0;
    owner.backend.borrow_mut().replace(cache);
    let mut stable_metadata = None;

    for _ in 0..64 {
        owner.pin(populations[0]);
        owner.pin(populations[1]);
        let first = acquire_pair(&relation_owner, &plan, &publications, &mut scratch);
        let weak = Arc::downgrade(first.storage());
        drop(first);

        owner.release(populations[0]);
        assert!(weak.upgrade().is_some());
        assert!(owner.working_bytes() > 0);
        owner.pin(populations[0]);
        let after_first = acquire_pair(&relation_owner, &plan, &publications, &mut scratch);
        assert!(Arc::ptr_eq(after_first.storage(), &weak.upgrade().unwrap()));
        drop(after_first);

        owner.release(populations[1]);
        assert!(weak.upgrade().is_some());
        let after_second = acquire_pair(&relation_owner, &plan, &publications, &mut scratch);
        assert!(Arc::ptr_eq(
            after_second.storage(),
            &weak.upgrade().unwrap()
        ));
        drop(after_second);

        owner.release(populations[0]);
        assert!(weak.upgrade().is_none());
        assert_eq!(owner.working_bytes(), 0);
        assert_eq!(
            owner.backend.borrow().as_ref().unwrap().bytes,
            metadata_charge,
            "zero payload budget never removes existing owner metadata"
        );
        let state = owner.state.borrow();
        assert_eq!(state.entries.len(), 1);
        assert_eq!(state.dependents[&populations[0]].len(), 1);
        assert_eq!(state.dependents[&populations[1]].len(), 1);
        drop(state);
        match stable_metadata {
            None => stable_metadata = Some(owner.metadata_bytes()),
            Some(bytes) => assert_eq!(owner.metadata_bytes(), bytes),
        }
    }
    assert_eq!(owner.counts().0, 64);
}
