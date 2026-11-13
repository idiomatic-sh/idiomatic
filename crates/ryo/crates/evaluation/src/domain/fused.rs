//! The rules of a request fused into one program: every step of every rule once, however many rules repeat it, and every root
//! select of the parser's tree served by one walk.

pub mod errors;
pub mod service;
pub mod types;
