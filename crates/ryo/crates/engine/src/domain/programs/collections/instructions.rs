//! One collection: the interface every word implements, and each word in order (one item per `ProgramStep` variant). It is
//! not called `steps`: that name is a namespace of its own in this project.

pub mod comparing;
pub mod each_match;
pub mod each_row;
pub mod emit;
pub mod exists;
pub mod fork;
pub mod interface;
pub mod let_value;
pub mod or;
pub mod read;
pub mod yield_row;
