//! The buffers a downstream emit borrows: one owner of the output sink's operator scratch and memo.

pub mod value;
#[cfg(test)]
mod value_test;
