//! Coverage acceptance gates the public benchmark operation.
#![expect(
    clippy::expect_used,
    reason = "acceptance assertions must fail the scenario on unexpected success or refusal"
)]
use cucumber::then;
use ryo_benchmark::domain::suite::service::require_complete;
use ryo_benchmark::outbound::suite::fixtures::driver::audited;
/// Assert that complete default coverage permits measurement.
#[then("complete default coverage permits measurement")]
fn complete(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    let audit = audited(true);
    require_complete(&audit).expect("complete audited inventory permits measurement");
}
/// Assert that missing default coverage prevents measurement.
#[then("missing default coverage prevents measurement")]
fn incomplete(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    let audit = audited(false);
    let error = require_complete(&audit).expect_err("missing default");
    assert_eq!(
        error.to_string(),
        "default coverage incomplete: 1 missing rules: fixture",
        "missing authored defaults must remain explicit"
    );
}
