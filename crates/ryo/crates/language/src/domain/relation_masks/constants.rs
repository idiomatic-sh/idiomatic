//! The names of the `relation_masks` concept: how many numbers a set of relations takes.

/// How many numbers a set of relations takes, a bit for each relation of the catalog.
pub const WORDS: usize = ryo_relations::domain::relations::service::ALL
    .len()
    .div_ceil(64);
