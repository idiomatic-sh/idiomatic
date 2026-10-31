//! Coverage acceptance gates the public benchmark operation.
use cucumber::then;
use ryo_benchmark::domain::suite::service::require_complete;
use ryo_benchmark::outbound::suite::fixtures::driver::audited;
#[then("complete default coverage permits measurement")]
fn complete(world: &mut crate::Behavior) {
    let _ = world;
    let audit = audited(true);
    assert!(require_complete(&audit).is_ok());
}
#[then("missing default coverage prevents measurement")]
fn incomplete(world: &mut crate::Behavior) {
    let _ = world;
    let audit = audited(false);
    let error = require_complete(&audit).expect_err("missing default");
    assert_eq!(
        error.to_string(),
        "default coverage incomplete: 1 missing rules: fixture"
    );
}
