//! The `repository` port's fake runs the port's contract.

use super::super::contract::check;
use super::RepositoryFake;

#[test]
fn succeeds() {
    check(
        &RepositoryFake::of(&[]),
        &tastemaker_values::domain::files::types::MachinePath("/fake".into()),
    );
}
