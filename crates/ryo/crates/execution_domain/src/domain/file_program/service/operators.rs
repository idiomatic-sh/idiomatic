//! Prepares every operation of a request once, shared by all file workers: each step's operator contract against the
//! schemas of its inputs. The caller binds each prepared contract's derived views; no rows or caches are read here.
pub mod service;
