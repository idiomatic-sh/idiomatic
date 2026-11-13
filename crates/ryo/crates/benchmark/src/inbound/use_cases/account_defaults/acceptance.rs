//! The public accounting over the real audit operation.
use cucumber::then;
use ryo_benchmark::inbound::use_cases::account_defaults::{
    interface::AccountDefaultsUseCase as _, use_case::AccountDefaultsUseCaseImpl,
};
use ryo_benchmark::inbound::use_cases::{
    admit_measurement::fixtures::tool, audit::fixtures::audited,
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
