//! The `hooks` port's fake runs the port's contract, and fails as it is told to.

use super::super::contract::check;
use super::{HooksFake, Reliability};
use crate::domain::manifest::types::{HookName, ProductRoot};
use crate::outbound::hooks::errors::HooksError;
use crate::outbound::hooks::interface::HooksPort as _;

#[test]
fn succeeds() {
    check(&HooksFake::default(), &ProductRoot("/fake".into()));
}

#[test]
fn refuses() {
    let fake = HooksFake {
        reliability: Reliability::Fails(HooksError::NotARepository {
            path: "/fake".to_owned(),
        }),
        ..HooksFake::default()
    };
    assert_eq!(
        fake.inspect(&ProductRoot("/fake".into()), &HookName("pre-commit".into())),
        Err(HooksError::NotARepository {
            path: "/fake".to_owned(),
        }),
        "the fake fails with the failure it was given"
    );
}
