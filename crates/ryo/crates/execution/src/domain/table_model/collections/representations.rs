//! All supported physical row representations share one required contract.
pub(crate) mod compact;
pub(crate) mod computed;
pub(in crate::domain::table_model) mod interface;
pub(crate) mod selected;
pub(crate) mod stored;
pub(crate) mod tree;
pub(crate) mod types;

pub(crate) mod positioned;
