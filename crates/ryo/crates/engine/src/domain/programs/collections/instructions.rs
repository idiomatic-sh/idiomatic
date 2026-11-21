//! One collection: the interface every word implements, and each word in order (one item per `ProgramStep` variant). It is
//! not called `steps`: that name is a namespace of its own in this project.

pub mod axis;
pub mod call;
pub mod comparing;
pub mod each_kind;
pub mod each_match;
pub mod each_row;
pub mod emit;
pub mod exists;
pub mod fork;
pub mod interface;
pub mod latest;
pub mod let_value;
pub mod nearest;
pub mod or;
pub mod reached;
pub mod read;
pub mod yield_row;
