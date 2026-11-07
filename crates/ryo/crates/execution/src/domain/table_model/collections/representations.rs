//! All supported physical row representations share one required contract.
pub(crate) mod compact;
pub(crate) mod computed;
pub(crate) mod interface;
pub(crate) mod publication;
mod registration;
pub(crate) mod selected;
pub(crate) mod service;
pub(crate) mod stored;
pub(crate) mod tree;
pub(crate) mod types;

pub(crate) mod positioned;
