//! The `catalog` port's fake runs the port's contract.

use super::super::contract::check;
use super::CatalogFake;

#[test]
fn succeeds() {
    check(&CatalogFake::default());
}
