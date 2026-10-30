//! The selector kinds: one item each, every one implementing [`interface::SelectorsItem`]; the service offers a selector to
//! them in order. A new kind is a new item.

pub(crate) mod interface;

pub(crate) mod glob;
pub(crate) mod part;
pub(crate) mod placed_by;
pub(crate) mod regex;
