//! The fake executes the same request and response contract as the JSON transport.
use super::super::{
    contract::check,
    data::{report, request},
};
use super::WorkerFake;
#[test]
fn succeeds() {
    let fake = WorkerFake::new(request());
    check(&fake);
    assert_eq!(fake.receive_calls(), 1);
    assert_eq!(fake.respond_calls(), 1);
    assert_eq!(fake.reports(), [report()]);
}
