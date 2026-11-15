//! The public accounting over the real audit operation.
use cucumber::then;
use ryo_benchmark::inbound::use_cases::account_defaults::{
    interface::AccountDefaultsUseCase as _, use_case::AccountDefaultsUseCaseImpl,
};
use ryo_benchmark::inbound::use_cases::{
    admit_measurement::fixtures::tool,
    audit::fixtures::{audit_of, audited, deferring},
};

#[then("account_defaults accepts an audit that authored the default")]
fn accepts(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert!(
        AccountDefaultsUseCaseImpl::new()
            .account_defaults(&audited(true), &tool())
            .is_ok()
    );
}

#[then("account_defaults refuses an audit that authored nothing")]
fn refuses(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert!(
        AccountDefaultsUseCaseImpl::new()
            .account_defaults(&audited(false), &tool())
            .is_err()
    );
}

#[then("account_defaults refuses a tool whose deferred default eslint.no-with is also authored")]
fn deferred_and_authored(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    let tool = deferring("eslint.no-with", "typescript_silent");
    let refused = AccountDefaultsUseCaseImpl::new()
        .account_defaults(&audit_of("eslint.no-with", true), &tool)
        .err()
        .map(|refusal| refusal.to_string());
    assert!(
        refused
            .as_deref()
            .is_some_and(|refusal| refusal.contains("invalid default deferral")),
        "{refused:?}"
    );
}
