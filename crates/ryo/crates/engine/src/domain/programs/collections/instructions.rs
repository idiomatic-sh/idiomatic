//! One collection: the interface every word implements, and each word in order (one item per `ProgramStep` variant). It is
//! not called `steps`: that name is a namespace of its own in this project. Until the loop is the scan's body the
//! collection is compiled with the crate's tests.

#[cfg(test)]
pub mod each_row;
#[cfg(test)]
pub mod emit;
#[cfg(test)]
pub mod exists;
#[cfg(test)]
pub mod fork;
#[cfg(test)]
pub mod interface;
#[cfg(test)]
pub mod let_value;
#[cfg(test)]
pub mod or;
#[cfg(test)]
pub mod read;
#[cfg(test)]
pub mod test;
#[cfg(test)]
pub mod yield_row;
