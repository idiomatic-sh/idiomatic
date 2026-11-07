//! The `syntax` port's fake runs the port's contract.

use super::super::contract::check;
use super::SyntaxFake;

#[test]
fn succeeds() {
    check(&SyntaxFake::default());
}
