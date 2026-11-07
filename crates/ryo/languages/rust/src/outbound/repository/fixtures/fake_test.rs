//! The `repository` port's fake runs the port's contract.

use super::super::contract::{FILES, check};
use super::RepositoryFake;

#[test]
fn succeeds() {
    let fake = RepositoryFake {
        files: FILES
            .iter()
            .map(|(path, text)| ((*path).to_owned(), (*text).to_owned()))
            .collect(),
    };
    check(&fake);
}
