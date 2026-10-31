//! Coverage acceptance gates the public benchmark operation.
use cucumber::then;
use ryo_benchmark::domain::suite::types::Audit;
use ryo_benchmark::inbound::use_cases::audit::service::require_complete;
#[then("complete default coverage permits measurement")]
fn complete(world: &mut crate::Behavior) {
    let _ = world;
    let audit = Audit {
        implemented: vec!["fixture".into()],
        missing: Vec::new(),
    };
    assert!(require_complete(&audit).is_ok());
}
#[then("missing default coverage prevents measurement")]
fn incomplete(world: &mut crate::Behavior) {
    let _ = world;
    let audit = Audit {
        implemented: Vec::new(),
        missing: vec!["fixture".into()],
    };
    let error = require_complete(&audit).expect_err("missing default");
    assert_eq!(
        error.to_string(),
        "default coverage incomplete: 1 missing rules: fixture"
    );
}
