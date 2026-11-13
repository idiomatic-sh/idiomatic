//! The `profile_tools` port's fake runs the port's contract.
use super::super::contract::check;
use super::ProfileToolsFake;

#[test]
fn succeeds() {
    check(&ProfileToolsFake::default(), std::path::Path::new("root"));
}
