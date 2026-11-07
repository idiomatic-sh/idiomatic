//! The test support of the `up` use case: the position its scenes name, and the rounds of checks and `up`s they run.

use super::errors::UpError;
use std::sync::atomic::Ordering;

use super::interface::UpUseCase as _;
use super::types::UpRequest;
use crate::domain::manifest::types::ProductRoot;
use crate::domain::selection::types::{At, Selectors};
use crate::domain::violations::types::{LineNumber, RepoPath};
use crate::inbound::use_cases::check::fixtures::{scenario, up, up_request};

/// The value the tests use.
#[must_use]
pub fn at(line: u32) -> At {
    At {
        path: RepoPath("src/a.rs".into()),
        line: LineNumber(line),
    }
}

/// `up` over a product whose root is missing: the lock and the store are left as they were.
///
/// # Errors
///
/// The error of the `up` that refuses the missing root.
///
/// # Panics
///
/// When the refusal changed the lock or the store.
pub fn manifest_missing() -> Result<String, UpError> {
    let product = scenario();
    let before = product.lock.held();
    let saves = product.store.saves.load(Ordering::Relaxed);
    let result = up(&product).up(UpRequest {
        start: ProductRoot("/missing".into()),
        ..up_request(Selectors::default())
    });
    assert_eq!(product.lock.held(), before, "the lock is as it was");
    assert_eq!(
        product.store.saves.load(Ordering::Relaxed),
        saves,
        "nothing was saved"
    );
    result.map(|_| "unexpected success".to_owned())
}
