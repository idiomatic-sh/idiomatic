//! Behavioral declaration-identity controls: boundaries, types and producer readiness stay distinct.
use super::{service::identity, types::*};
use crate::domain::runs::types::RuleId;
use ryo_relations::domain::relations::types::CellKind;

fn declaration(name: &str, rule: &str, output: &str) -> ObservationBinding {
    ObservationBinding {
        name: ObservationBindingName(name.into()),
        columns: vec![ObservationColumn {
            name: ObservationColumnName("file".into()),
            kind: CellKind::File,
        }],
        producers: vec![ObservationProducer::Output {
            rule: RuleId(rule.into()),
            output: ObservationName(output.into()),
        }],
    }
}

#[test]
fn identity_is_typed_and_length_delimited_and_keeps_ordinary_identity_empty() {
    assert!(identity(&[]).0.is_empty());
    let left = declaration("input", "a\tb", "c");
    let right = declaration("input", "a", "b\tc");
    assert_ne!(identity(&[left.clone()]), identity(&[right]));
    let mut typed = left.clone();
    typed.columns[0].kind = CellKind::Number;
    assert_ne!(identity(&[left.clone()]), identity(&[typed]));
    let mut unavailable = left.clone();
    unavailable.producers = vec![ObservationProducer::NoQuery {
        rule: RuleId("a\tb".into()),
    }];
    assert_ne!(identity(&[left.clone()]), identity(&[unavailable]));
    assert_eq!(identity(&[left.clone()]), identity(&[left]));
}

#[test]
fn readiness_is_bound_to_the_semantic_layout_and_never_defaults_to_complete() {
    use super::service::{frontier_state, new_frontiers, set_frontier, validate_frontiers};
    use crate::domain::stored_facts::types::Digest;
    let layout = ObservationReadinessLayout(Digest([3; 16]));
    let count = ObservationReadinessCount(4);
    let mut map = new_frontiers(layout, count);
    set_frontier(
        &mut map,
        ObservationReadinessSlot(0),
        ProducerReadiness::Complete,
    )
    .unwrap();
    set_frontier(
        &mut map,
        ObservationReadinessSlot(1),
        ProducerReadiness::Unknown(ObservationReason("reader refused\tλ\0payload".into())),
    )
    .unwrap();
    set_frontier(
        &mut map,
        ObservationReadinessSlot(2),
        ProducerReadiness::Skipped,
    )
    .unwrap();
    let checked = validate_frontiers(&map, layout, count).unwrap();
    assert_eq!(
        frontier_state(checked, ObservationReadinessSlot(0)).unwrap(),
        ProducerReadiness::Complete
    );
    assert!(matches!(
        frontier_state(checked, ObservationReadinessSlot(1)),
        Ok(ProducerReadiness::Unknown(_))
    ));
    assert!(
        matches!(
            frontier_state(checked, ObservationReadinessSlot(3)),
            Ok(ProducerReadiness::Unknown(_))
        ),
        "missing publication is not an empty complete output"
    );
    assert!(
        validate_frontiers(&map, ObservationReadinessLayout(Digest([4; 16])), count).is_err(),
        "reordered producer layout cannot reinterpret completion bits"
    );
    assert!(validate_frontiers(&map, layout, ObservationReadinessCount(3)).is_err());
    map.version = ObservationReadinessVersion(1);
    assert!(validate_frontiers(&map, layout, count).is_err());
}

#[test]
fn selected_rule_order_and_producer_configuration_define_readiness_layout() {
    use crate::domain::stored_facts::types::Digest;
    let rule = |name: &str, configuration| ObservationRuleLayout {
        rule: RuleId(name.into()),
        frontiers: ObservationFrontierCount(1),
        outputs: ObservationOutputLayout::Persistent {
            configuration: Digest([configuration; 16]),
            outputs: Vec::new(),
        },
    };
    let first = rule("producer-a", 1);
    let second = rule("producer-b", 2);
    let original = super::service::layout_identity(&[first.clone(), second.clone()]);
    assert_ne!(
        original,
        super::service::layout_identity(&[second, first.clone()])
    );
    assert_ne!(original, super::service::layout_identity(&[first.clone()]));
    assert_ne!(
        original,
        super::service::layout_identity(&[first, rule("producer-b", 3)])
    );
}

#[test]
fn malformed_readiness_cannot_publish_completion() {
    use super::service::{new_frontiers, validate_frontiers};
    use crate::domain::{
        records::types::{FrontierRefusal, FrontierRefusalReason},
        stored_facts::types::Digest,
    };
    let layout = ObservationReadinessLayout(Digest([9; 16]));
    let count = ObservationReadinessCount(1);
    let valid = new_frontiers(layout, count);
    let mut changed = valid.clone();
    changed.complete[0] = 2;
    assert!(validate_frontiers(&changed, layout, count).is_err());
    let mut changed = valid.clone();
    changed.complete[0] = 1;
    changed.skipped = vec![1];
    assert!(validate_frontiers(&changed, layout, count).is_err());
    let mut changed = valid.clone();
    changed.complete.clear();
    assert!(validate_frontiers(&changed, layout, count).is_err());
    let mut changed = valid;
    changed.refusals = vec![
        FrontierRefusal {
            slot: ObservationReadinessSlot(0),
            reason: FrontierRefusalReason::NoQuery
        };
        2
    ];
    assert!(validate_frontiers(&changed, layout, count).is_err());
}
